#!/usr/bin/env python3
"""CN-1 per-CELL recall@K (the metric the scale claim rests on). Random sampling fixed the first-N
bias, but per-ITEM stats still weight cells by how many queries they have. The scale claim is "can
a NEW CELL be found" — unit = cell. Per-cell recall@K = fraction of held-out cells whose per-cell
rank (median over its items) is <= K. Reported at the K_exec values that matter (CPU@4.8%=13,
median=98, CPU@100%=266, GPU@4.8%=4718, whole library=790).

Run: python3 cn1_percell_recall.py [--seed 81]
"""
from __future__ import annotations

import argparse
import json
import statistics as st
from collections import defaultdict
from pathlib import Path

import torch
import cn1_model
from artifact_paths import checkpoint_input, dataset_input
import cn1_decode

HERE = Path(__file__).resolve().parent


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=81)
    a = ap.parse_args()
    import v11
    tok = v11.Tokenizer.from_file(str(dataset_input("v11-cells.vocab.bin")))
    device = "mps" if torch.backends.mps.is_available() else "cpu"

    model, names, held = cn1_model.build("fingerprint")
    ck = torch.load(checkpoint_input(f"cn1_ckpt_fingerprint_s{a.seed}.pt"), map_location="cpu")
    with torch.no_grad():
        model.base.embed.weight.copy_(ck["embed"])
    model.w_f.load_state_dict(ck["w_f"])
    for i, blk in enumerate(model.base.layers[-16:]):
        blk.load_state_dict(ck[f"block_{i}"])
    if "norm" in ck:
        model.base.norm.load_state_dict(ck["norm"])
    model = model.to(device).eval()

    lib = {json.loads(l)["name"]: json.loads(l) for l in (HERE / "cn1_library.jsonl").read_text().splitlines() if l.strip()}
    _, _, cell_ids, _ = cn1_decode.load_call_grammar()
    cid = torch.tensor(sorted(cell_ids), device=device)
    ev = [json.loads(l) for l in dataset_input("cn1_corpus_eval.jsonl").read_text().splitlines() if l.strip()]
    ho = [r for r in ev if r["bucket_cell"] == "novel_cell" and r["bucket_comp"] == "seen_comp"]

    per_cell = defaultdict(list)
    with torch.no_grad():
        for r in ho:
            ids = torch.tensor([[2] + tok.encode(r["context"] + " <call>")], device=device)
            order = torch.argsort(model(ids)[0, -1][cid], descending=True)
            pos = int((cid[order] == r["cell_id"]).nonzero().flatten()[0])
            per_cell[r["cell"]].append(pos)

    cell_rank = {c: st.median(v) for c, v in per_cell.items()}
    val = {c: r for c, r in cell_rank.items() if lib[c]["arity"] >= 1}
    Ks = [13, 50, 98, 266, 790, 4718]
    print(f"seed {a.seed} | held-out cells with items: {len(cell_rank)} ({len(val)} value)")
    print(f"per-cell median rank — all held-out: median {st.median(list(cell_rank.values())):.0f} | "
          f"value only: median {st.median(list(val.values())):.0f}\n")
    print(f"{'K':>6} {'per-cell recall@K (all)':>26} {'value-only':>14}   (K_exec context)")
    ctx = {13: "CPU@4.8%", 98: "median", 266: "CPU@100%", 4718: "GPU@4.8%", 790: "whole lib"}
    for K in Ks:
        rall = sum(1 for r in cell_rank.values() if r <= K) / len(cell_rank)
        rval = sum(1 for r in val.values() if r <= K) / len(val)
        print(f"{K:>6} {rall:>26.3f} {rval:>14.3f}   {ctx.get(K,'')}")


if __name__ == "__main__":
    main()
