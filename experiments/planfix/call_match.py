#!/usr/bin/env python3
"""The keystone: the model emits a function CALL by intent (any fuzzy name), and we
SEARCH our library to bind it to the real, already-verified cell, then RUN it.
Reuse over regeneration — the cell80 thesis. This proves fuzzy-call -> real-cell ->
executed-answer end to end, with a confidence margin to gate on."""
import os

LIB = os.environ.get("CELL_LIBRARY", "/Users/christopherhay/chris-source/cell80/cell80/cells")

from cell80_mcp.library import CellLibrary

lib = CellLibrary(LIB)
host = lib.host


import re


def sig_arity(cell_id):
    try:
        m = host.manifest(cell_id)
    except Exception:
        return None
    p = m.get("params")
    if isinstance(p, list):
        return len(p)
    inner = re.search(r"\(([^)]*)\)", m.get("signature", ""))
    if not inner:
        return None
    s = inner.group(1).strip()
    return 0 if not s else s.count(",") + 1


def resolve(intent, nargs):
    hits = host.search_scored(intent, 6)
    ranked = [(m["id"] if isinstance(m, dict) else m, round(score, 3)) for score, m in hits]
    if not ranked:
        return None, ranked
    # prefer the highest-scoring candidate whose ARITY matches the call
    for cid, sc in ranked:
        if sig_arity(cid) == nargs:
            return (cid, sc), ranked
    return ranked[0], ranked


def run_cell(cell_id, args):
    last = None
    for fn in (lambda: lib.run(cell_id, args), lambda: host.run(cell_id, args)):
        try:
            r = fn()
            return r.get("result") if isinstance(r, dict) else r
        except Exception as e:
            last = e
    return f"ERR:{last}"


# (intent the model might emit, args, expected) — intents are deliberately fuzzy /
# misspelled / paraphrased, never the exact cell id.
CASES = [
    ("greatest common divisor", [1071, 462], 21),
    ("hcf", [1071, 462], 21),
    ("max of two numbers", [15, 42], 42),
    ("pick the larger", [15, 42], 42),
    ("is greater than", [7, 3], 1),
    ("absolute difference", [38, 36], 2),
    ("smallest of two", [3, 7], 3),
    ("least common multiple", [4, 6], 12),
]

print(f"library: {LIB}\n")
print(f"{'model intent':26s} {'->resolved cell':>18s} {'margin':>7s} {'args':>10s} {'result':>7s}  ok?")
print("-" * 92)
ok = 0
for intent, args, exp in CASES:
    top, ranked = resolve(intent, len(args))
    if top is None:
        print(f"{intent:26s} {'(no match)':>18s}")
        continue
    top_id, top_score = top
    res = run_cell(top_id, args)
    good = res == exp
    ok += good
    print(f"{intent:26s} {top_id:>18s} {top_score:>7} {str(args):>10s} {str(res):>7s}  "
          f"{'ok' if good else '✗ want '+str(exp)}   text-top={ranked[0][0]} alts={[c for c,_ in ranked[1:3]]}")
print("-" * 92)
print(f"resolved+ran correctly: {ok}/{len(CASES)}")
print("(top-1 by char-3-gram search; 'margin' = top1-top2 cosine, the confidence to gate on)")
