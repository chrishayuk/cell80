#!/usr/bin/env python3
"""CN-1 synthetic library expansion: generate ~9,200 synthetic cell fingerprints to grow the
library 790 -> ~10^4, so the scale curve gets another decade of N (pin alpha) and passes the
~2,500 softmax/token-vocab ceiling. The synthetics MUST preserve the real library's behavioural
DENSITY — if they cluster differently, the curve measures the perturbation choice, not the
mechanism. So each synthetic is a real cell's fingerprint with a fraction of probes resampled from
that probe's empirical marginal across the real library (preserves per-probe distributions and the
cluster structure). We calibrate the resample fraction so the synthetic->nearest-real agreement
distribution matches real->nearest-real, and report both distributions to prove the match.

Run: python3 cn1_synth_cells.py [--target 10000] [--p 0.30]
"""
from __future__ import annotations

import argparse
import json
import random
import statistics as st
from pathlib import Path

HERE = Path(__file__).resolve().parent


def agree(a, b):
    return sum(1 for x, y in zip(a, b) if x == y) / len(a)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--target", type=int, default=10000)
    ap.add_argument("--p", type=float, default=0.30, help="fraction of probes resampled per synthetic")
    args = ap.parse_args()
    rng = random.Random(11)

    lib = [json.loads(l) for l in (HERE / "cn1_library.jsonl").read_text().splitlines() if l.strip()]
    fps = [r["fingerprint"] for r in lib]           # 20 Option<u16> each (ints or None)
    n_probes = len(fps[0])
    # per-probe empirical pool of values across the real library
    pools = [[fp[i] for fp in fps] for i in range(n_probes)]

    n_synth = args.target - len(lib)
    synth = []
    for k in range(n_synth):
        src = rng.choice(fps)
        f = list(src)
        for i in range(n_probes):
            if rng.random() < args.p:
                f[i] = rng.choice(pools[i])
        synth.append({"name": f"__synth_{k}", "pack": "__synth", "arity": 2, "fingerprint": f, "synthetic": True})

    # density check: nearest-real agreement distribution, real vs synthetic (sampled for speed)
    real_sample = rng.sample(fps, 200)
    def nearest_real_agreement(f, exclude_self=None):
        best = 0.0
        for j, rf in enumerate(fps):
            if exclude_self is not None and j == exclude_self:
                continue
            a = agree(f, rf)
            if a > best:
                best = a
        return best
    real_nn = [nearest_real_agreement(fps[i], exclude_self=i) for i in rng.sample(range(len(fps)), 150)]
    synth_nn = [nearest_real_agreement(s["fingerprint"]) for s in rng.sample(synth, 150)]
    print(f"generated {n_synth} synthetic cells (p={args.p}) -> library {args.target}")
    print(f"nearest-real agreement — REAL cells:  median {st.median(real_nn):.3f}  mean {st.mean(real_nn):.3f}")
    print(f"nearest-real agreement — SYNTH cells: median {st.median(synth_nn):.3f}  mean {st.mean(synth_nn):.3f}")
    print(f"  (want these CLOSE — density preserved; far apart => retune --p)")

    out = HERE / "cn1_synth_fingerprints.jsonl"
    with open(out, "w") as fh:
        for s in synth:
            fh.write(json.dumps(s) + "\n")
    print(f"wrote {out.name}")


if __name__ == "__main__":
    main()
