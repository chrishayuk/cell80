#!/usr/bin/env python3
"""CN-0 follow-up: is narrative's near-total readout failure (0-3% at every
layer, both wave-1 runs) a wrong-tap-position artifact, or a genuine
representational gap?

Narrative embeds operands many tokens before the generation point ("Sam had
{a} marbles and found {b} more. Sam now has ") — unlike digit/word/mixed,
where the operands sit immediately before the tap. The original CN-0 script
only ever reads the LAST token's residual (the natural tap for "what comes
next"). This script tests a different, more surgical hypothesis: is the
operand information present in the residual stream AT THE OPERAND'S OWN
TOKEN POSITION, just not at the tap position downstream of it?

Method: locate each operand's last digit-token index via prefix
tokenization (verified manually: "Sam had 72 marbles..." -> the '2' of '72'
sits at token index 5, well before the tap at index 17). Capture the full
per-layer residual at three feature reads: (a) tap-only (the original
method, for reference), (b) concat[resid(pos_a), resid(pos_b)] (read
exactly where the operands are), (c) mean-pooled over the whole sequence (a
cheaper, position-agnostic alternative). Compare in-distribution (random
80/20 split within narrative alone) exact-pair recovery across all three —
this isolates "is the info there, readable from the right place" from the
separate held-out-family generalization question CN-0's main script already
answered.

Run: python3 cn0_narrative_probe.py
"""
from __future__ import annotations

import json
import time
from pathlib import Path

import mlx.core as mx
import mlx.nn as nn
import mlx_lm
import numpy as np

MODEL_ID = "google/gemma-3-4b-it"
import os
OUT = Path(__file__).resolve().parent / os.environ.get("CN0_OUT", "cn0_narrative_probe_results.json")

LAYERS = ["embed", 0, 1, 2, 5] + list(range(19, 27))  # null (embed, L0-2,5) + the peak region
N = 200
VAL_RANGE = (1, 99)
SEED = 7
RIDGE_LAMBDA_GRID = [0.01, 0.03, 0.1, 0.3, 1.0, 3.0, 10.0, 30.0, 100.0]
PERIODS = [2, 5, 10, 100]


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

    max_want = max(L for L in LAYERS if isinstance(L, int))

    def trace_full(text, want):
        ids = mx.array([tokenizer.encode(text)])
        mask = nn.MultiHeadAttention.create_additive_causal_mask(ids.shape[1]).astype(mx.bfloat16)
        h = embed(ids) * (hidden ** 0.5)
        out = {}
        if "embed" in want:
            # the null: raw token embedding, zero transformer computation. If a probe
            # reads this as cleanly as a deep layer, it's decoding the tokenizer, not
            # anything the model computed.
            out["embed"] = np.array(h[0].astype(mx.float32))
        for i in range(nL):
            h = run_block(h, i, mask)
            if i in want:
                out[i] = np.array(h[0].astype(mx.float32))  # (seq_len, hidden) - every position
            if i >= max_want:
                break
        return out

    return nL, hidden, trace_full


def operand_positions(tokenizer, text, a, b):
    a_str, b_str = str(a), str(b)
    idx_a = text.index(a_str)
    idx_b = text.index(b_str, idx_a + len(a_str))
    pos_a = len(tokenizer.encode(text[: idx_a + len(a_str)])) - 1
    pos_b = len(tokenizer.encode(text[: idx_b + len(b_str)])) - 1
    return pos_a, pos_b


def make_narrative_battery(rng, n):
    rows = []
    for _ in range(n):
        a = int(rng.integers(*VAL_RANGE, endpoint=True))
        b = int(rng.integers(*VAL_RANGE, endpoint=True))
        text = f"Sam had {a} marbles and found {b} more. Sam now has "
        rows.append({"a": a, "b": b, "text": text})
    return rows


def ridge_fit(X, Y, lam):
    d = X.shape[1]
    return np.linalg.solve(X.T @ X + lam * np.eye(d), X.T @ Y)


def design_row(x, mean, std):
    row = [1.0, (x - mean) / (std + 1e-9)]
    for T in PERIODS:
        row += [np.cos(2 * np.pi * x / T), np.sin(2 * np.pi * x / T)]
    return np.array(row)


def fourier_probe(Xtr, Atr, Btr, Xte, Ate, Bte, lo, hi, lam):
    Xtr1 = np.hstack([Xtr, np.ones((len(Xtr), 1))])
    Xte1 = np.hstack([Xte, np.ones((len(Xte), 1))])
    mean, std = float((lo + hi) / 2), float(np.std(np.arange(lo, hi + 1)))
    codebook = np.stack([design_row(x, mean, std) for x in range(lo, hi + 1)])
    Ma = ridge_fit(Xtr1, np.stack([design_row(x, mean, std) for x in Atr]), lam)
    Mb = ridge_fit(Xtr1, np.stack([design_row(x, mean, std) for x in Btr]), lam)
    pred_a, pred_b = Xte1 @ Ma, Xte1 @ Mb
    a_hat = np.array([lo + int(np.argmin(np.linalg.norm(codebook - r, axis=1))) for r in pred_a])
    b_hat = np.array([lo + int(np.argmin(np.linalg.norm(codebook - r, axis=1))) for r in pred_b])
    return a_hat, b_hat


