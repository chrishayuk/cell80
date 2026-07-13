#!/usr/bin/env python3
"""CN-1 probe-richness sweep (model-free): is the 20-probe fingerprint too COARSE to separate the
cells the model confuses (~0.44 agreement), or are those cells genuinely that similar?

The address is `W_f(fingerprint)` and the fingerprint is 20 DEFAULT_PROBES. If two cells agree on
~0.44 of 20 probes but are actually behaviourally distinct, a richer battery would push their
agreement down — meaning the *fingerprint*, not the model, is the resolution bottleneck, and a
better address could sharpen (top-1 back in play). If agreement stays ~0.44 under 20× more probes,
the cells are genuinely moderately-similar and the fingerprint isn't the limiter.

Model-free: execute cells on batteries of increasing size (20 → 100 → 500 → 2000 diverse inputs)
and recompute pairwise agreement. For each held-out value cell, look at its top-20 nearest cells by
the 20-probe fingerprint (its coarse "confusion neighbourhood") and track their agreement as the
battery grows. Separation = agreement falls; the gap between 20-probe and rich-probe agreement is
how much resolution the coarse fingerprint threw away.

Run: python3 cn1_probe_richness.py
"""
from __future__ import annotations

import json
import random
import statistics as st
from pathlib import Path

import cell80_py

HERE = Path(__file__).resolve().parent
CELLS_DIR = HERE.parent.parent / "cell80" / "cells"

# the 20 DEFAULT_PROBES (cell80/src/fingerprint.rs), the current fingerprint battery
DEFAULT_PROBES = [
    [3, 7, 12], [7, 3, 1], [0, 0, 0], [1, 1, 1], [5, 5, 9], [2, 9, 5], [10, 3, 7],
    [255, 1, 128], [100, 4, 50], [12, 12, 12], [1230, 0, 2], [65531, 3, 6], [5, 2, 9],
    [9, 5, 2], [2, 8, 4], [4, 2, 4], [7, 0, 0], [12, 3, 4], [9000, 2500, 40], [2, 0, 1],
]


def main():
    lib = {json.loads(l)["name"]: json.loads(l) for l in (HERE / "cn1_library.jsonl").read_text().splitlines() if l.strip()}
    arity = {n: r["arity"] for n, r in lib.items()}
    held = {h["name"] for h in json.loads((HERE / "cn1_axis_a_heldout.json").read_text())["held_out_cells"]}
    val = [n for n, a in arity.items() if a >= 1]

    host = cell80_py.CellHost()
    handles = {}
    for n in val:
        try:
            host.add_source(n, next(CELLS_DIR.rglob(f"{n}.rs")).read_text())
            handles[n] = host.load(n)
        except Exception:
            pass
    val = [n for n in val if n in handles]

    rng = random.Random(0)
    # richer batteries: diverse 3-tuples spanning small/mid/boundary/wide, each cell uses its arity slice
    def battery(n):
        b = list(DEFAULT_PROBES)
        while len(b) < n:
            regime = rng.choice(["small", "mid", "boundary", "wide"])
            def one():
                if regime == "small": return rng.randint(0, 20)
                if regime == "mid": return rng.randint(0, 999)
                if regime == "boundary": return rng.choice([0, 1, 2, 255, 256, 32767, 65535])
                return rng.randint(0, 65535)
            b.append([one(), one(), one()])
        return b[:n]

    SIZES = [20, 100, 500, 2000]
    big = battery(SIZES[-1])

    # cache each cell's output vector on the big battery (None where it doesn't return cleanly)
    outs = {}
    for n in val:
        a = arity[n]
        vec = []
        for probe in big:
            try:
                r = host.run(handles[n], probe[:a])
                vec.append(r["result"] if r.get("halt") == "returned" else None)
            except Exception:
                vec.append(None)
        outs[n] = vec

    def agree(a, b, k):
        va, vb = outs[a][:k], outs[b][:k]
        return sum(1 for x, y in zip(va, vb) if x == y) / k

    heldval = [n for n in val if n in held]
    # aggregate: mean agreement of each held-out cell's coarse top-20 neighbourhood, per battery size
    print(f"held-out value cells: {len(heldval)} | value cells executed: {len(val)}")
    print(f"{'battery':>8} {'mean nbr agreement':>20} {'nbrs still >=0.7':>18}")
    for k in SIZES:
        per_cell_mean = []
        frac_high = []
        for T in heldval:
            near = sorted((n for n in val if n != T and arity[n] == arity[T]), key=lambda c: -agree(T, c, 20))[:20]
            if not near:
                continue
            ags = [agree(T, c, k) for c in near]
            per_cell_mean.append(st.mean(ags))
            frac_high.append(sum(a >= 0.7 for a in ags) / len(ags))
        print(f"{k:>8} {st.mean(per_cell_mean):>20.3f} {st.mean(frac_high):>18.3f}")
    print("\nreading: if mean agreement of the coarse top-20 neighbourhood FALLS as the battery grows,")
    print("the 20-probe fingerprint was over-merging distinct cells -> a richer address could sharpen.")


if __name__ == "__main__":
    main()
