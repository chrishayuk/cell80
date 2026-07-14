#!/usr/bin/env python3
"""CN-6 stage 1 — validate the runtime tier before training the model to emit examples.

The whole two-tier / CN-6 story rests on: the runtime resolves a cell from its I/O examples by
EXECUTION, no token needed (so it works for held-out and post-freeze cells). This checks that
directly: for each HELD-OUT value cell, generate k I/O examples and ask `CellHost.route` (the pure
behavioural router) to resolve them — does the true cell come back rank-1? If this fails on held-out
cells, CN-6's premise is broken and the model-side (emit examples) is moot. If it holds, CN-6
reduces to "can the model emit good examples."

Run: python3 cn6_router_check.py
"""
from __future__ import annotations

import json
import random
import statistics as st
from pathlib import Path

import cell80_py

HERE = Path(__file__).resolve().parent
CELLS_DIR = HERE.parent.parent / "cell80" / "cells"


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
    seen_val = [n for n in value if n not in held]
    print(f"library loaded: {len(value)} value cells | held-out value: {len(held_val)} | seen: {len(seen_val)}")

    rng = random.Random(0)

    def examples_for(name, k, safe=False):
        a = lib[name]["arity"]
        out = []
        tries = 0
        while len(out) < k and tries < k * 10:
            tries += 1
            args = [rng.randint(1, 12) if safe else rng.randint(0, 300) for _ in range(a)]
            r = host.run(handles[name], args)
            if r.get("halt") == "returned":
                out.append((args, r["result"]))  # route wants (inputs, out) tuples
        return out

    def rank_of(name, k):
        ex = examples_for(name, k) or examples_for(name, k, safe=True)
        if not ex:
            return None
        ranked = host.route(ex, limit=len(value))  # ranked list of briefs/ids
        names = [_name(x) for x in ranked]
        return names.index(name) if name in names else len(value)

    for label, cells in [("HELD-OUT", held_val), ("seen (control)", seen_val[:40])]:
        for k in [3, 6]:
            ranks = [r for r in (rank_of(n, k) for n in cells) if r is not None]
            if not ranks:
                continue
            p1 = sum(r == 0 for r in ranks) / len(ranks)
            p5 = sum(r < 5 for r in ranks) / len(ranks)
            ranks.sort()
            print(f"  {label:<16} k={k}  P@1 {p1:.3f}  P@5 {p5:.3f}  median rank {ranks[len(ranks)//2]}  (n={len(ranks)})")


def _name(brief):
    if isinstance(brief, dict):
        return brief.get("id") or brief.get("name")
    if isinstance(brief, (list, tuple)):
        return brief[0]
    return str(brief)


if __name__ == "__main__":
    main()
