#!/usr/bin/env python3
"""Reload a saved arm checkpoint and compute the FULL rank distribution of the true cell among
the 790 masked candidates, per eval bucket — to confirm the fingerprint-vs-random held-out
contrast (median rank 59 vs 570) is not a single-statistic artifact. Eval only, no training.

Run: python3 cn1_eval_ckpt.py
"""
from __future__ import annotations

import json
import statistics as st
from pathlib import Path

import torch

import cn1_model
import cn1_decode

HERE = Path(__file__).resolve().parent


def reload_arm(arm, seed, device, unfreeze_top=12):
    model, names, held = cn1_model.build(arm)  # loads on cpu
    ckpt_path = HERE / f"cn1_ckpt_{arm}_s{seed}.pt"
    if not ckpt_path.exists():  # fall back to the pre-seed-suffix naming (first runs)
        ckpt_path = HERE / f"cn1_ckpt_{arm}.pt"
    ck = torch.load(ckpt_path, map_location="cpu")
    with torch.no_grad():
        model.base.embed.weight.copy_(ck["embed"])
    if arm in ("fingerprint", "shuffled", "description"):
        model.w_f.load_state_dict(ck["w_f"])
    blocks = model.base.layers[-unfreeze_top:]
    for i, blk in enumerate(blocks):
        blk.load_state_dict(ck[f"block_{i}"])
    if "norm" in ck:
        model.base.norm.load_state_dict(ck["norm"])
    model.eval()
    return model.to(device)  # run the eval forwards on the GPU (MPS), not CPU


def rank_stats(model, items, cell_ids_t, tok, cap=200):
    device = next(model.parameters()).device
    ranks = []
    with torch.no_grad():
        for r in items[:cap]:
            ids = torch.tensor([[2] + tok.encode(r["context"] + " <call>")], device=device)
            logits = model(ids)[0, -1]
            cl = logits[cell_ids_t]
            order = torch.argsort(cl, descending=True)
            ranked = cell_ids_t[order]
            pos = int((ranked == r["cell_id"]).nonzero().flatten()[0])
            ranks.append(pos)
    ranks.sort()
    n = len(ranks)
    return {
        "n": n,
        "mean": round(st.mean(ranks), 1),
        "median": ranks[n // 2],
        "p25": ranks[n // 4],
        "p75": ranks[3 * n // 4],
        "frac_top79": round(sum(x < 79 for x in ranks) / n, 3),  # top 10%
        "frac_top10": round(sum(x < 10 for x in ranks) / n, 3),
    }


def main():
    import argparse

    import v11

    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=80)
    ap.add_argument("--arms", nargs="+", default=["fingerprint", "shuffled", "random"])
    ap.add_argument("--device", default=None)
    a = ap.parse_args()

    device = a.device or ("mps" if torch.backends.mps.is_available() else "cpu")
    print(f"eval device: {device}")
    tok = v11.Tokenizer.from_file(str(HERE / "v11-cells.vocab.bin"))
    _, _, cell_ids, _ = cn1_decode.load_call_grammar()
    cell_ids_t = torch.tensor(sorted(cell_ids), device=device)
    eval_rows = [json.loads(l) for l in (HERE / "cn1_corpus_eval.jsonl").read_text().splitlines() if l.strip()]
    buckets = {
        "seen_x_seen": ("seen_cell", "seen_comp"),
        "seen_x_novel": ("seen_cell", "novel_comp"),
        "HELDOUT_x_seen": ("novel_cell", "seen_comp"),
        "HELDOUT_x_novel": ("novel_cell", "novel_comp"),
    }
    grouped = {k: [r for r in eval_rows if (r["bucket_cell"], r["bucket_comp"]) == v] for k, v in buckets.items()}
    for k, v in grouped.items():
        print(f"{k}: {len(v)} items")
    print("chance median rank ~395/790\n")
    for arm in a.arms:
        try:
            m = reload_arm(arm, a.seed, device)
        except FileNotFoundError:
            print(f"arm {arm}: no checkpoint yet, skipping")
            continue
        print(f"arm {arm}:")
        for k, items in grouped.items():
            print(f"  {k:<16} {rank_stats(m, items, cell_ids_t, tok)}")


if __name__ == "__main__":
    main()
