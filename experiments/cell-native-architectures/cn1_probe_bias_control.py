#!/usr/bin/env python3
"""CN-1 probe-bias control (the winner's-curse check): is the 20-probe fingerprint GENUINELY
over-merging, or was the probe-richness drop (0.344->0.245) pure selection regression?

The sweep picked each held-out cell's top-20 neighbours BY the noisy 20-probe estimate, then
re-measured with a less-noisy one — regression to the mean forces a drop even if the 20-probe
battery is perfectly unbiased. So that sweep cannot establish over-merging. This control removes
the selection: FIXED, randomly-drawn same-arity cell pairs (chosen by nothing), agreement under the
20 DEFAULT_PROBES vs an INDEPENDENT rich battery (1980 fresh random probes, disjoint from the 20).
  - if mean(20-probe) >> mean(rich) on random pairs -> the battery genuinely runs high (biased),
    a richer address is worth a retrain;
  - if mean(20-probe) ~= mean(rich) -> unbiased, the earlier drop was selection regression, and the
    probe-richness retrain would be chasing an artifact.
Same instrument shape as the P=0 sensitivity lane and the permutation null.

Run: python3 cn1_probe_bias_control.py
"""
from __future__ import annotations

import json
import random
import statistics as st
from pathlib import Path

import cell80_py

HERE = Path(__file__).resolve().parent
CELLS_DIR = HERE.parent.parent / "cell80" / "cells"
DEFAULT_PROBES = [
    [3, 7, 12], [7, 3, 1], [0, 0, 0], [1, 1, 1], [5, 5, 9], [2, 9, 5], [10, 3, 7],
    [255, 1, 128], [100, 4, 50], [12, 12, 12], [1230, 0, 2], [65531, 3, 6], [5, 2, 9],
    [9, 5, 2], [2, 8, 4], [4, 2, 4], [7, 0, 0], [12, 3, 4], [9000, 2500, 40], [2, 0, 1],
]
N_RICH = 1980  # independent fresh probes, disjoint from DEFAULT_PROBES
N_PAIRS = 3000


def main():
    lib = {json.loads(l)["name"]: json.loads(l) for l in (HERE / "cn1_library.jsonl").read_text().splitlines() if l.strip()}
    arity = {n: r["arity"] for n, r in lib.items()}
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

    rng = random.Random(1)
    def draw():
        r = rng.choice(["small", "mid", "boundary", "wide"])
        f = (lambda: rng.randint(0, 20)) if r == "small" else (lambda: rng.randint(0, 999)) if r == "mid" \
            else (lambda: rng.choice([0, 1, 2, 255, 256, 32767, 65535])) if r == "boundary" else (lambda: rng.randint(0, 65535))
        return [f(), f(), f()]
    coarse = DEFAULT_PROBES
    rich = [draw() for _ in range(N_RICH)]  # independent of DEFAULT_PROBES

    outs_c, outs_r = {}, {}
    def run_battery(n, battery):
        a = arity[n]; vec = []
        for p in battery:
            try:
                res = host.run(handles[n], p[:a]); vec.append(res["result"] if res.get("halt") == "returned" else None)
            except Exception:
                vec.append(None)
        return vec
    for n in val:
        outs_c[n] = run_battery(n, coarse)
        outs_r[n] = run_battery(n, rich)

    def agree(o, a, b):
        va, vb = o[a], o[b]
        return sum(1 for x, y in zip(va, vb) if x == y) / len(va)

    # FIXED random same-arity pairs, chosen by nothing
    by_ar = {}
    for n in val:
        by_ar.setdefault(arity[n], []).append(n)
    pairs = []
    while len(pairs) < N_PAIRS:
        a = rng.choice(list(by_ar))
        if len(by_ar[a]) < 2:
            continue
        x, y = rng.sample(by_ar[a], 2)
        pairs.append((x, y))

    a20 = [agree(outs_c, x, y) for x, y in pairs]
    arich = [agree(outs_r, x, y) for x, y in pairs]
    diffs = [c - r for c, r in zip(a20, arich)]
    print(f"random same-arity pairs: {len(pairs)} | value cells: {len(val)}")
    print(f"  mean agreement  20-probe (DEFAULT_PROBES): {st.mean(a20):.4f}")
    print(f"  mean agreement  rich (1980 independent):   {st.mean(arich):.4f}")
    print(f"  mean(20 - rich): {st.mean(diffs):+.4f}   (>0 => 20-probe runs HIGH = genuine over-merge)")
    print(f"  median(20 - rich): {st.median(diffs):+.4f}")
    verdict = ("GENUINE over-merge: the 20-probe battery runs systematically high -> richer address worth a retrain"
               if st.mean(diffs) > 0.02 else
               "UNBIASED: 20-probe ~= rich on random pairs -> the sweep drop was selection regression; probe-richness retrain would chase an artifact")
    print(f"\n  VERDICT: {verdict}")


if __name__ == "__main__":
    main()
