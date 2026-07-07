#!/usr/bin/env python3
"""Generate docs/cell-index.md from the actual cell library — the ground truth for what's
landed, so the index can't drift the way hand-maintained cell-count prose has before.

Usage: cargo run -q -p cell80 --bin cell80 -- index cell80/cells --json | python3 cell80/scripts/gen_cell_index.py > docs/cell-index.md
"""
import json
import sys

# Pack assignment used to be a hand-maintained dict here (every new cell had to be added by
# hand, or the script failed loudly). Cells now live in pack subdirectories
# (`cell80/cells/<pack>/<id>.rs`, `docs/library-growth.md`), and `cell80 index --json` reports
# each cell's `pack` (its parent directory name) directly — so the directory *is* the pack now,
# and grouping just reads that field. Nothing here needs updating when a cell lands.

# Aliases removed by the Phase 2.2 admission gate (behaviourally identical to a landed cell;
# not separate code — see docs/library-growth.md). Listed so a reader searching for the old
# name still finds where it went.
ALIASES = {
    "argmin2": "is_gt",
    "argmax2": "is_lt",
    "quantize": "safe_div",
    "wrap": "safe_mod",
}


def main():
    data = json.load(sys.stdin)
    cells = data["cells"]

    packs = {}
    for c in cells:
        packs.setdefault(c["pack"], []).append(c)
    for pack_cells in packs.values():
        pack_cells.sort(key=lambda c: c["id"])

    print("# Cell index — every landed cell, by pack")
    print()
    print(f"*Generated from `{data['dir']}` ({len(cells)} cells) by "
          "`cell80/scripts/gen_cell_index.py`. Regenerate after any cell is added/removed:*")
    print()
    print("```")
    print("cargo run -q -p cell80 --bin cell80 -- index cell80/cells --json \\")
    print("  | python3 cell80/scripts/gen_cell_index.py > docs/cell-index.md")
    print("```")
    print()
    print("See `docs/library-growth.md` for the packs' purpose, the contribution rule, and "
          "the admission gate that enforces \"no behavioural duplicates.\"")
    print()

    for pack in sorted(packs):
        ids = packs[pack]
        print(f"## {pack} ({len(ids)})")
        print()
        print("| id | signature | summary |")
        print("|---|---|---|")
        for c in ids:
            summary = c["summary"].replace("|", "\\|")
            print(f"| `{c['id']}` | `{c['signature']}` | {summary} |")
        print()

    print(f"## aliases ({len(ALIASES)})")
    print()
    print("Behaviourally identical to a landed cell (found by the Phase 2.2 admission gate); "
          "removed as separate code, vocabulary merged into the surviving cell's tags.")
    print()
    print("| old id | → | landed as |")
    print("|---|---|---|")
    for old, new in ALIASES.items():
        print(f"| `{old}` | → | `{new}` |")
    print()

    print("## planned (not yet landed)")
    print()
    print("See `docs/library-growth.md` \"Next waves\" for the prioritized list "
          "(stateful/RNG, signed-deltas, and scoring/choice's second slice above are landed "
          "— bounded_rand and time/budget's five named candidates were all found to be "
          "exact duplicates of existing cells, not built; score_2factor's vocabulary was "
          "merged into weighted_sum2's tags rather than shipping a duplicate; "
          "cosine_score_approx still ahead), the Phase 2.3 "
          "pilot-batch section for the author->verify->admit loop, and "
          "`docs/math-campaign-spec.md` for the "
          "GSM8K math campaign (M1 complete: checked-arithmetic, money-bps, units, "
          "verifier-ranker, and fractions above are all five authored packs — M0 landed "
          "Tier 2, one u32 param per call, so fractions inlines its own GCD-reduction loop "
          "per cell rather than sharing a two-u32-param gcd_u32 helper; M2-M4 remain gated "
          "behind cell_solve). All five originally-planned "
          "wave-3 packs plus the Phase 2.3 pilot batch (packing/BCD, vector) landed a first slice "
          "above. `unpack_lo`/`unpack_hi` were never built — checking docs/cell-index.md "
          "before authoring found they'd be exact duplicates of `low_byte`/`high_byte`. "
          "Each first slice deferred its harder items: ISBN/IBAN/UPC checksums need a "
          "wider-than-u32 input (see library-growth.md); q_sqrt/piecewise sigmoid-tanh, "
          "rate_window_update, a fixed-point running variance (Welford), Morton "
          "encode/decode (needs a u32 state field, not yet risked), a Bresenham stepper, "
          "and cosine_score_approx (deferred: exact fixed-point cosine needs a wide "
          "sqrt-of-a-product without overflow, not yet worked out) are all still open.")


if __name__ == "__main__":
    main()
