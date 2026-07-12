#!/usr/bin/env python3
"""CN-0 — operand readout: can a linear/Fourier/MLP probe decode the two
operands of an in-flight addition from Gemma 3 4B's residual stream, across
varied surface forms, at layers L12-L28?

Vendors the mlx_lm loading + per-layer last-position residual capture pattern
from chris-experiments/arithmetic_mechanism/a1_trace.py (`run_block`/`trace`)
and the Fourier/helix design-matrix scaffolding from a2c_helix_rotation.py
(`design_row`, periods 2/5/10/100) — reused here in the READ direction
(residual -> operand value) rather than a2c's WRITE direction (value ->
injection vector).

Gate (docs/16 / experiments/cell-native-architectures.md CN-0): some
(probe, layer) achieves >=95% exact-pair recovery on held-out surface forms.
Kill: no family exceeds 80% anywhere in the band.

Wave-1 rerun (5x data, tuned lambda, L12-28) left every held-out family below
both lines (max 57.5%, mixed) but still visibly climbing with N, and every
hyperparameter (ridge lambda, MLP architecture) was a single fixed choice, not
searched. This version closes that gap with a real, honest sweep: for each
held-out-family rotation, hyperparameters are selected on an INNER validation
split carved out of the pooled training families (never the true held-out
family itself, which would leak the test set into hyperparameter selection
and inflate the reported number) before the final held-out evaluation.

Run: python3 cn0_operand_readout.py
"""
from __future__ import annotations

import json
import os
import time
from pathlib import Path

import mlx.core as mx
import mlx.nn as nn
import mlx.optimizers as optim
import mlx_lm
import numpy as np

MODEL_ID = "google/gemma-3-4b-it"
OUT = Path(__file__).resolve().parent / os.environ.get("CN0_OUT", "cn0_operand_readout_results.json")

_layers_lo, _layers_hi = (int(x) for x in os.environ.get("CN0_LAYERS", "12,29").split(","))
LAYERS = list(range(_layers_lo, _layers_hi))  # L12..L28 by default
N_PER_FAMILY = int(os.environ.get("CN0_N_PER_FAMILY", 200))
VAL_RANGE = (1, 99)  # two-operand arithmetic, a,b in [1,99]
SEED = 7
PERIODS = [2, 5, 10, 100]

# ---- hyperparameter grids (the "real sweep" this version adds) ----
RIDGE_LAMBDA_GRID = [float(x) for x in os.environ.get(
    "CN0_RIDGE_GRID", "0.01,0.03,0.1,0.3,1.0,3.0,10.0,30.0,100.0").split(",")]
MLP_HIDDEN_GRID = [int(x) for x in os.environ.get("CN0_MLP_HIDDEN_GRID", "32,64,128,256").split(",")]
MLP_LR_GRID = [float(x) for x in os.environ.get("CN0_MLP_LR_GRID", "0.001,0.005,0.01").split(",")]
MLP_EPOCHS = int(os.environ.get("CN0_MLP_EPOCHS", 400))
INNER_VAL_FRAC = 0.2

OPS = os.environ.get("CN0_OPS", "add").split(",")  # add | sub | mul, comma-separated

ONES = ["", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
        "ten", "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen",
        "seventeen", "eighteen", "nineteen"]
