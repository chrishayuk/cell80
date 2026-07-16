#!/usr/bin/env python3
"""CN-7.5 off-distribution probe — the failure PROFILE (prereg §8.11, shape frozen pre-run).

For each S2 beyond-tier cell, teacher-forced answer NLL + argmax-exact in three bands:
  B0 in-range   : the training generator's own ranges
  B1 one-past   : one digit past the trained operand range
  B2 well-past  : two past
Crammed distribution -> cliff at the B0/B1 boundary (cliff location = training range echoed
back, a corpus-consistency check). Compressed circuit -> graceful degradation with carry
depth (add/sub report per-carry-depth lines). Saturated instances (u16 clamp) are EXCLUDED —
a constant answer is trivially learnable and would fake robustness.

Items use the exact S2 story templates (the trained surface); the measured span is the
injected answer position.

Run: python3 cn7_offdist.py --ckpt cn7_ckpt_midtrain_nomask.pt [--n 150]
"""
from __future__ import annotations

import argparse
import json
import random
import time
from pathlib import Path

import torch
import torch.nn.functional as F

import cn1_model
from cn1_model import resize_embedding
from cn1_corpus import Oracle
from cn7_corpus import Enc, S2_BEYOND

HERE = Path(__file__).resolve().parent

BANDS = {
    "mul_sat": [lambda r: [r.randint(13, 99), r.randint(13, 99)],
                lambda r: [r.randint(100, 999), r.randint(13, 99)],
                lambda r: [r.randint(100, 999), r.randint(100, 999)]],
    "safe_div": [lambda r: [r.randint(100, 999), r.randint(3, 19)],
                 lambda r: [r.randint(1000, 9999), r.randint(3, 19)],
                 lambda r: [r.randint(10000, 65535), r.randint(21, 99)]],
    "ceil_div": [lambda r: [r.randint(100, 999), r.randint(6, 24)],
                 lambda r: [r.randint(1000, 9999), r.randint(6, 24)],
                 lambda r: [r.randint(10000, 65535), r.randint(25, 99)]],
    "add_sat": [lambda r: [r.randint(100, 999), r.randint(100, 999)],
                lambda r: [r.randint(1000, 9999), r.randint(1000, 9999)],
                lambda r: [r.randint(10000, 32000), r.randint(10000, 32000)]],
    "sub_sat": [lambda r: [r.randint(500, 999), r.randint(100, 499)],
                lambda r: [r.randint(5000, 9999), r.randint(1000, 4999)],
                lambda r: [r.randint(50000, 65535), r.randint(10000, 49999)]],
    "round_to_multiple": [lambda r: [r.randint(100, 999), r.choice([25, 50, 100])],
                          lambda r: [r.randint(1000, 9999), r.choice([25, 50, 100])],
                          lambda r: [r.randint(10000, 65535), r.choice([25, 50, 100])]],
}
STORY = {c: (story, tail) for c, _, story, tail in S2_BEYOND}


def carries(a, b):
    c = n = 0
    while a or b:
        n += (a % 10 + b % 10 + c) >= 10
        c = 1 if (a % 10 + b % 10 + c) >= 10 else 0
        a //= 10; b //= 10
    return n


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ckpt", required=True)
    ap.add_argument("--n", type=int, default=150)
    ap.add_argument("--device", default="cpu")
    args = ap.parse_args()
    t0 = time.time()

    tokmap = json.load(open(HERE / "cn7_token_map.json"))
    enc = Enc(tokmap["cells"])
    oracle = Oracle(sorted(BANDS))
    rng = random.Random(983)  # unused elsewhere

    from tiny_model_v11.loader import load_from_artifacts
    base, cfg = load_from_artifacts(str(cn1_model.TINY_MODEL / "model" / "v11"), device="cpu")
    ck = torch.load(HERE / args.ckpt, map_location="cpu")
    resize_embedding(base, ck["vocab"])
    base.load_state_dict(ck["state"])
    base = base.to(args.device).eval()

    @torch.no_grad()
    def answer_stats(cell, args_, res):
        story, tail = STORY[cell]
        pre = [(story(args_), 1), (f"<call> ⟨{cell}⟩ {' '.join(map(str, args_))} </call> ", 1)]
        _, pre_ids, _ = enc.encode(pre)
        _, full_ids, _ = enc.encode(pre + [(str(res), 0), (tail, 1)])
        k = len(pre_ids)
        n = len(enc.encode(pre + [(str(res), 0)])[1]) - k
        x = torch.tensor([full_ids], device=args.device)
        lg = base(x)[0]
        tgt = x[0, k:k + n]
        nll = float(F.cross_entropy(lg[k - 1:k - 1 + n], tgt, reduction="mean"))
        exact = bool((lg[k - 1:k - 1 + n].argmax(-1) == tgt).all())
        return nll, exact

    out = {"ckpt": args.ckpt, "cells": {}}
    for cell, gens in BANDS.items():
        row = {}
        for b, gen in enumerate(gens):
            nlls, exacts, cd = [], [], {}
            tries = 0
            while len(nlls) < args.n and tries < args.n * 8:
                tries += 1
                a = gen(rng)
                r = oracle.run(cell, a)
                if r.get("halt") != "returned" or r["result"] == 65535:  # saturation excluded
                    continue
                res = r["result"]
                nll, exact = answer_stats(cell, a, res)
                nlls.append(nll); exacts.append(exact)
                if cell in ("add_sat", "sub_sat"):
                    d = carries(a[0], a[1]) if cell == "add_sat" else carries(a[0] - a[1] if a[0] >= a[1] else 0, a[1])
                    e = cd.setdefault(d, [0, 0])
                    e[0] += exact; e[1] += 1
            row[f"B{b}"] = {"n": len(nlls), "nll": round(sum(nlls) / max(1, len(nlls)), 3),
                            "exact": round(sum(exacts) / max(1, len(exacts)), 3)}
            if cd:
                row[f"B{b}"]["by_carries"] = {str(k): f"{a}/{b_}" for k, (a, b_) in sorted(cd.items())}
        out["cells"][cell] = row
        print(f"  {cell:<18} " + "  ".join(f"B{b}: nll {row[f'B{b}']['nll']:.2f} exact {row[f'B{b}']['exact']:.2f} (n={row[f'B{b}']['n']})" for b in range(3)), flush=True)

    path = HERE / f"cn7_offdist_{Path(args.ckpt).stem}.json"
    path.write_text(json.dumps(out, indent=1))
    print(f"wrote {path.name} ({time.time()-t0:.0f}s)")


if __name__ == "__main__":
    main()
