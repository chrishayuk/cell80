#!/usr/bin/env python3
"""Root-cause the equation-arm failures: capture the RAW arithmetic the model
emitted for every problem, parse+solve it, and dump the failures verbatim so we
can categorize *why* (fractional/percent, comprehension, parse-edge, unit)."""
import json
import os
import pathlib
import subprocess
import sys
import tempfile

from equations_to_plan import ParseFail, equations_to_plan
from format_bakeoff import EQ_SYS, ask, MODEL

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[1]
CELL80 = os.environ.get("CELL80_BIN", "/Users/christopherhay/chris-source/cell80/target/release/cell80")
sys.path.insert(0, str(REPO / "experiments" / "gsm8k-small-model-pilot"))
import gsm8k_small_model_pilot as pilot  # noqa: E402


def solve(plan):
    path = os.path.join(tempfile.gettempdir(), "_eqdiag.json")
    with open(path, "w") as f:
        json.dump([plan], f)
    out = subprocess.run([CELL80, "solve", path, "--json"], capture_output=True, text=True, timeout=30)
    if out.returncode != 0:
        return None, (out.stderr or out.stdout).strip()
    try:
        return json.loads(out.stdout).get("answer"), None
    except json.JSONDecodeError:
        return None, "non-json"


def run():
    print(f"model = {MODEL}\n")
    fails = []
    for name, problem, exp in pilot.PROBLEMS:
        raw = ask(EQ_SYS, problem)
        parse_err = plan = ans = kill = None
        try:
            plan = equations_to_plan(raw)
            ans, kill = solve(plan)
        except ParseFail as e:
            parse_err = str(e)
        ok = ans == exp
        if not ok:
            fails.append((name, exp, ans, problem, raw, parse_err, kill))
    print(f"=== {len(fails)} FAILURES (verbatim model output) ===\n")
    for name, exp, ans, problem, raw, parse_err, kill in fails:
        print(f"### {name}  want={exp} got={ans}"
              f"{'  parse_err='+parse_err if parse_err else ''}"
              f"{'  kill='+str(kill) if kill else ''}")
        print(f"  Q: {problem[:150]}")
        print("  MODEL:")
        for ln in raw.strip().splitlines():
            print(f"    {ln}")
        print()


if __name__ == "__main__":
    run()
