#!/usr/bin/env python3
"""CN-7 math-only control — the S1 in-weights cliff (companion to cn7_offdist.py).

`cn7_offdist.py` measures the failure PROFILE of the S2 beyond-tier cells (mul_sat,
safe_div, ceil_div, add_sat, sub_sat, round_to_multiple) via the trained `<call>...</call>`
story surface. That surface is meaningless for a math-only checkpoint that never saw a
cell token (its embedding rows for `<call>`/cell ids are untrained xavier-init noise).

This script asks the analogous question directly on S1's plain in-weights arithmetic
surface ("7 + 5 = 12"), which the math-only corpus (S1+S4, `--s2 0 --s3-per-cell 0`) is
built entirely from: teacher-forced answer NLL + argmax-exact in three operand-magnitude
bands per op —
  B0 in-range   : S1's own trained range (tier_a_instance's boundary)
  B1 one-past   : one more decimal digit on the operand(s)
  B2 well-past  : two more

Same band/scoring shape as cn7_offdist.py (`answer_stats`-style teacher forcing over the
answer span), applied to add_sat/sub_sat/mul_sat/safe_mod's S1 canonical templates instead
of the S2 call-injection template.

Run: python3 cn7_offdist_s1.py --ckpt cn7_ckpt_midtrain_mathonly.pt [--n 150]
"""
from __future__ import annotations

import argparse
import json
import random
import time
from pathlib import Path

from artifact_paths import checkpoint_input

import torch
import torch.nn.functional as F

import cn1_model
from cn1_model import resize_embedding
from cn1_corpus import Oracle
from cn7_corpus import Enc

HERE = Path(__file__).resolve().parent

# op -> (cell, [B0/B1/B2 arg generators], template(a, b, r) -> full canonical text)
BANDS = {
    "add": ("add_sat",
            [lambda r: [r.randint(0, 99), r.randint(0, 99)],
             lambda r: [r.randint(100, 999), r.randint(100, 999)],
             lambda r: [r.randint(1000, 9999), r.randint(1000, 9999)]],
            lambda a, b, res: f"{a} + {b} = {res}"),
    "sub": ("sub_sat",
            [lambda r: (lambda a: [a, r.randint(0, a)])(r.randint(1, 99)),
             lambda r: (lambda a: [a, r.randint(0, a)])(r.randint(100, 999)),
             lambda r: (lambda a: [a, r.randint(0, a)])(r.randint(1000, 9999))],
            lambda a, b, res: f"{a} - {b} = {res}"),
    "mul": ("mul_sat",
            [lambda r: [r.randint(0, 12), r.randint(0, 12)],
             lambda r: [r.randint(13, 99), r.randint(13, 99)],
             lambda r: [r.randint(100, 999), r.randint(100, 999)]],
            lambda a, b, res: f"{a} x {b} = {res}"),
    "mod": ("safe_mod",
            [lambda r: [r.randint(0, 99), r.randint(2, 12)],
             lambda r: [r.randint(100, 999), r.randint(2, 12)],
             lambda r: [r.randint(1000, 9999), r.randint(2, 12)]],
            lambda a, b, res: f"{a} mod {b} = {res}"),
}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ckpt", required=True)
    ap.add_argument("--n", type=int, default=150)
    ap.add_argument("--device", default="cpu")
    args = ap.parse_args()
    t0 = time.time()

    tokmap = json.load(open(HERE / "cn7_token_map.json"))
    enc = Enc(tokmap["cells"])
    oracle = Oracle(sorted({cell for cell, *_ in BANDS.values()}))
    rng = random.Random(984)  # distinct from cn7_offdist.py's 983 and cn7_probe.py's 982

    from tiny_model_v11.loader import load_from_artifacts
    base, cfg = load_from_artifacts(str(cn1_model.TINY_MODEL / "model" / "v11"), device="cpu")
    ck = torch.load(checkpoint_input(args.ckpt), map_location="cpu")
    resize_embedding(base, ck["vocab"])
    base.load_state_dict(ck["state"])
    base = base.to(args.device).eval()

    @torch.no_grad()
    def answer_stats(op, a, b, res):
        template = BANDS[op][2]
        full_text = template(a, b, res)
        prefix_text = full_text[:full_text.rindex(str(res))]
        _, prefix_ids, _ = enc.encode([(prefix_text, 1)])
        _, full_ids, _ = enc.encode([(full_text, 1)])
        k = len(prefix_ids)
        n = len(full_ids) - k
        if n <= 0:
            return None
        x = torch.tensor([full_ids], device=args.device)
        lg = base(x)[0]
        tgt = x[0, k:k + n]
        nll = float(F.cross_entropy(lg[k - 1:k - 1 + n], tgt, reduction="mean"))
        exact = bool((lg[k - 1:k - 1 + n].argmax(-1) == tgt).all())
        return nll, exact

    out = {"ckpt": args.ckpt, "cells": {}}
    for op, (cell, gens, _template) in BANDS.items():
        row = {}
        for b_idx, gen in enumerate(gens):
            nlls, exacts = [], []
            tries = 0
            while len(nlls) < args.n and tries < args.n * 8:
                tries += 1
                a, b = gen(rng)
                r = oracle.run(cell, [a, b])
                if r.get("halt") != "returned" or r["result"] == 65535:  # saturation excluded
                    continue
                res = r["result"]
                stats = answer_stats(op, a, b, res)
                if stats is None:
                    continue
                nll, exact = stats
                nlls.append(nll); exacts.append(exact)
            row[f"B{b_idx}"] = {"n": len(nlls), "nll": round(sum(nlls) / max(1, len(nlls)), 3),
                                 "exact": round(sum(exacts) / max(1, len(exacts)), 3)}
        out["cells"][op] = row
        print(f"  {op:<6} " + "  ".join(
            f"B{b_idx}: nll {row[f'B{b_idx}']['nll']:.2f} exact {row[f'B{b_idx}']['exact']:.2f} "
            f"(n={row[f'B{b_idx}']['n']})" for b_idx in range(3)), flush=True)

    path = HERE / f"cn7_offdist_s1_{Path(args.ckpt).stem}.json"
    path.write_text(json.dumps(out, indent=1))
    print(f"wrote {path.name} ({time.time()-t0:.0f}s)")


if __name__ == "__main__":
    main()
