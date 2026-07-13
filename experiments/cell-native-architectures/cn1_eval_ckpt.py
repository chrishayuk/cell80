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


def reload_arm(arm, unfreeze_top=12):
    model, names, held = cn1_model.build(arm)  # cpu
    ck = torch.load(HERE / f"cn1_ckpt_{arm}.pt", map_location="cpu")
    with torch.no_grad():
        model.base.embed.weight.copy_(ck["embed"])
    if arm == "fingerprint":
        model.w_f.load_state_dict(ck["w_f"])
    blocks = model.base.layers[-unfreeze_top:]
    for i, blk in enumerate(blocks):
        blk.load_state_dict(ck[f"block_{i}"])
    model.eval()
    return model


def rank_stats(model, items, cell_ids_t, tok, cap=200):
    ranks = []
    with torch.no_grad():
        for r in items[:cap]:
            ids = torch.tensor([[2] + tok.encode(r["context"] + " <call>")])
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
    import v11

    tok = v11.Tokenizer.from_file(str(HERE / "v11-cells.vocab.bin"))
    _, _, cell_ids, _ = cn1_decode.load_call_grammar()
    cell_ids_t = torch.tensor(sorted(cell_ids))
    eval_rows = [json.loads(l) for l in (HERE / "cn1_corpus_eval.jsonl").read_text().splitlines() if l.strip()]
    held = [r for r in eval_rows if r["bucket_cell"] == "novel_cell" and r["bucket_comp"] == "seen_comp"]
    seen = [r for r in eval_rows if r["bucket_cell"] == "seen_cell" and r["bucket_comp"] == "seen_comp"]

    print(f"held-out (novel_cell x seen_comp): {len(held)} items;  chance median rank ~395/790\n")
    for arm in ["fingerprint", "random"]:
        m = reload_arm(arm)
        hs = rank_stats(m, held, cell_ids_t, tok)
        ss = rank_stats(m, seen, cell_ids_t, tok)
        print(f"arm {arm}:")
        print(f"  HELD-OUT  {hs}")
        print(f"  seen      {ss}")


if __name__ == "__main__":
    main()
