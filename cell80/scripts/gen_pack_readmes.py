#!/usr/bin/env python3
"""Generate cell80/cells/<pack>/README.md for every pack — landed cells (mechanical, from
`cell80 index --json`), open roadmap items (curated below, from docs/library-growth.md),
and a math-server coverage section for the packs docs/math-server-map.md's 642-function
mining actually touches (mechanical, from cell80/data/math_server_catalog_map.json).

Usage: cargo run -q -p cell80 --bin cell80 -- index cell80/cells --json \
         | python3 cell80/scripts/gen_pack_readmes.py
"""
import json
import os
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CATALOG_PATH = os.path.join(REPO_ROOT, "cell80", "data", "math_server_catalog_map.json")

# Curated, one entry per pack with real open items outstanding (docs/library-growth.md's
# "Next waves" section and per-pack notes) — omitted entirely for packs with nothing open.
ROADMAP_NOTES = {
    "scoring-choice": (
        "`choose_best4` (the straightforward 4-candidate generalization, once actually "
        "needed) and `tie_break_*` (under-specified — every existing ranking cell already "
        "bakes in its own concrete tie-break rule) are still open — see "
        "`docs/library-growth.md` \"Next waves\"."
    ),
    "calendrical-checksum": (
        "ISBN-10/13, IBAN mod-97, and UPC checksums are deferred: a free-fn cell's calling "
        "convention is 16-bit registers, so a real multi-digit checksum needs a state-cell "
        "version (digits carried as array/state fields) or wider host-side preprocessing — "
        "not yet worth the design cost. `luhn_check` itself stays scoped to a `u16` input "
        "(<=5 decimal digits)."
    ),
    "agentic-runtime": (
        "`ucb1_score_q8` was not attempted: UCB1's score needs a fixed-point `ln`, a "
        "primitive the dialect doesn't have — parked behind the same open question as "
        "`vector`'s `cosine_score_approx`."
    ),
    "running-stats": (
        "Percentile-from-histogram is open, gated on the array-state-field question no "
        "landed cell has needed to answer yet. The math-server mining separately flags a "
        "**sliding-window** family (`simple_moving_average`, `weighted_moving_average`, "
        "`rolling_variance`, `rolling_std`) as candidates distinct from the already-shipped "
        "`running_variance_step` (cumulative over the whole stream, not windowed) — same "
        "array-state-field gate."
    ),
    "vector": (
        "`cosine_score_approx` is blocked on an overflow-safe fixed-point square root of a "
        "product — not yet worked out."
    ),
    "verifier-ranker": (
        "`answer_in_options` (checking an answer against an arbitrary-length option list) "
        "is deferred — GSM8K is free-response, not multiple-choice, so the motivation is "
        "thin, and a real implementation would need an array state field."
    ),
    "signed-deltas": (
        "`lerp_i16` (interpolating between two signed values) is deferred — signed "
        "multiply/divide's rounding direction and overflow safety haven't been worked out."
    ),
    "units": (
        "The dimension-code dispatch table is a fixed small enum with hand-written pairwise "
        "rules, not a general symbolic exponent-vector algebra — extended on demand (most "
        "recently for wage-rate and production-rate word problems), not built out fully "
        "ahead of need."
    ),
}

# Which math-server candidate (docs/math-server-map.md, cell80/data/math_server_catalog_map.json)
# lands closest to which existing cell80 pack — curated once, by hand, since the catalogue's
# own namespace/category grouping doesn't map 1:1 to cell80's packs. A category not listed
# here has no cell80-pack home and is omitted from every README.
CATEGORY_TO_PACK = {
    ("arithmetic", "basic_sequences"): "sequences",
    ("arithmetic", None): "sequences",  # series_sum, negate -> series_sum fits here; negate below overrides
    ("arithmetic", "advanced_primality"): "number-theory",
    ("arithmetic", "arithmetic_functions"): "number-theory",
    ("arithmetic", "combinatorial_numbers"): "combinatorics",
    ("arithmetic", "digital_operations"): "number-theory",
    ("arithmetic", "diophantine_equations"): "number-theory",
    ("arithmetic", "number_theory"): "number-theory",
    ("arithmetic", "harmonic_series"): "fractions",
    ("arithmetic", "farey_sequences"): "fractions",
    ("arithmetic", "figurate_numbers"): "number-theory",
    ("arithmetic", "iterative_sequences"): "sequences",
    ("arithmetic", "modular_arithmetic"): "number-theory",
    ("arithmetic", "additive_number_theory"): "number-theory",
    ("arithmetic", "recursive_sequences"): "sequences",
    ("arithmetic", "special_numbers"): "number-theory",
    ("arithmetic", "primality_tests"): "number-theory",
    ("geometry", "geometry.distances"): "geometry",
    ("geometry", "geometry.intersections"): "geometry",
    ("geometry", "geometry.shapes"): "geometry",
    ("linear_algebra.vectors", "linear_algebra.vectors.geometric"): "vector",
    ("linear_algebra.vectors", "linear_algebra.vectors.operations"): "vector",
    ("statistics", "statistics.descriptive"): "running-stats",
    ("statistics", "statistics.inference"): "running-stats",
    ("linear_algebra.matrices", None): "vector",
    ("numerical", "series"): "sequences",
    ("timeseries", "analysis"): "running-stats",
}
# Per-name overrides where the category-level default isn't quite right.
NAME_OVERRIDES = {
    "negate": "signed-deltas",
}


