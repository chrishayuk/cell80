#!/usr/bin/env python3
"""Capture WHY each of the 4 gemma4 escalations diverged: dump both derivations'
source, link result, and answer for row86/89/93/101, to ground the roadmap."""
import pathlib
import sys

from compose_link import execute, link_and_compile
from structured_consensus import BASE, METHODS, ask, autofix, to_fn

REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "experiments" / "gsm8k-small-model-pilot"))
import gsm8k_small_model_pilot as pilot  # noqa: E402

ROWS = {"row86_gene", "row89_marilyn", "row93_emily", "row101_jerome"}

for name, problem, exp in pilot.PROBLEMS:
    if name not in ROWS:
        continue
    print(f"\n{'='*80}\n{name}  want={exp}\n  Q: {problem[:150]}")
    for m, instr in METHODS.items():
        src = to_fn(ask(BASE + instr, problem))
        res = link_and_compile(autofix(src)) if src else {"ok": False, "err": "no-code"}
        ans = execute(res["cell"]) if res.get("ok") else None
        links = ", ".join(f"{n}->{c}" for n, c in res.get("resolutions", [])) or "inline"
        print(f"\n  --- {m}  (links: {links})  -> {ans} ---")
        for ln in (src or "").splitlines():
            print(f"    {ln}")
        if not res.get("ok"):
            print(f"    [FAIL: {res.get('err','')[:90]}]")
