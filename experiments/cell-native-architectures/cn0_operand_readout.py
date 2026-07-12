#!/usr/bin/env python3
"""CN-0 slice-0 — operand readout: can a linear/Fourier/MLP probe decode the two
operands of an in-flight addition from Gemma 3 4B's residual stream, across
varied surface forms, at layers L12-L22?

Vendors the mlx_lm loading + per-layer last-position residual capture pattern
from chris-experiments/arithmetic_mechanism/a1_trace.py (`run_block`/`trace`)
and the Fourier/helix design-matrix scaffolding from a2c_helix_rotation.py
(`design_row`, periods 2/5/10/100) — reused here in the READ direction
(residual -> operand value) rather than a2c's WRITE direction (value ->
injection vector).

Gate (docs/16 / experiments/cell-native-architectures.md CN-0): some
(probe, layer) achieves >=95% exact-pair recovery on held-out surface forms.
Kill: no family exceeds 80% anywhere in the band.

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
OUT = Path(__file__).resolve().parent / "cn0_operand_readout_results.json"

_layers_lo, _layers_hi = (int(x) for x in os.environ.get("CN0_LAYERS", "12,23").split(","))
LAYERS = list(range(_layers_lo, _layers_hi))  # L12..L22 by default; slice-1 sweeps past L22
N_PER_FAMILY = int(os.environ.get("CN0_N_PER_FAMILY", 40))
VAL_RANGE = (1, 99)  # two-operand addition, a,b in [1,99]
SEED = 7
RIDGE_LAMBDA = float(os.environ.get("CN0_RIDGE_LAMBDA", 1.0))
PERIODS = [2, 5, 10, 100]
MLP_HIDDEN = int(os.environ.get("CN0_MLP_HIDDEN", 64))
MLP_EPOCHS = int(os.environ.get("CN0_MLP_EPOCHS", 300))
MLP_LR = 5e-3

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

    def trace_last(text, want):
        ids = mx.array([tokenizer.encode(text)])
        mask = nn.MultiHeadAttention.create_additive_causal_mask(ids.shape[1]).astype(mx.bfloat16)
        h = embed(ids) * (hidden ** 0.5)
        out = {}
        for i in range(nL):
            h = run_block(h, i, mask)
            if i in want:
                out[i] = np.array(h[:, -1, :][0].astype(mx.float32))
            if i >= max_want:
                break
        return out

    return nL, hidden, trace_last


# ---- prompt battery: four surface-form families over the same (a, b) pairs ----

def make_battery(rng):
    families = {}
    for fam in ("digit", "word", "mixed", "narrative"):
        pairs = [(int(rng.integers(*VAL_RANGE, endpoint=True)),
                  int(rng.integers(*VAL_RANGE, endpoint=True)))
                 for _ in range(N_PER_FAMILY)]
        rows = []
        for a, b in pairs:
            if fam == "digit":
                text = f"{a} + {b} = "
            elif fam == "word":
                text = f"{num2word(a)} plus {num2word(b)} equals "
            elif fam == "mixed":
                text = f"{a} plus {num2word(b)} is "
            else:  # narrative
                text = f"Sam had {a} marbles and found {b} more. Sam now has "
            rows.append({"a": a, "b": b, "text": text})
        families[fam] = rows
    return families


# ---- probes ----

def ridge_fit(X, Y, lam=RIDGE_LAMBDA):
    d = X.shape[1]
    M = np.linalg.solve(X.T @ X + lam * np.eye(d), X.T @ Y)
    return M


def linear_probe_eval(Xtr, Atr, Btr, Xte, Ate, Bte):
    Xtr1 = np.hstack([Xtr, np.ones((len(Xtr), 1))])
    Xte1 = np.hstack([Xte, np.ones((len(Xte), 1))])
    M = ridge_fit(Xtr1, np.stack([Atr, Btr], axis=1).astype(np.float64))
    pred = Xte1 @ M
    a_hat = np.round(pred[:, 0]).astype(int)
    b_hat = np.round(pred[:, 1]).astype(int)
    return a_hat, b_hat


def design_row(x, mean, std):
    row = [1.0, (x - mean) / (std + 1e-9)]
    for T in PERIODS:
        row += [np.cos(2 * np.pi * x / T), np.sin(2 * np.pi * x / T)]
    return np.array(row)


def fourier_probe_eval(Xtr, Atr, Btr, Xte, Ate, Bte, lo, hi):
    Xtr1 = np.hstack([Xtr, np.ones((len(Xtr), 1))])
    Xte1 = np.hstack([Xte, np.ones((len(Xte), 1))])
    mean = float((lo + hi) / 2)
    std = float(np.std(np.arange(lo, hi + 1)))
    codebook = np.stack([design_row(x, mean, std) for x in range(lo, hi + 1)])  # (V, 10)

    # fit separate decoders for a and b using their own training targets
    Ba = np.stack([design_row(x, mean, std) for x in Atr])
    Bb = np.stack([design_row(x, mean, std) for x in Btr])
    Ma = ridge_fit(Xtr1, Ba)
    Mb = ridge_fit(Xtr1, Bb)
    pred_a = Xte1 @ Ma  # (N, 10)
    pred_b = Xte1 @ Mb
    # nearest-neighbour decode against the codebook (respects periodic structure)
    a_hat = np.array([lo + int(np.argmin(np.linalg.norm(codebook - row, axis=1))) for row in pred_a])
    b_hat = np.array([lo + int(np.argmin(np.linalg.norm(codebook - row, axis=1))) for row in pred_b])
    return a_hat, b_hat


def mlp_probe_eval(Xtr, Atr, Btr, Xte, Ate, Bte):
    d = Xtr.shape[1]
    mu, sd = Xtr.mean(0, keepdims=True), Xtr.std(0, keepdims=True) + 1e-6
    Xtr_n = mx.array(((Xtr - mu) / sd).astype(np.float32))
    Xte_n = mx.array(((Xte - mu) / sd).astype(np.float32))
    Ytr = mx.array(np.stack([Atr, Btr], axis=1).astype(np.float32))

    mlp = nn.Sequential(nn.Linear(d, MLP_HIDDEN), nn.ReLU(), nn.Linear(MLP_HIDDEN, 2))
    opt = optim.Adam(learning_rate=MLP_LR)

    def loss_fn(model, x, y):
        return nn.losses.mse_loss(model(x), y)

    lv_and_grad = nn.value_and_grad(mlp, loss_fn)
    for _ in range(MLP_EPOCHS):
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


def main():
    t0 = time.time()
    rng = np.random.default_rng(SEED)
    print(f"loading {MODEL_ID} ...", flush=True)
    model, tokenizer = mlx_lm.load(MODEL_ID)
    nL, hidden, trace_last = build(model, tokenizer)
    print(f"loaded; {nL} layers, hidden={hidden} ({time.time()-t0:.1f}s)", flush=True)

    battery = make_battery(rng)
    fam_names = list(battery.keys())
    print(f"battery: {sum(len(v) for v in battery.values())} prompts across {fam_names}", flush=True)

    want = set(LAYERS)
    # capture: per family -> list of {a,b, resid:{layer: vec}}
    captured = {fam: [] for fam in fam_names}
    n_done = 0
    n_total = sum(len(v) for v in battery.values())
    for fam, rows in battery.items():
        for row in rows:
            resid = trace_last(row["text"], want)
            captured[fam].append({"a": row["a"], "b": row["b"], "resid": resid})
            n_done += 1
            if n_done % 20 == 0:
                print(f"  captured {n_done}/{n_total} ({time.time()-t0:.1f}s)", flush=True)
    print(f"capture done ({time.time()-t0:.1f}s)", flush=True)

    results = {"model": MODEL_ID, "n_layers": nL, "layers": LAYERS,
               "n_per_family": N_PER_FAMILY, "val_range": list(VAL_RANGE),
               "families": fam_names, "held_out_family": {}, "random_split": {}}

    # ---- held-out-family: train on 3 families pooled, test on the 4th, rotate ----
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
            a_hat, b_hat = linear_probe_eval(Xtr, Atr, Btr, Xte, Ate, Bte)
            row["linear"] = round(exact_pair_rate(a_hat, b_hat, Ate, Bte), 3)
            a_hat, b_hat = fourier_probe_eval(Xtr, Atr, Btr, Xte, Ate, Bte, *VAL_RANGE)
            row["fourier"] = round(exact_pair_rate(a_hat, b_hat, Ate, Bte), 3)
            a_hat, b_hat = mlp_probe_eval(Xtr, Atr, Btr, Xte, Ate, Bte)
            row["mlp"] = round(exact_pair_rate(a_hat, b_hat, Ate, Bte), 3)
            results["held_out_family"][held][L] = row
        best = max(max(v.values()) for v in results["held_out_family"][held].values())
        print(f"held-out={held:10s} best-across-layers-and-probes={best:.3f}  ({time.time()-t0:.1f}s)", flush=True)

    # ---- random split baseline (pooled, 80/20), for reference against held-out-family ----
    all_rows = [r for f in fam_names for r in captured[f]]
    idx = rng.permutation(len(all_rows))
    n_test = max(1, len(all_rows) // 5)
    te_idx, tr_idx = idx[:n_test], idx[n_test:]
    for L in LAYERS:
        X = np.stack([r["resid"][L] for r in all_rows])
        A = np.array([r["a"] for r in all_rows])
        B = np.array([r["b"] for r in all_rows])
        row = {}
        a_hat, b_hat = linear_probe_eval(X[tr_idx], A[tr_idx], B[tr_idx], X[te_idx], A[te_idx], B[te_idx])
        row["linear"] = round(exact_pair_rate(a_hat, b_hat, A[te_idx], B[te_idx]), 3)
        a_hat, b_hat = fourier_probe_eval(X[tr_idx], A[tr_idx], B[tr_idx], X[te_idx], A[te_idx], B[te_idx], *VAL_RANGE)
        row["fourier"] = round(exact_pair_rate(a_hat, b_hat, A[te_idx], B[te_idx]), 3)
        a_hat, b_hat = mlp_probe_eval(X[tr_idx], A[tr_idx], B[tr_idx], X[te_idx], A[te_idx], B[te_idx])
        row["mlp"] = round(exact_pair_rate(a_hat, b_hat, A[te_idx], B[te_idx]), 3)
        results["random_split"][L] = row

    OUT.write_text(json.dumps(results, indent=2))
    print(f"\nwrote {OUT} ({time.time()-t0:.1f}s)", flush=True)

    # summary
    print("\n=== summary: best (probe, layer) per held-out family ===")
    for held in fam_names:
        table = results["held_out_family"][held]
        best_layer, best_probe, best_val = None, None, -1
        for L, row in table.items():
            for probe, val in row.items():
                if val > best_val:
                    best_val, best_layer, best_probe = val, L, probe
        print(f"  held-out={held:10s} best={best_probe}@L{best_layer} exact-pair={best_val:.3f}")


if __name__ == "__main__":
    main()
