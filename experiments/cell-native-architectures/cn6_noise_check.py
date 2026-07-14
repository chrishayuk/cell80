#!/usr/bin/env python3
"""CN-6 stage 1b — noise sensitivity of the router (model-free, decisive before any training spend).

Stage 1 used ORACLE examples (ran the true cell) — that is a CEILING, not a floor. A deployed model
emits imperfect examples, and if the router behavioural-matches, ONE wrong output could EXCLUDE the
true cell. This measures exactly that: take clean 6-example sets for held-out cells, corrupt j of
them (j=1,2) three ways, and watch P@1. Graceful degradation => the router already tolerates model
error. A cliff => CN-6's router needs majority/confidence-weighted matching BEFORE we train a model
against it.

Corruption modes: random (wrong u16), off_by_one (±1 — the classic arithmetic slip), sibling (the
output of a different same-arity cell — a plausible-but-wrong answer, the realistic model failure).

Run: python3 cn6_noise_check.py
"""
from __future__ import annotations

import json
import random
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
    by_ar = {}
    for n in value:
        by_ar.setdefault(lib[n]["arity"], []).append(n)
    rng = random.Random(0)

    def clean_examples(name, k=6):
        a = lib[name]["arity"]
        out, tries = [], 0
        while len(out) < k and tries < k * 12:
            tries += 1
            args = [rng.randint(0, 300) for _ in range(a)]
            r = host.run(handles[name], args)
            if r.get("halt") == "returned":
                out.append((args, r["result"]))
        return out

    def corrupt(name, ex, j, mode):
        a = lib[name]["arity"]
        ex = [list(e) for e in ex]
        idxs = rng.sample(range(len(ex)), j)
        for i in idxs:
            args, out = ex[i]
            if mode == "random":
                bad = rng.randint(0, 65535)
            elif mode == "off_by_one":
                bad = (out + rng.choice([-1, 1])) & 0xFFFF
            else:  # sibling
                sib = rng.choice([c for c in by_ar[a] if c != name])
                r = host.run(handles[sib], args)
                bad = r["result"] if r.get("halt") == "returned" else (out + 1) & 0xFFFF
            ex[i] = (args, bad)
        return [tuple(e) for e in ex]

    def p1_at(cells, transform):
        hits = 0
        for n in cells:
            ex = clean_examples(n)
            if len(ex) < 6:
                continue
            ex = transform(n, ex)
            ranked = host.route(ex, limit=len(value))
            names = [r.get("id") if isinstance(r, dict) else r for r in ranked]
            hits += (names[0] == n) if names else 0
        return hits / len(cells)

    print(f"held-out value cells: {len(held_val)} | library {len(value)}\n")
    print(f"{'corruption':<14}{'clean':>8}{'1 of 6':>9}{'2 of 6':>9}")
    clean = p1_at(held_val, lambda n, e: e)
    for mode in ["random", "off_by_one", "sibling"]:
        p1 = p1_at(held_val, lambda n, e, m=mode: corrupt(n, e, 1, m))
        p2 = p1_at(held_val, lambda n, e, m=mode: corrupt(n, e, 2, m))
        print(f"{mode:<14}{clean:>8.3f}{p1:>9.3f}{p2:>9.3f}")
    print("\nreading: graceful fall => router tolerates model error; cliff to ~0 => needs majority/"
          "confidence matching in the router BEFORE training a model against it.")


if __name__ == "__main__":
    main()
