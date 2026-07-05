#!/usr/bin/env python3
"""Replay harness: for every captured model emission, compare
  baseline  = feed the raw emission straight to `cell80 solve`
  planfix   = run it through the deterministic adapter first, then solve
against the known expected answer. Model-free, deterministic, reproducible.

The corpus is the frozen pilot transcript — no model is called here.
"""
import json
import os
import pathlib
import subprocess
import sys
import tempfile

import planfix

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[1]
CELL80 = os.environ.get(
    "CELL80_BIN", "/Users/christopherhay/chris-source/cell80/target/release/cell80"
)
PILOT_DIR = REPO / "experiments" / "gsm8k-small-model-pilot"
CORPUS = os.environ.get(
    "CORPUS", str(PILOT_DIR / "results" / "pilot_results_granite.json")
)

# Authoritative expected answers — the corpus only records `expected` on rows
# that reached stage=solved, so join against the pilot's own PROBLEMS list.
sys.path.insert(0, str(PILOT_DIR))
import gsm8k_small_model_pilot as _pilot  # noqa: E402

EXPECTED = {name: exp for (name, _text, exp) in _pilot.PROBLEMS}


def solve(plans):
    """plans: list[dict] -> (answer, kills). Runs the real cell80 binary."""
    path = os.path.join(tempfile.gettempdir(), "_planfix_replay.json")
    with open(path, "w") as f:
        json.dump(plans, f)
    try:
        out = subprocess.run(
            [CELL80, "solve", path, "--json"], capture_output=True, text=True, timeout=30
        )
    except subprocess.TimeoutExpired:
        return None, ["timeout"]
    if out.returncode != 0:
        return None, [(out.stderr or out.stdout).strip()]
    try:
        rep = json.loads(out.stdout)
    except json.JSONDecodeError:
        return None, ["non-json solve output"]
    kills = [p.get("kill") for p in rep.get("plans", []) if p.get("kill")]
    return rep.get("answer"), kills


def baseline_input(rec):
    if "plan" in rec:
        return rec["plan"]
    if "raw" in rec:
        return rec["raw"]
    return None


def run():
    corpus = json.loads(pathlib.Path(CORPUS).read_text())
    rows = []
    b_ok = p_ok = 0
    for rec in corpus:
        name = rec["name"]
        expected = EXPECTED.get(name, rec.get("expected"))

        # --- baseline ---
        src = baseline_input(rec)
        b_ans, b_kill = None, ["no-input"]
        if isinstance(src, dict):
            b_ans, b_kill = solve([src])
        elif isinstance(src, str):
            try:
                obj = json.loads(src)
                b_ans, b_kill = solve([obj] if isinstance(obj, dict) else obj)
            except json.JSONDecodeError:
                b_ans, b_kill = None, ["baseline-bad-json"]
        b_correct = b_ans is not None and b_ans == expected
        b_ok += b_correct

        # --- planfix ---
        p_ans, p_note = None, None
        try:
            plans, repairs = planfix.normalize(src)
            p_ans, p_kill = solve(plans)
            p_note = "+".join(sorted({r[0] for r in repairs})) or "(no-op)"
            if p_kill and p_ans is None:
                p_note += f" | kill:{p_kill[0][:40]}"
        except planfix.Reject as e:
            p_note = f"REJECT:{e.code}"
        except Exception as e:  # adapter bug — surface it
            p_note = f"ADAPTER-ERR:{type(e).__name__}:{e}"
        p_correct = p_ans is not None and p_ans == expected
        p_ok += p_correct

        flag = ""
        if p_correct and not b_correct:
            flag = "  <== RESCUED"
        elif b_correct and not p_correct:
            flag = "  <== REGRESSED"
        elif p_ans is not None and not p_correct:
            # planfix rendered a runnable plan that returned the WRONG number —
            # format was repaired but the model's comprehension was wrong.
            flag = "  <== ran-but-wrong (needs consensus)"
        rows.append((name, expected, b_ans, b_correct, p_ans, p_correct, p_note, flag))

    n = len(corpus)
    print(f"{'row':16s} {'want':>6s} {'base':>6s} {'':2s} {'planfix':>7s} {'':2s}  notes")
    print("-" * 96)
    for name, exp, ba, bc, pa, pc, note, flag in rows:
        print(f"{name:16s} {str(exp):>6s} {str(ba):>6s} {'ok' if bc else '  ':>2s} "
              f"{str(pa):>7s} {'ok' if pc else '  ':>2s}  {note}{flag}")
    print("-" * 96)
    print(f"baseline correct: {b_ok}/{n} ({100*b_ok//n}%)")
    print(f"planfix  correct: {p_ok}/{n} ({100*p_ok//n}%)")
    print(f"net delta       : {p_ok - b_ok:+d}")


if __name__ == "__main__":
    run()
