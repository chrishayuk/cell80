#!/usr/bin/env python3
"""CN-6 stage 1c — discriminativeness of CANONICAL examples (model-free, decides the fork).

Correctness isn't enough; examples must DISCRIMINATE. Stages 1/1b used RANDOM oracle inputs. A model
demonstrating a pattern emits SMALL, CANONICAL inputs — 0,1,2,10,100 — precisely where many cells
agree (discount(100,10)->90 is satisfied by half the finance pack). This tests whether canonical
examples resolve the cell AT ALL, with oracle-correct outputs (so any collapse is discriminativeness,
not error rate). If canonical P@1 holds -> generation is viable. If it collapses -> generation is
broken independent of the model, and the fix belongs in the design (ask the model for a varied/large
example, or have the router request a discriminating probe back).

Run: python3 cn6_canonical_check.py
"""
from __future__ import annotations

import json
import random
from pathlib import Path

import cell80_py

HERE = Path(__file__).resolve().parent
CELLS_DIR = HERE.parent.parent / "cell80" / "cells"

import math

POOLS = {
    "tiny (0..10)":      (0, 10),
    "round (0..100)":    (0, 100),
    "mid (0..1000)":     (0, 1000),
    "wide (0..65535)":   (0, 65535),
}


def main():
    lib = {json.loads(l)["name"]: json.loads(l) for l in (HERE / "cn1_library.jsonl").read_text().splitlines() if l.strip()}
    held = {h["name"] for h in json.loads((HERE / "cn1_axis_a_heldout.json").read_text())["held_out_cells"]}
    value = [n for n, r in lib.items() if r["arity"] >= 1]
    host = cell80_py.CellHost()
    handles = {}
    for n in value:
        try:
            host.add_source(n, next(CELLS_DIR.rglob(f"{n}.rs")).read_text())
            handles[n] = host.load(n)
        except Exception:
            pass
    value = [n for n in value if n in handles]
    rng = random.Random(0)

    def draw(pool):
        return rng.randint(pool[0], pool[1])

    def examples(name, pool, k=6):
        a = lib[name]["arity"]
        out, tries = [], 0
        while len(out) < k and tries < k * 15:
            tries += 1
            args = [draw(pool) for _ in range(a)]
            r = host.run(handles[name], args)
            if r.get("halt") == "returned":
                out.append((args, r["result"]))
        return out

    def p1(pool):
        # LEAVE-ONE-OUT over ALL value cells (n~249), not just the 24 held-out — router resolution
        # is training-independent, so discriminativeness is a library property; testing on all of
        # them collapses the error bars (n=24 gave SE~0.10, useless for a ~0.1 effect).
        hits = tot = 0
        for n in value:
            ex = examples(n, pool)
            if len(ex) < 6:
                continue
            tot += 1
            ranked = host.route(ex, limit=len(value))
            names = [r.get("id") if isinstance(r, dict) else r for r in ranked]
            hits += (names[0] == n) if names else 0
        p = hits / tot
        se = math.sqrt(p * (1 - p) / tot)
        return p, se, tot

    print(f"value cells (leave-one-out): {len(value)} | library {len(value)}\n")
    print(f"{'input pool':<20}{'router P@1':>12}{'±SE':>8}{'n':>6}")
    res = {}
    for label, pool in POOLS.items():
        p, se, tot = p1(pool)
        res[label] = (p, se)
        print(f"{label:<20}{p:>12.3f}{se:>8.3f}{tot:>6}")
    # is the width effect significant? wide vs round, difference / SE-of-difference
    (pw, sw), (pr, sr) = res["wide (0..65535)"], res["round (0..100)"]
    d = pw - pr; sd = math.sqrt(sw * sw + sr * sr)
    print(f"\nwide − round = {d:+.3f}  (SE-of-diff {sd:.3f}, z = {d/sd:+.1f})")
    print(f"  monotonic in width? tiny {res['tiny (0..10)'][0]:.3f} <= round {res['round (0..100)'][0]:.3f}"
          f" <= mid {res['mid (0..1000)'][0]:.3f} <= wide {res['wide (0..65535)'][0]:.3f}")
    print("reading: |z|>2 and monotone => width effect is real; else it was n=24 noise.")


if __name__ == "__main__":
    main()