def load_catalog_candidates():
    if not os.path.exists(CATALOG_PATH):
        return {}
    data = json.load(open(CATALOG_PATH))
    by_pack = {}
    for e in data["entries"]:
        if e["status"] != "candidate":
            continue
        pack = NAME_OVERRIDES.get(e["name"]) or CATEGORY_TO_PACK.get((e["namespace"], e.get("category")))
        if pack:
            by_pack.setdefault(pack, []).append(e)
    return by_pack


def render_pack_readme(pack, cells, candidates):
    cells = sorted(cells, key=lambda c: c["id"])
    lines = []
    lines.append(f"# {pack} — cell pack")
    lines.append("")
    lines.append(
        f"*Generated by `cell80/scripts/gen_pack_readmes.py` from the live library "
        f"(`cell80 index --json`) plus `docs/math-server-map.md`'s coverage map. "
        f"Regenerate after any cell lands or moves:*"
    )
    lines.append("")
    lines.append("```")
    lines.append("cargo run -q -p cell80 --bin cell80 -- index cell80/cells --json \\")
    lines.append("  | python3 cell80/scripts/gen_pack_readmes.py")
    lines.append("```")
    lines.append("")
    lines.append(f"## Landed ({len(cells)})")
    lines.append("")
    lines.append("| id | signature | summary |")
    lines.append("|---|---|---|")
    for c in cells:
        summary = c["summary"].replace("|", "\\|")
        lines.append(f"| `{c['id']}` | `{c['signature']}` | {summary} |")
    lines.append("")

    note = ROADMAP_NOTES.get(pack)
    if note:
        lines.append("## Roadmap — open items")
        lines.append("")
        lines.append(note)
        lines.append("")

    if candidates:
        by_status_note = (
            "Genuinely new, bounded candidates from mining `chuk-mcp-math-server`'s 642 "
            "functions (`docs/math-server-map.md`) that land closest to this pack — **not "
            "yet built**, and not authored until re-checked against the live library (a "
            "candidate recorded in the map may since be covered)."
        )
        lines.append(f"## Math-server coverage — {len(candidates)} candidate(s) not yet built")
        lines.append("")
        lines.append(by_status_note)
        lines.append("")
        lines.append("| name | reason |")
        lines.append("|---|---|")
        for e in sorted(candidates, key=lambda c: c["name"]):
            reason = e["reason"].replace("|", "\\|")
            lines.append(f"| `{e['name']}` | {reason} |")
        lines.append("")

    return "\n".join(lines)


def main():
    data = json.load(sys.stdin)
    cells = data["cells"]
    packs = {}
    for c in cells:
        packs.setdefault(c["pack"], []).append(c)
    live_ids = {c["id"] for c in cells}

    candidates_by_pack = load_catalog_candidates()
    # The catalogue JSON is a point-in-time mining snapshot (docs/math-server-map.md) — a
    # candidate it names may have landed as a real cell since. Drop any whose name is now a
    # live cell id, so a README never claims "not yet built" for something that already is.
    for pack in candidates_by_pack:
        candidates_by_pack[pack] = [e for e in candidates_by_pack[pack] if e["name"] not in live_ids]

    cells_root = os.path.join(REPO_ROOT, "cell80", "cells")
    written = []
    for pack in sorted(packs):
        readme = render_pack_readme(pack, packs[pack], candidates_by_pack.get(pack, []))
        path = os.path.join(cells_root, pack, "README.md")
        with open(path, "w") as f:
            f.write(readme + "\n")
        written.append(path)

    print(f"wrote {len(written)} pack READMEs", file=sys.stderr)


if __name__ == "__main__":
    main()
