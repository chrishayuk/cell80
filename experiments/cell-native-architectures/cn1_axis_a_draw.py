#!/usr/bin/env python3
"""CN-1 real build, step 3 (`cell-native-architectures-cn1-preregistration.md`): draw and
record the **axis-A held-out cells** — the ~10% of the cell vocabulary that gate (ii)
depends on (cells that never appear as a call target anywhere in training, so their only
possible address is the one W_f derives from their fingerprint).

This runs, and its output is committed, BEFORE any corpus is generated or any arm is
trained — that ordering is the whole point of pre-registration: the held-out set cannot have
been chosen to flatter a result it hasn't seen. The draw is fully deterministic (fixed seed,
cells sorted by name within each pack), so anyone can reproduce the exact set from
`cn1_library.jsonl` + this script.

Stratification (pre-registered): by pack, "so no family is entirely held out". Per pack we
hold out `round(0.10 * size)` cells, clamped to at most `size - 1` (never the whole pack)
and to 0 for packs too small to give a nonzero round (size < 5) — those small packs
contribute their cells only as *seen* siblings, which is exactly what a held-out cell in a
larger pack needs to have any address to generalize from.

Each held-out cell is tagged with its arity. Gate (ii) is only *testable* on a held-out
cell the corpus would otherwise invoke; the arithmetic-shaped invocation corpus exercises
value cells (arity >= 1), not state cells (arity 0, driven by named state fields). Recording
arity now lets the eval report gate (ii) on the value-cell subset — where it is genuinely
testable — without a re-draw, while still honoring the frozen "10% of the vocabulary" text
(the draw is over all 790 cells; a held-out state cell simply yields an empty eval bucket,
which is honest, not a bug).

Run: python3 cn1_axis_a_draw.py
"""
from __future__ import annotations

import json
import random
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
LIBRARY = HERE / "cn1_library.jsonl"
OUT = HERE / "cn1_axis_a_heldout.json"

SEED = 80  # fixed, recorded; "cell80"
HOLDOUT_FRACTION = 0.10


def held_count(pack_size: int) -> int:
    """round(0.10 * size), clamped to [0, size-1] so a pack is never wholly held out."""
    n = round(HOLDOUT_FRACTION * pack_size)
    return max(0, min(n, pack_size - 1))


def main() -> None:
    rows = [json.loads(line) for line in LIBRARY.read_text().splitlines() if line.strip()]
    by_pack: dict[str, list[dict]] = defaultdict(list)
    for r in rows:
        by_pack[r["pack"]].append(r)

    rng = random.Random(SEED)
    held: list[dict] = []
    per_pack = {}
    # Iterate packs in sorted order, and cells sorted by name within each pack, so the draw
    # depends only on (seed, library contents), not on dict/file iteration order.
    for pack in sorted(by_pack):
        cells = sorted(by_pack[pack], key=lambda r: r["name"])
        k = held_count(len(cells))
        picks = rng.sample(cells, k) if k else []
        picks = sorted(picks, key=lambda r: r["name"])
        per_pack[pack] = {"size": len(cells), "held": k}
        for r in picks:
            held.append(
                {
                    "name": r["name"],
                    "pack": r["pack"],
                    "family_hash": r["family_hash"],
                    "arity": r["arity"],
                }
            )

    held_by_arity = defaultdict(int)
    for h in held:
        held_by_arity[h["arity"]] += 1

    n_value_cells = sum(1 for r in rows if r["arity"] >= 1)
    n_held_value = sum(1 for h in held if h["arity"] >= 1)

    out = {
        "_provenance": {
            "script": "cn1_axis_a_draw.py",
            "library": "cn1_library.jsonl",
            "seed": SEED,
            "holdout_fraction": HOLDOUT_FRACTION,
            "rule": "per pack: round(0.10*size) clamped to [0, size-1]; cells sorted by name; random.Random(seed).sample",
            "note": "Drawn and committed BEFORE corpus generation and training (pre-registration step 3).",
        },
        "n_library": len(rows),
        "n_held": len(held),
        "held_fraction_overall": round(len(held) / len(rows), 4),
        "n_value_cells": n_value_cells,
        "n_held_value": n_held_value,
        "held_value_fraction": round(n_held_value / n_value_cells, 4) if n_value_cells else None,
        "held_by_arity": {str(a): held_by_arity[a] for a in sorted(held_by_arity)},
        "per_pack": per_pack,
        "held_out_cells": sorted(held, key=lambda h: (h["pack"], h["name"])),
    }
    OUT.write_text(json.dumps(out, indent=2))

    print(f"library: {len(rows)} cells across {len(by_pack)} packs")
    print(
        f"axis-A held out: {len(held)} cells "
        f"({out['held_fraction_overall']*100:.1f}% overall; "
        f"{n_held_value}/{n_value_cells} = {(out['held_value_fraction'] or 0)*100:.1f}% of value cells)"
    )
    print(f"held by arity: {out['held_by_arity']}")
    packs_fully_seen = [p for p, v in per_pack.items() if v["held"] == 0]
    print(f"packs contributing 0 held-out (all-seen siblings): {len(packs_fully_seen)}")
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