def linear_probe(Xtr, Atr, Btr, Xte, Ate, Bte, lam):
    Xtr1 = np.hstack([Xtr, np.ones((len(Xtr), 1))])
    Xte1 = np.hstack([Xte, np.ones((len(Xte), 1))])
    M = ridge_fit(Xtr1, np.stack([Atr, Btr], axis=1).astype(np.float64), lam)
    pred = Xte1 @ M
    return np.round(pred[:, 0]).astype(int), np.round(pred[:, 1]).astype(int)


def exact_pair_rate(a_hat, b_hat, Ate, Bte):
    return float(((a_hat == Ate) & (b_hat == Bte)).mean())


def tune_and_eval(probe_fn, Xtr, Atr, Btr, Xte, Ate, Bte, rng, extra_args=()):
    idx = rng.permutation(len(Xtr))
    n_val = max(1, len(Xtr) // 5)
    val_idx, in_tr_idx = idx[:n_val], idx[n_val:]
    best_lam, best_score = RIDGE_LAMBDA_GRID[0], -1.0
    for lam in RIDGE_LAMBDA_GRID:
        a_hat, b_hat = probe_fn(Xtr[in_tr_idx], Atr[in_tr_idx], Btr[in_tr_idx],
                                 Xtr[val_idx], Atr[val_idx], Btr[val_idx], *extra_args, lam)
        score = exact_pair_rate(a_hat, b_hat, Atr[val_idx], Btr[val_idx])
        if score > best_score:
            best_score, best_lam = score, lam
    a_hat, b_hat = probe_fn(Xtr, Atr, Btr, Xte, Ate, Bte, *extra_args, best_lam)
    return exact_pair_rate(a_hat, b_hat, Ate, Bte), best_lam


def main():
    t0 = time.time()
    rng = np.random.default_rng(SEED)
    print(f"loading {MODEL_ID} ...", flush=True)
    model, tokenizer = mlx_lm.load(MODEL_ID)
    nL, hidden, trace_full = build(model, tokenizer)
    print(f"loaded; {nL} layers, hidden={hidden} ({time.time()-t0:.1f}s)", flush=True)

    rows = make_narrative_battery(rng, N)
    want = set(LAYERS)
    captured = []
    for i, row in enumerate(rows):
        pos_a, pos_b = operand_positions(tokenizer, row["text"], row["a"], row["b"])
        full = trace_full(row["text"], want)
        tap = {L: full[L][-1] for L in LAYERS}
        at_a = {L: full[L][pos_a] for L in LAYERS}
        at_b = {L: full[L][pos_b] for L in LAYERS}
        pooled = {L: full[L].mean(axis=0) for L in LAYERS}
        captured.append({"a": row["a"], "b": row["b"], "pos_a": pos_a, "pos_b": pos_b,
                          "tap": tap, "at_a": at_a, "at_b": at_b, "pooled": pooled})
        if (i + 1) % 40 == 0:
            print(f"  captured {i+1}/{N} ({time.time()-t0:.1f}s)", flush=True)
    print(f"capture done ({time.time()-t0:.1f}s)", flush=True)

    A = np.array([r["a"] for r in captured])
    B = np.array([r["b"] for r in captured])
    idx = rng.permutation(N)
    n_test = max(1, N // 5)
    te_idx, tr_idx = idx[:n_test], idx[n_test:]

    results = {"n": N, "layers": LAYERS, "features": {}}
    feature_builders = {
        "tap": lambda L: np.stack([r["tap"][L] for r in captured]),
        "operand_positions": lambda L: np.stack(
            [np.concatenate([r["at_a"][L], r["at_b"][L]]) for r in captured]),
        "mean_pooled": lambda L: np.stack([r["pooled"][L] for r in captured]),
    }

    for feat_name, build_X in feature_builders.items():
        results["features"][feat_name] = {}
        for L in LAYERS:
            X = build_X(L)
            row = {}
            score, lam = tune_and_eval(linear_probe, X[tr_idx], A[tr_idx], B[tr_idx],
                                        X[te_idx], A[te_idx], B[te_idx], rng)
            row["linear"] = round(score, 3)
            score, lam = tune_and_eval(fourier_probe, X[tr_idx], A[tr_idx], B[tr_idx],
                                        X[te_idx], A[te_idx], B[te_idx], rng, extra_args=VAL_RANGE)
            row["fourier"] = round(score, 3)
            results["features"][feat_name][L] = row
        best = max(max(v.values()) for v in results["features"][feat_name].values())
        print(f"  feature={feat_name:18s} best-across-layers={best:.3f}  ({time.time()-t0:.1f}s)", flush=True)

    OUT.write_text(json.dumps(results, indent=2))
    print(f"\nwrote {OUT} ({time.time()-t0:.1f}s)", flush=True)

    print("\n=== summary: best (probe, layer) per feature representation ===")
    for feat_name, table in results["features"].items():
        best_layer, best_probe, best_val = None, None, -1
        for L, row in table.items():
            for probe, val in row.items():
                if val > best_val:
                    best_val, best_layer, best_probe = val, L, probe
        print(f"  feature={feat_name:18s} best={best_probe}@L{best_layer} exact-pair={best_val:.3f}")


if __name__ == "__main__":
    main()
