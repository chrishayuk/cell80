#!/usr/bin/env python3
"""Replay captured M2.7 sources through the current `cell80 compose` — no model calls,
no generation drift: the registered way to measure a pass/rule change (here: the
zero-guard + E0205 method-to-kernel amendments, 2026-07-06)."""
import json
import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "experiments" / "gsm8k-small-model-pilot"))
import gsm8k_small_model_pilot as pilot  # noqa: E402

BIN = str(REPO / "target" / "release" / "cell80")
CELLS = str(REPO / "cell80" / "cells")
EXP = {n: e for n, _, e in pilot.PROBLEMS}
SRC = pathlib.Path(__file__).parent / "m27_sources"

for model_dir in sorted(SRC.iterdir()):
    if not model_dir.is_dir():
        continue
    uni_ok = uni_bad = maj_ok = maj_bad = esc = zero = 0
    changed = []
    for rowdir in sorted(model_dir.iterdir()):
        exp = EXP[rowdir.name]
        srcs = sorted(rowdir.glob("d*.rs"))
        r = subprocess.run([BIN, "compose", CELLS, *map(str, srcs), "--json"],
                           capture_output=True, text=True, timeout=90)
        rep = json.loads(r.stdout) if r.returncode == 0 else {"answer": None, "agreement": "escalate"}
        got, gate = rep.get("answer"), rep.get("agreement")
        old = json.loads((rowdir / "report.json").read_text())
        if (got, gate) != (old.get("answer"), old.get("agreement")):
            changed.append(f"{rowdir.name}: {old.get('answer')}/{old.get('agreement')} -> {got}/{gate}")
        if gate == "unanimous":
            uni_ok += got == exp; uni_bad += got != exp
        elif gate == "majority":
            maj_ok += got == exp; maj_bad += got != exp
        elif gate == "degenerate_zero":
            zero += 1
        else:
            esc += 1
    n = uni_ok + uni_bad + maj_ok + maj_bad
    print(f"{model_dir.name:20s} accepted {n:2d}/20  correct {uni_ok+maj_ok:2d}  wrong {uni_bad+maj_bad}"
          f"  escalate {esc}  degenerate_zero {zero}")
    for c in changed:
        print(f"    changed: {c}")
