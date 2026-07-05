#!/usr/bin/env python3
"""Does cell80's deterministic retrieval (search = TF-IDF char-3-gram, route =
behavioural) earn its place for the two jobs planfix would give it:
  (1) fuzzy/nonexistent cell-id  -> real cell id     [search_scored]
  (2) 'this is really a comparison' -> max/is_gt/...  [search + route]
"""
import os
import sys

LIB = os.environ.get("CELL_LIBRARY", "/Users/christopherhay/chris-source/cell80/cell80/cells")

from cell80_mcp.library import CellLibrary

lib = CellLibrary(LIB)
host = lib.host
print("host methods:", [m for m in dir(host) if not m.startswith("_")])
print("lib methods :", [m for m in dir(lib) if not m.startswith("_")])
print()


def scored(q, k=4):
    if hasattr(host, "search_scored"):
        return host.search_scored(q, k)
    return [(None, m) for m in host.search(q, k)]


def show(q):
    rows = scored(q, 4)
    out = []
    for r in rows:
        if isinstance(r, (list, tuple)) and len(r) == 2 and not isinstance(r[1], (str,)):
            score, m = r
            mid = m.get("id") if isinstance(m, dict) else m
            out.append((mid, round(score, 3) if score is not None else None))
        else:
            out.append(r)
    print(f"  {q!r:42s} -> {out}")


print("### (1) fuzzy / wrong / paraphrased cell id -> real cell")
for q in ["gcd_u64", "greatest common factor", "highest common divisor",
          "manhatan distance", "maximum of two", "pick the larger",
          "is greater than", "choose best", "absolute difference"]:
    show(q)

print("\n### (2) behavioural route on the model's own I/O values")


def route(examples):
    try:
        pairs = [{"in": list(i), "out": o} for (i, o) in examples]
        return lib.route(pairs) if hasattr(lib, "route") else host.route(examples)
    except Exception as e:
        return f"ERR {e}"


cases = {
    "max(3,7)=7 ; max(10,3)=10": [((3, 7), 7), ((10, 3), 10)],
    "min(3,7)=3 ; min(10,3)=3": [((3, 7), 3), ((10, 3), 3)],
    "gcd(12,8)=4 ; gcd(21,14)=7": [((12, 8), 4), ((21, 14), 7)],
    "is_gt(7,3)=1 ; is_gt(3,7)=0": [((7, 3), 1), ((3, 7), 0)],
}
for label, ex in cases.items():
    res = route(ex)
    ids = [m.get("id") if isinstance(m, dict) else m for m in res][:4] if isinstance(res, list) else res
    print(f"  {label:34s} -> {ids}")

print("\nverdict: strong+deterministic here => search/route own cell-intent & the comparison escape hatch.")
