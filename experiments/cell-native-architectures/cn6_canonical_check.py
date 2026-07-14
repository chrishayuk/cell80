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

POOLS = {
    "tiny (0..10)":      [0, 1, 2, 3, 4, 5, 10],
    "round (0..100)":    [0, 1, 2, 3, 5, 10, 20, 50, 100],
    "random (0..300)":   None,          # baseline — stage-1 regime
    "wide (0..65535)":   "wide",        # what a discriminating prompt would ask for
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
    held_val = [n for n in value if n in held]
    rng = random.Random(0)

    def draw(pool):
        if pool is None:
            return rng.randint(0, 300)
        if pool == "wide":
            return rng.randint(0, 65535)
        return rng.choice(pool)

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
        hits = uniq = tot = 0
        for n in held_val:
            ex = examples(n, pool)
            if len(ex) < 6:
                continue
            tot += 1
            ranked = host.route(ex, limit=len(value))
            names = [r.get("id") if isinstance(r, dict) else r for r in ranked]
            hits += (names[0] == n) if names else 0
            # how many cells match the examples as well as #1? (a proxy for non-discrimination:
            # canonical examples that many cells satisfy => ties at the top)
            if names:
                uniq += (names.count(names[0]) if False else 1)  # placeholder; ties measured below
        return hits / tot, tot

    def top_ties(pool, k=6, probe=8):
        """mean number of DISTINCT cells that reproduce the true cell's outputs on the example inputs
        (>1 => the examples don't uniquely identify the cell — non-discriminating)."""
        vals = []
        for n in held_val:
            ex = examples(n, pool, k)
            if len(ex) < k:
                continue
            matches = 0
            for c in value:
                ok = True
                for args, out in ex:
                    r = host.run(handles[c], list(args))
                    if not (r.get("halt") == "returned" and r["result"] == out):
                        ok = False; break
                matches += ok
            vals.append(matches)
        return sum(vals) / len(vals) if vals else 0.0

    print(f"held-out value cells: {len(held_val)} | library {len(value)}\n")
    print(f"{'input pool':<20}{'router P@1':>12}{'mean #cells matching all 6':>30}")
    for label, pool in POOLS.items():
        acc, tot = p1(pool)
        ties = top_ties(pool)
        print(f"{label:<20}{acc:>12.3f}{ties:>30.2f}")
    print("\nreading: if router P@1 collapses on tiny/round vs random/wide, canonical examples are")
    print("non-discriminating (many cells match them) => GENERATION is broken independent of the model.")


if __name__ == "__main__":
    main()
