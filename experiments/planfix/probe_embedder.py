#!/usr/bin/env python3
"""Empirical probe: does the potion embedder actually separate the semantic
distinctions planfix wants to lean on it for? Run BEFORE committing to the
learned layers. If NN is noisy here, fall back to structure + hardcoding.

Two questions:
  1. op phrasing  -> {add, sub, mul, div}
  2. field name   -> canonical role
and a control: object-op KEY names -> {operator, left, right, output}.
"""
import sys

import numpy as np

MODEL = sys.argv[1] if len(sys.argv) > 1 else "default"  # or "cell-potion"

from cell_eval.tiers import Embedder

emb = Embedder() if MODEL == "default" else Embedder(MODEL)


def nn(query, exemplars, k=2):
    q = emb.encode([query])[0]
    E = emb.encode(exemplars)
    sims = E @ q
    order = np.argsort(-sims)
    return [(exemplars[i], round(float(sims[i]), 3)) for i in order[:k]]


def block(title, queries, exemplars, expect=None):
    print(f"\n### {title}  (model={MODEL})")
    hits = 0
    for i, qy in enumerate(queries):
        ranked = nn(qy, exemplars)
        top = ranked[0][0]
        mark = ""
        if expect is not None:
            ok = top == expect[i]
            hits += ok
            mark = "  ✓" if ok else f"  ✗ want={expect[i]}"
        print(f"  {qy:24s} -> {ranked}{mark}")
    if expect is not None:
        print(f"  == {hits}/{len(queries)} top-1 correct ==")


# 1. op phrasing -> operation (open vocab the alias table would miss)
OPS = ["addition sum plus total", "subtraction minus difference less",
       "multiplication product times each", "division quotient split per"]
OP_LABEL = ["add", "sub", "mul", "div"]
op_phrases = ["increased by", "reduced by", "product of", "split evenly among",
              "three times as many", "how many fewer", "combined total of",
              "per hour", "twice as many", "remaining after"]
op_expect = ["add", "sub", "mul", "div", "mul", "sub", "add", "div", "mul", "sub"]
print("op exemplars:", dict(zip(OP_LABEL, OPS)))
block("op phrasing -> op", op_phrases,
      OPS, [OPS[OP_LABEL.index(e)] for e in op_expect])

# 2. field name -> role
ROLES = ["quantity count of items", "price per unit cost",
         "running total sum", "amount remaining left over",
         "rate speed per time", "elapsed time duration", "distance length"]
ROLE_LABEL = ["qty", "unit_price", "total", "remaining", "rate", "time", "distance"]
fields = ["pencils", "pencil_price", "notebooks", "sheep", "money_left",
          "miles_per_hour", "total_meters", "games_played", "containers",
          "harry_hours", "second_friend", "vehicles_per_container"]
print("\nrole exemplars:", dict(zip(ROLE_LABEL, ROLES)))
block("field name -> role", fields, ROLES)

# 3. object-op key names -> slot (the [A]-layer key matching)
KEYS = ["operator operation op", "left first operand a",
        "right second operand b", "output result destination out"]
KEY_LABEL = ["op", "a", "b", "out"]
keys = ["op", "operator", "a", "a_id", "left", "x", "arg0",
        "b", "b_id", "right", "y", "out", "out_id", "result", "to", "output"]
block("object key -> slot", keys, KEYS)

print("\nverdict: eyeball whether top-1 tracks intent. Noisy => lean structural.")