TENS = ["", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"]


def num2word(n: int) -> str:
    if n < 20:
        return ONES[n]
    t, o = divmod(n, 10)
    return TENS[t] + ("-" + ONES[o] if o else "")


# ---- vendored from a1_trace.py / a2c_helix_rotation.py (residual capture) ----

def build(model, tokenizer):
    inner = model.language_model.model
    layers = model.layers
    embed = inner.embed_tokens
    hidden = int(embed.weight.shape[1])
    nL = len(layers)

    def run_block(h, i, mask):
        b = layers[i]
        r = h
        a = b.self_attn(b.input_layernorm(h), mask=mask)
        if isinstance(a, tuple):
            a = a[0]
        h = r + b.post_attention_layernorm(a)
        return h + b.post_feedforward_layernorm(b.mlp(b.pre_feedforward_layernorm(h)))

    max_want = max(LAYERS)
    feature_mode = os.environ.get("CN0_FEATURE", "tap")  # tap | mean_pooled

    def trace_last(text, want):
        ids = mx.array([tokenizer.encode(text)])
        mask = nn.MultiHeadAttention.create_additive_causal_mask(ids.shape[1]).astype(mx.bfloat16)
        h = embed(ids) * (hidden ** 0.5)
        out = {}
        for i in range(nL):
            h = run_block(h, i, mask)
            if i in want:
                if feature_mode == "mean_pooled":
                    out[i] = np.array(h[0].astype(mx.float32)).mean(axis=0)
                else:
                    out[i] = np.array(h[:, -1, :][0].astype(mx.float32))
            if i >= max_want:
                break
        return out

    return nL, hidden, trace_last


# ---- prompt battery: four surface-form families over the same (a, b) pairs ----
# CN0_OPS selects which operation(s) populate the battery; each op gets its own
# independent (a,b) draw and its own results block, evaluated identically.

def op_text(op, fam, a, b):
    if op == "add":
        verb_word, verb_narr, sym = "plus", ("found", "now has"), "+"
    elif op == "sub":
        verb_word, verb_narr, sym = "minus", ("gave away", "now has"), "-"
    else:  # mul
        verb_word, verb_narr, sym = "times", ("bought groups of", "now has in total"), "*"

    if fam == "digit":
        return f"{a} {sym} {b} = "
    if fam == "word":
        return f"{num2word(a)} {verb_word} {num2word(b)} equals "
    if fam == "mixed":
        return f"{a} {verb_word} {num2word(b)} is "
    # narrative
    if op == "add":
        return f"Sam had {a} marbles and found {b} more. Sam now has "
    if op == "sub":
        return f"Sam had {a} marbles and gave away {b}. Sam now has "
    return f"Sam bought {b} groups of {a} marbles each. Sam now has in total "


def op_value(op, a, b):
    if op == "add":
        return a + b
    if op == "sub":
        return a - b
    return a * b


def make_battery(rng, op):
    families = {}
    for fam in ("digit", "word", "mixed", "narrative"):
        lo, hi = VAL_RANGE
        if op == "sub":
            # keep the narrative's "gave away" non-negative and in-range
            pairs = []
            for _ in range(N_PER_FAMILY):
                a = int(rng.integers(lo + 1, hi, endpoint=True))
                b = int(rng.integers(lo, a, endpoint=True))
                pairs.append((a, b))
        elif op == "mul":
            # keep products decodable in a modest range: a,b in [2,12]
            pairs = [(int(rng.integers(2, 12, endpoint=True)),
                      int(rng.integers(2, 12, endpoint=True))) for _ in range(N_PER_FAMILY)]
        else:
            pairs = [(int(rng.integers(*VAL_RANGE, endpoint=True)),
                      int(rng.integers(*VAL_RANGE, endpoint=True))) for _ in range(N_PER_FAMILY)]
        rows = [{"a": a, "b": b, "text": op_text(op, fam, a, b)} for a, b in pairs]
        families[fam] = rows
    return families


# ---- probes (all take an explicit hyperparameter, no module-level globals) ----

def ridge_fit(X, Y, lam):
    d = X.shape[1]
    M = np.linalg.solve(X.T @ X + lam * np.eye(d), X.T @ Y)
    return M


def linear_probe_eval(Xtr, Atr, Btr, Xte, Ate, Bte, lam):
    Xtr1 = np.hstack([Xtr, np.ones((len(Xtr), 1))])
    Xte1 = np.hstack([Xte, np.ones((len(Xte), 1))])
    M = ridge_fit(Xtr1, np.stack([Atr, Btr], axis=1).astype(np.float64), lam)
    pred = Xte1 @ M
    a_hat = np.round(pred[:, 0]).astype(int)
    b_hat = np.round(pred[:, 1]).astype(int)
    return a_hat, b_hat


def design_row(x, mean, std):
    row = [1.0, (x - mean) / (std + 1e-9)]
    for T in PERIODS:
        row += [np.cos(2 * np.pi * x / T), np.sin(2 * np.pi * x / T)]
    return np.array(row)


def fourier_probe_eval(Xtr, Atr, Btr, Xte, Ate, Bte, lo, hi, lam):
    Xtr1 = np.hstack([Xtr, np.ones((len(Xtr), 1))])
    Xte1 = np.hstack([Xte, np.ones((len(Xte), 1))])
    mean = float((lo + hi) / 2)
    std = float(np.std(np.arange(lo, hi + 1)))
    codebook = np.stack([design_row(x, mean, std) for x in range(lo, hi + 1)])  # (V, 10)

    Ba = np.stack([design_row(x, mean, std) for x in Atr])
    Bb = np.stack([design_row(x, mean, std) for x in Btr])
    Ma = ridge_fit(Xtr1, Ba, lam)
    Mb = ridge_fit(Xtr1, Bb, lam)
    pred_a = Xte1 @ Ma
    pred_b = Xte1 @ Mb
    a_hat = np.array([lo + int(np.argmin(np.linalg.norm(codebook - row, axis=1))) for row in pred_a])
    b_hat = np.array([lo + int(np.argmin(np.linalg.norm(codebook - row, axis=1))) for row in pred_b])
    return a_hat, b_hat


def mlp_probe_eval(Xtr, Atr, Btr, Xte, Ate, Bte, hidden, lr, epochs=MLP_EPOCHS):
    d = Xtr.shape[1]
    mu, sd = Xtr.mean(0, keepdims=True), Xtr.std(0, keepdims=True) + 1e-6
    Xtr_n = mx.array(((Xtr - mu) / sd).astype(np.float32))
    Xte_n = mx.array(((Xte - mu) / sd).astype(np.float32))
    Ytr = mx.array(np.stack([Atr, Btr], axis=1).astype(np.float32))

    mlp = nn.Sequential(nn.Linear(d, hidden), nn.ReLU(), nn.Linear(hidden, 2))
    opt = optim.Adam(learning_rate=lr)

    def loss_fn(model, x, y):
        return nn.losses.mse_loss(model(x), y)

    lv_and_grad = nn.value_and_grad(mlp, loss_fn)
    for _ in range(epochs):
        loss, grads = lv_and_grad(mlp, Xtr_n, Ytr)
        opt.update(mlp, grads)
        mx.eval(mlp.parameters(), opt.state)

    pred = np.array(mlp(Xte_n))
    a_hat = np.round(pred[:, 0]).astype(int)
    b_hat = np.round(pred[:, 1]).astype(int)
    return a_hat, b_hat


def exact_pair_rate(a_hat, b_hat, Ate, Bte):
    hits = (a_hat == Ate) & (b_hat == Bte)
    return float(hits.mean())


# ---- nested-validation hyperparameter selection (never touches the true
# held-out family — only the pooled *training* families are split further) ----

def inner_split(rng, n, val_frac=INNER_VAL_FRAC):
    idx = rng.permutation(n)
    n_val = max(1, int(n * val_frac))
    return idx[n_val:], idx[:n_val]


def tune_ridge(probe_fn, Xtr, Atr, Btr, rng, extra_args=()):
    tr_idx, val_idx = inner_split(rng, len(Xtr))
    best_lam, best_score = RIDGE_LAMBDA_GRID[0], -1.0
    for lam in RIDGE_LAMBDA_GRID:
        a_hat, b_hat = probe_fn(Xtr[tr_idx], Atr[tr_idx], Btr[tr_idx],
                                 Xtr[val_idx], Atr[val_idx], Btr[val_idx], *extra_args, lam)
        score = exact_pair_rate(a_hat, b_hat, Atr[val_idx], Btr[val_idx])
        if score > best_score:
            best_score, best_lam = score, lam
    return best_lam


def tune_mlp(Xtr, Atr, Btr, rng):
    tr_idx, val_idx = inner_split(rng, len(Xtr))
    best_hp, best_score = (MLP_HIDDEN_GRID[0], MLP_LR_GRID[0]), -1.0
    for hidden in MLP_HIDDEN_GRID:
        for lr in MLP_LR_GRID:
            a_hat, b_hat = mlp_probe_eval(Xtr[tr_idx], Atr[tr_idx], Btr[tr_idx],
                                           Xtr[val_idx], Atr[val_idx], Btr[val_idx], hidden, lr)
            score = exact_pair_rate(a_hat, b_hat, Atr[val_idx], Btr[val_idx])
            if score > best_score:
                best_score, best_hp = score, (hidden, lr)
    return best_hp


def main():
    t0 = time.time()
    rng = np.random.default_rng(SEED)
    print(f"loading {MODEL_ID} ...", flush=True)
    model, tokenizer = mlx_lm.load(MODEL_ID)
    nL, hidden_dim, trace_last = build(model, tokenizer)
    print(f"loaded; {nL} layers, hidden={hidden_dim} ({time.time()-t0:.1f}s)", flush=True)

    all_results = {"model": MODEL_ID, "n_layers": nL, "layers": LAYERS,
                   "n_per_family": N_PER_FAMILY, "val_range": list(VAL_RANGE), "ops": {}}

    for op in OPS:
        print(f"\n### op={op} ###", flush=True)
        battery = make_battery(rng, op)
        fam_names = list(battery.keys())
        n_total = sum(len(v) for v in battery.values())
        print(f"battery: {n_total} prompts across {fam_names}", flush=True)

        want = set(LAYERS)
        captured = {fam: [] for fam in fam_names}
        n_done = 0
        for fam, rows in battery.items():
            for row in rows:
                resid = trace_last(row["text"], want)
                captured[fam].append({"a": row["a"], "b": row["b"], "resid": resid})
                n_done += 1
                if n_done % 40 == 0:
                    print(f"  captured {n_done}/{n_total} ({time.time()-t0:.1f}s)", flush=True)
        print(f"capture done ({time.time()-t0:.1f}s)", flush=True)

        results = {"held_out_family": {}, "random_split": {}}

        for held in fam_names:
            train_fams = [f for f in fam_names if f != held]
            results["held_out_family"][held] = {}
            for L in LAYERS:
                Xtr = np.stack([r["resid"][L] for f in train_fams for r in captured[f]])
                Atr = np.array([r["a"] for f in train_fams for r in captured[f]])
                Btr = np.array([r["b"] for f in train_fams for r in captured[f]])
                Xte = np.stack([r["resid"][L] for r in captured[held]])
                Ate = np.array([r["a"] for r in captured[held]])
                Bte = np.array([r["b"] for r in captured[held]])

                row = {}
                lam_lin = tune_ridge(linear_probe_eval, Xtr, Atr, Btr, rng)
                a_hat, b_hat = linear_probe_eval(Xtr, Atr, Btr, Xte, Ate, Bte, lam_lin)
                row["linear"] = round(exact_pair_rate(a_hat, b_hat, Ate, Bte), 3)
                row["linear_lambda"] = lam_lin

                lam_four = tune_ridge(fourier_probe_eval, Xtr, Atr, Btr, rng, extra_args=VAL_RANGE)
                a_hat, b_hat = fourier_probe_eval(Xtr, Atr, Btr, Xte, Ate, Bte, *VAL_RANGE, lam_four)
                row["fourier"] = round(exact_pair_rate(a_hat, b_hat, Ate, Bte), 3)
                row["fourier_lambda"] = lam_four

                hidden_hp, lr_hp = tune_mlp(Xtr, Atr, Btr, rng)
                a_hat, b_hat = mlp_probe_eval(Xtr, Atr, Btr, Xte, Ate, Bte, hidden_hp, lr_hp)
                row["mlp"] = round(exact_pair_rate(a_hat, b_hat, Ate, Bte), 3)
                row["mlp_hidden"], row["mlp_lr"] = hidden_hp, lr_hp

                results["held_out_family"][held][L] = row
            best = max(max(v[p] for p in ("linear", "fourier", "mlp")) for v in results["held_out_family"][held].values())
            print(f"  held-out={held:10s} best-across-layers-and-probes={best:.3f}  ({time.time()-t0:.1f}s)", flush=True)

        all_rows = [r for f in fam_names for r in captured[f]]
        idx = rng.permutation(len(all_rows))
        n_test = max(1, len(all_rows) // 5)
        te_idx, tr_idx = idx[:n_test], idx[n_test:]
        for L in LAYERS:
            X = np.stack([r["resid"][L] for r in all_rows])
            A = np.array([r["a"] for r in all_rows])
            B = np.array([r["b"] for r in all_rows])
            row = {}
            lam_lin = tune_ridge(linear_probe_eval, X[tr_idx], A[tr_idx], B[tr_idx], rng)
            a_hat, b_hat = linear_probe_eval(X[tr_idx], A[tr_idx], B[tr_idx], X[te_idx], A[te_idx], B[te_idx], lam_lin)
            row["linear"] = round(exact_pair_rate(a_hat, b_hat, A[te_idx], B[te_idx]), 3)
            lam_four = tune_ridge(fourier_probe_eval, X[tr_idx], A[tr_idx], B[tr_idx], rng, extra_args=VAL_RANGE)
            a_hat, b_hat = fourier_probe_eval(X[tr_idx], A[tr_idx], B[tr_idx], X[te_idx], A[te_idx], B[te_idx], *VAL_RANGE, lam_four)
            row["fourier"] = round(exact_pair_rate(a_hat, b_hat, A[te_idx], B[te_idx]), 3)
            hidden_hp, lr_hp = tune_mlp(X[tr_idx], A[tr_idx], B[tr_idx], rng)
            a_hat, b_hat = mlp_probe_eval(X[tr_idx], A[tr_idx], B[tr_idx], X[te_idx], A[te_idx], B[te_idx], hidden_hp, lr_hp)
            row["mlp"] = round(exact_pair_rate(a_hat, b_hat, A[te_idx], B[te_idx]), 3)
            results["random_split"][L] = row

        all_results["ops"][op] = results

        print(f"\n=== op={op} summary: best (probe, layer) per held-out family ===")
        for held in fam_names:
            table = results["held_out_family"][held]
            best_layer, best_probe, best_val = None, None, -1
            for L, row in table.items():
                for probe in ("linear", "fourier", "mlp"):
                    if row[probe] > best_val:
                        best_val, best_layer, best_probe = row[probe], L, probe
            print(f"  held-out={held:10s} best={best_probe}@L{best_layer} exact-pair={best_val:.3f}")

    OUT.write_text(json.dumps(all_results, indent=2))
    print(f"\nwrote {OUT} ({time.time()-t0:.1f}s)", flush=True)


if __name__ == "__main__":
    main()
