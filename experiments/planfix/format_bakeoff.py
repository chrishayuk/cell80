#!/usr/bin/env python3
"""Head-to-head: does asking the model for ARITHMETIC (assignment lines, parsed by
`ast`) beat asking it for our strict JSON plan IR? Same model, same problems, so the
only variable is the extraction FORMAT. Both arms end at the same `cell80 solve`.
"""
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import urllib.request

import planfix
from equations_to_plan import ParseFail, equations_to_plan

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[1]
PILOT_DIR = REPO / "experiments" / "gsm8k-small-model-pilot"
CELL80 = os.environ.get("CELL80_BIN", "/Users/christopherhay/chris-source/cell80/target/release/cell80")
MODEL = os.environ.get("BAKEOFF_MODEL", "qwen2.5:3b")
OLLAMA = "http://localhost:11434/v1/chat/completions"

sys.path.insert(0, str(PILOT_DIR))
import gsm8k_small_model_pilot as pilot  # noqa: E402

JSON_SYS = pilot.SYSTEM_PROMPT  # the exact strict-JSON prompt from the pilot
EQ_SYS = """Solve the math problem by writing ONLY a list of assignment statements, one per line.
Rules:
- Use only non-negative integers and the operators + - * / (and parentheses).
- Give every intermediate step a name; reference earlier names.
- The LAST line must be `answer = <expression>`.
- Output ONLY the assignment lines. No prose, no units, no explanation.

Example:
Problem: Janet's ducks lay 16 eggs per day. She eats 3 for breakfast and bakes 4 into muffins. She sells the rest at $2 per egg. How many eggs does she sell daily?
eggs = 16
after_breakfast = eggs - 3
answer = after_breakfast - 4
"""


def ask(system, problem):
    body = {"model": MODEL, "temperature": 0, "stream": False,
            "messages": [{"role": "system", "content": system},
                         {"role": "user", "content": f'Problem: "{problem}"'}]}
    req = urllib.request.Request(OLLAMA, data=json.dumps(body).encode(),
                                headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=180) as r:
        return json.loads(r.read())["choices"][0]["message"]["content"]


def solve(plan):
    path = os.path.join(tempfile.gettempdir(), "_bakeoff.json")
    with open(path, "w") as f:
        json.dump([plan], f)
    out = subprocess.run([CELL80, "solve", path, "--json"], capture_output=True, text=True, timeout=30)
    if out.returncode != 0:
        return None
    try:
        return json.loads(out.stdout).get("answer")
    except json.JSONDecodeError:
        return None


def json_arm(problem):
    raw = ask(JSON_SYS, problem)
    try:
        plans, _ = planfix.normalize(raw)   # includes planfix repair
        return solve(plans[0]), "ok"
    except planfix.Reject as e:
        return None, f"reject:{e.code}"
    except Exception as e:
        return None, f"err:{type(e).__name__}"


def eq_arm(problem):
    raw = ask(EQ_SYS, problem)
    try:
        plan = equations_to_plan(raw)
        return solve(plan), "ok"
    except ParseFail as e:
        return None, f"parsefail:{e}"
    except Exception as e:
        return None, f"err:{type(e).__name__}"


def run():
    # parser self-test
    demo = "eggs = 16\nafter = eggs - 3\nanswer = after - 4"
    assert solve(equations_to_plan(demo)) == 9, "parser self-test failed"
    print(f"model = {MODEL}   (parser self-test ok)\n")
    print(f"{'row':16s} {'want':>6s} {'json+planfix':>13s} {'':2s} {'equations':>10s} {'':2s}")
    print("-" * 78)
    jb = eqb = 0
    for name, problem, exp in pilot.PROBLEMS:
        ja, jn = json_arm(problem)
        ea, en = eq_arm(problem)
        jc, ec = (ja == exp), (ea == exp)
        jb += jc
        eqb += ec
        print(f"{name:16s} {str(exp):>6s} {str(ja):>13s} {'ok' if jc else '  ':>2s} "
              f"{str(ea):>10s} {'ok' if ec else '  ':>2s}  {'' if jn=='ok' else jn} {'' if en=='ok' else en}")
    n = len(pilot.PROBLEMS)
    print("-" * 78)
    print(f"JSON + planfix : {jb}/{n} ({100*jb//n}%)")
    print(f"equations(ast) : {eqb}/{n} ({100*eqb//n}%)")


if __name__ == "__main__":
    run()
