#!/usr/bin/env python3
"""CN-7.2 panel — P-a1/P-a2: in-tier probe correctness, canonical vs narrative.

Fresh S1 items (seed 982 — never used by corpus (80) or role-NLL (981)), split at the answer
span; the model greedily completes from the prefix; the completion's first numeral (or
even/odd word) is cell-signed. Canonical and narrative grade separately: P-a1 >= 0.90
canonical, P-a2 >= 0.80 narrative (B12, the paraphrase cliff, is why the split is a gate).

Decode restricted to the legality mask (cn7_decode_mask.pt, built by cn7_deck.py).

Run: python3 cn7_probe.py --ckpt cn7_ckpt_midtrain.pt [--n 400]
"""
from __future__ import annotations

import argparse
import json
import random
import re
import time
from pathlib import Path

from artifact_paths import checkpoint_input

import torch

import cn1_model
from cn1_model import resize_embedding
from cn1_corpus import Oracle
from cn7_corpus import Enc, s1_item, CELL_KIND
from cn7_deck import decode_mask

HERE = Path(__file__).resolve().parent
PROBE_SEED = 982


def wilson(k, n):
    import math
    if n == 0:
        return (0.0, 0.0, 0.0)
    p = k / n
    z = 1.96
    d = 1 + z * z / n
    c = (p + z * z / (2 * n)) / d
    h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / d
    return p, max(0, c - h), min(1, c + h)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ckpt", required=True)
    ap.add_argument("--n", type=int, default=400, help="target items per register")
    ap.add_argument("--device", default=None)
    args = ap.parse_args()
    device = args.device or ("mps" if torch.backends.mps.is_available() else "cpu")
    t0 = time.time()

    tokmap = json.load(open(HERE / "cn7_token_map.json"))
    enc = Enc(tokmap["cells"])
    oracle = Oracle(sorted(CELL_KIND))
    rng = random.Random(PROBE_SEED)

    from tiny_model_v11.loader import load_from_artifacts
    base, cfg = load_from_artifacts(str(cn1_model.TINY_MODEL / "model" / "v11"), device="cpu")
    ck = torch.load(checkpoint_input(args.ckpt), map_location="cpu")
    resize_embedding(base, ck["vocab"])
    base.load_state_dict(ck["state"])
    base = base.to(device).eval()
    neg = torch.full((ck["vocab"],), float("-inf"), device=device)
    neg[decode_mask(ck["vocab"])] = 0.0

    # build fresh probe items until each register has n usable ones
    CANON = re.compile(r"^\d|^smallest|^after")  # canonical forms start with a digit/keyword
    items = {"canonical": [], "narrative": []}
    guard = 0
    while (len(items["canonical"]) < args.n or len(items["narrative"]) < args.n) and guard < args.n * 30:
        guard += 1
        parts, meta = s1_item(rng, oracle)
        text = parts[0][0]
        if meta["op"] == "cmp":
            continue  # answer is relational, not a suffix span — excluded from this probe
        m = re.search(r"(\d+|even|odd)(?=[^\d]*$)", text)
        if not m or m.start() == 0:
            continue
        reg = "canonical" if CANON.match(text) else "narrative"
        if len(items[reg]) < args.n:
            items[reg].append({"prefix": text[:m.start()].rstrip(), "answer": m.group(1),
                               "op": meta["op"]})

    @torch.no_grad()
    def complete(prefix):
        ids = enc.seg_ids(prefix)
        toks = []
        for _ in range(6):
            lg = base(torch.tensor([[2] + ids + toks], device=device))[0, -1] + neg
            t = int(lg.argmax())
            if t == 3:
                break
            toks.append(t)
        return enc.sp.decode([t for t in toks if t < 71261])

    results = {}
    for reg, its in items.items():
        good, per_op = 0, {}
        for it in its:
            out = complete(it["prefix"])
            m = re.search(r"\d+|even|odd", out)
            ok = bool(m) and m.group(0) == it["answer"]
            good += ok
            po = per_op.setdefault(it["op"], [0, 0])
            po[0] += ok; po[1] += 1
        p, lo, hi = wilson(good, len(its))
        results[reg] = {"correct": good, "n": len(its), "p": round(p, 4),
                        "ci95": [round(lo, 4), round(hi, 4)],
                        "per_op": {k: f"{a}/{b}" for k, (a, b) in sorted(per_op.items())}}
        print(f"  {reg:<10} {good}/{len(its)} = {p:.3f} [{lo:.3f},{hi:.3f}]  {results[reg]['per_op']}", flush=True)

    gate = {"P-a1 (canonical >= 0.90)": results["canonical"]["p"] >= 0.90,
            "P-a2 (narrative >= 0.80)": results["narrative"]["p"] >= 0.80}
    for k, v in gate.items():
        print(f"  {k}: {'PASS' if v else 'FAIL'}")
    out = {"ckpt": args.ckpt, "probe_seed": PROBE_SEED, "results": results, "gate": gate}
    path = HERE / f"cn7_probe_{Path(args.ckpt).stem}.json"
    path.write_text(json.dumps(out, indent=1))
    print(f"wrote {path.name} ({time.time()-t0:.0f}s)")


if __name__ == "__main__":
    main()
