#!/usr/bin/env python3
"""CN-7.3 — stratified generation eval on the midtrained v11 (the headline), and the P-b gate.

Exact CN-6 stage-2 shape: per held-out cell (n=24), prompt = descriptor + " <call>", the model
greedily emits an I/O-example spec, we parse pairs, run the true cell on the emitted inputs
(per-pair CORRECTNESS), and rank the true cell among all value cells by behavioural routing
(resolve@k). Stratified by the FROZEN classification (cn7_frontier_classification.json);
Wilson CIs; per-stratum counts reported alongside rates (prereg §4 CN-7.3).

P-b (panel gate, graded first): within-frontier per-pair correctness >= 0.83. If P-b fails,
7.3's resolution numbers are recorded but the experiment is not graded (the uninformative
outcome the 7.2 gate exists to prevent).

Decode: no BOS (training format), legality-masked, stop at </call>, budget 48.

Run: python3 cn7_eval.py --ckpt cn7_ckpt_midtrain.pt
"""
from __future__ import annotations

import argparse
import json
import math
import re
import time
from pathlib import Path

import torch

import cell80_py
import cn1_model
from cn1_model import resize_embedding
from cn7_corpus import CALL_ID, CLOSE_ID, Enc
from cn7_deck import decode_mask
from artifact_paths import checkpoint_input, dataset_input

HERE = Path(__file__).resolve().parent
CELLS_DIR = HERE.parent.parent / "cell80" / "cells"


def wilson(k, n):
    if n == 0:
        return (0.0, 0.0, 0.0)
    p = k / n
    z = 1.96
    d = 1 + z * z / n
    c = (p + z * z / (2 * n)) / d
    h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / d
    return p, max(0, c - h), min(1, c + h)


def parse_spec(text):
    out = []
    for chunk in text.split(";"):
        if "=" not in chunk:
            continue
        lhs, rhs = chunk.split("=", 1)
        nums = re.findall(r"-?\d+", lhs)
        rnum = re.findall(r"-?\d+", rhs)
        if nums and rnum:
            out.append(([int(x) & 0xFFFF for x in nums], int(rnum[0]) & 0xFFFF))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ckpt", required=True)
    ap.add_argument("--k", type=int, nargs="+", default=[1, 5, 10])
    ap.add_argument("--device", default="cpu")
    args = ap.parse_args()
    t0 = time.time()

    tokmap = json.load(open(HERE / "cn7_token_map.json"))
    enc = Enc(tokmap["cells"])
    strata = json.load(open(HERE / "cn7_frontier_classification.json"))["cells"]
    lib = {json.loads(l)["name"]: json.loads(l) for l in (HERE / "cn1_library.jsonl").read_text().splitlines() if l.strip()}
    value = [n for n, r in lib.items() if r["arity"] >= 1]
    ev = [json.loads(l) for l in dataset_input("cn6_corpus_eval_generation.jsonl").read_text().splitlines() if l.strip()]
    contexts = {}
    for r in ev:
        contexts.setdefault(r["cell"], r["context"])
    assert set(contexts) == set(strata)

    host = cell80_py.CellHost()
    handles = {}
    for n in value:
        try:
            host.add_source(n, next(CELLS_DIR.rglob(f"{n}.rs")).read_text())
            handles[n] = host.load(n)
        except Exception:
            pass
    value = [n for n in value if n in handles]

    from tiny_model_v11.loader import load_from_artifacts
    base, cfg = load_from_artifacts(str(cn1_model.TINY_MODEL / "model" / "v11"), device="cpu")
    ck = torch.load(checkpoint_input(args.ckpt), map_location="cpu")
    resize_embedding(base, ck["vocab"])
    base.load_state_dict(ck["state"])
    base = base.to(args.device).eval()
    neg = torch.full((ck["vocab"],), float("-inf"), device=args.device)
    neg[decode_mask(ck["vocab"])] = 0.0

    @torch.no_grad()
    def gen_spec(context):
        ids = enc.seg_ids(context) + [CALL_ID]
        toks = []
        for _ in range(48):
            lg = base(torch.tensor([ids + toks], device=args.device))[0, -1] + neg
            t = int(lg.argmax())
            if t == CLOSE_ID or t == 3:
                break
            toks.append(t)
        return enc.sp.decode([t for t in toks if t < 71261])

    def route_rank(name, pairs):
        if not pairs:
            return len(value)
        ranked = host.route([(list(a), o) for a, o in pairs], limit=len(value))
        names = [r.get("id") if isinstance(r, dict) else r for r in ranked]
        return names.index(name) if name in names else len(value)

    per = []
    for name in sorted(contexts):
        seg = gen_spec(contexts[name])
        pairs = parse_spec(seg)
        good = 0
        for a, o in pairs:
            try:
                r = host.run(handles[name], list(a))
                good += r.get("halt") == "returned" and r["result"] == o
            except Exception:
                pass
        corr = good / len(pairs) if pairs else 0.0
        rk = route_rank(name, pairs)
        per.append({"cell": name, "stratum": strata[name]["stratum"], "n_pairs": len(pairs),
                    "good_pairs": good, "corr": round(corr, 3), "rank": rk, "raw": seg.strip()[:100]})
        print(f"  {name:<32} [{strata[name]['stratum']:<6}] pairs {len(pairs)}  corr {corr:.2f}  rank {rk:>3}", flush=True)

    out = {"ckpt": args.ckpt, "per_cell": per, "strata": {}}
    print()
    for st in ("within", "beyond"):
        cells = [p for p in per if p["stratum"] == st]
        kp = sum(p["good_pairs"] for p in cells)
        np_ = sum(p["n_pairs"] for p in cells)
        pc, pl, ph = wilson(kp, np_)
        row = {"cells": len(cells), "pair_corr": [round(pc, 3), round(pl, 3), round(ph, 3)],
               "pairs": f"{kp}/{np_}"}
        for k in args.k:
            r, lo, hi = wilson(sum(p["rank"] < k for p in cells), len(cells))
            row[f"resolve@{k}"] = [round(r, 3), round(lo, 3), round(hi, 3),
                                   f"{sum(p['rank'] < k for p in cells)}/{len(cells)}"]
        out["strata"][st] = row
        print(f"  {st:<7} ({len(cells)} cells): pair-corr {pc:.3f} [{pl:.3f},{ph:.3f}] ({kp}/{np_})  "
              + "  ".join(f"r@{k} {row[f'resolve@{k}'][0]:.3f} ({row[f'resolve@{k}'][3]})" for k in args.k))

    pb = out["strata"]["within"]["pair_corr"][0]
    out["P-b"] = {"value": pb, "threshold": 0.83, "pass": pb >= 0.83}
    print(f"\n  P-b (within-frontier pair correctness >= 0.83): {pb:.3f} -> {'PASS' if pb >= 0.83 else 'FAIL'}")
    path = HERE / f"cn7_eval_{Path(args.ckpt).stem}.json"
    path.write_text(json.dumps(out, indent=1))
    print(f"wrote {path.name} ({time.time()-t0:.0f}s)")


if __name__ == "__main__":
    main()
