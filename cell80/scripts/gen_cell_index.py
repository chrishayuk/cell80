#!/usr/bin/env python3
"""Generate docs/cell-index.md from the actual cell library — the ground truth for what's
landed, so the index can't drift the way hand-maintained cell-count prose has before.

Usage: cargo run -q -p cell80 --bin cell80 -- index cell80/cells --json | python3 cell80/scripts/gen_cell_index.py > docs/cell-index.md
"""
import json
import sys

# Pack assignment: a pack is a tag, not a directory (docs/library-growth.md), so this is a
# curated grouping for readability, not a field on the cell itself. Every landed cell id must
# appear in exactly one pack below — the script asserts that, so a new cell that isn't added
# here fails loudly instead of silently missing from the index.
PACKS = {
    "predicates": ["eq", "neq", "is_lt", "is_le", "is_gt", "is_ge", "is_zero", "nonzero", "is_even", "is_odd"],
    "safe-arith": ["add_sat", "sub_sat", "mul_sat", "safe_div", "safe_mod", "ceil_div", "avg2", "square", "square_wide"],
    "bounds": ["between_exclusive", "normalize_0_100", "snap_down", "snap_up", "round_to_multiple", "clamp"],
    "validation": ["range_check"],
    "percent": ["percent", "permille", "ratio_255", "scale_percent", "increase_percent", "discount_percent", "within_percent"],
    "ranking-stats": ["min", "max", "min3", "max3", "median3", "argmax3", "argmin3", "sum3", "mean3", "range3", "mode3", "majority3", "midrange3"],
    "bit/mask": ["popcount", "parity", "bit_is_set", "set_bit", "clear_bit", "toggle_bit", "mask_has_all", "mask_has_any", "mask_union", "mask_intersection", "mask_xor"],
    "number-theory": ["lcm", "gcd", "gcd3", "divides", "is_coprime", "is_prime", "is_square", "isqrt", "digit_sum", "num_digits", "factor_count", "triangular", "next_pow2", "is_pow2", "pow_small", "cube_sat", "pow_mod"],
    "distance": ["abs_diff", "manhattan", "chebyshev", "euclid_sq"],
    "bit-encoding": ["low_byte", "high_byte", "swap_bytes", "rotl16", "rotr16", "reverse_bits", "leading_zeros", "trailing_zeros", "bit_length"],
    "hashing": ["hash_pair", "fnv1a_step", "crc8_step", "mix16"],
    "bucket/convert": ["bucket3", "percent_to_byte", "byte_to_percent"],
    "scoring/choice": ["weighted_sum", "weighted_sum_wide"],
    "calendrical/checksum": ["is_leap_year", "days_in_month", "day_of_week", "luhn_check"],
    "fixed-point": ["q_mul", "q_div", "q_lerp"],
    "agentic-runtime": ["token_bucket_step", "backoff_next", "circuit_breaker_step", "debounce_step", "hysteresis"],
    "running-stats": ["running_min_max_step", "streak_step", "accumulate_step"],
    "spatial/grid": ["grid_index", "point_in_rect", "aabb_intersect"],
    "packing/BCD": ["pack_u8", "pack_nibbles", "bcd_encode", "bcd_decode"],
    "vector": ["dot2", "norm2_sq"],
    "checked-arithmetic": ["mul_u16_u16_to_u32", "add_checked_u32", "sub_checked_u32", "div_exact_u32", "div_floor_u32", "div_ceil_u32", "mod_u32", "fits_u16"],
}

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
    cells = {c["id"]: c for c in data["cells"]}

    assigned = set()
    for pack_cells in PACKS.values():
        assigned.update(pack_cells)
    missing = set(cells) - assigned
    extra = assigned - set(cells)
    if missing:
        raise SystemExit(f"cells landed but not in any PACK — add them: {sorted(missing)}")
    if extra:
        raise SystemExit(f"PACK lists a cell that no longer exists — remove it: {sorted(extra)}")

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

    for pack, ids in PACKS.items():
        print(f"## {pack} ({len(ids)})")
        print()
        print("| id | signature | summary |")
        print("|---|---|---|")
        for cid in ids:
            c = cells[cid]
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
          "(stateful/RNG, time/budget, signed deltas), the Phase 2.3 pilot-batch section "
          "for the author->verify->admit loop, and `docs/math-campaign-spec.md` for the "
          "GSM8K math campaign (M1: checked-arithmetic above is the first authored pack; "
          "fractions/money-bps/units/verifier-ranker still ahead, gated on M0's "
          "u32-across-a-call-boundary compiler feature, confirmed still unbuilt this "
          "session even after Cond32 landed). All five originally-planned wave-3 packs "
          "plus the Phase 2.3 pilot batch (packing/BCD, vector) landed a first slice "
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
