#!/usr/bin/env python3
"""Prove the two ingredients a plan-search needs, both already in `solve`:
  A. consensus battery as a BEAM ADJUDICATOR — divergent survivors -> escalate,
     computationally-equivalent survivors -> answer.
  B. ExactDiv as an ANSWER-FREE validity oracle — a non-exact division kills the
     candidate without anyone knowing the right answer.
No model, no known-answer fitness. Just render+compile+run outcomes.
"""
import json
import os
import subprocess
import tempfile

CELL80 = os.environ.get(
    "CELL80_BIN", "/Users/christopherhay/chris-source/cell80/target/release/cell80"
)


def solve(plans):
    path = os.path.join(tempfile.gettempdir(), "_beam_demo.json")
    with open(path, "w") as f:
        json.dump(plans, f)
    out = subprocess.run([CELL80, "solve", path, "--json"],
                         capture_output=True, text=True, timeout=30)
    if out.returncode != 0:
        return {"error": (out.stderr or out.stdout).strip()}
    return json.loads(out.stdout)


def q(id, v, u="count"):
    return {"id": id, "value": v, "unit": u}


def show(title, plans):
    rep = solve(plans)
    ans = rep.get("answer")
    battery = rep.get("battery_ran")
    kills = [p.get("kill") for p in rep.get("plans", [])]
    survivors = sum(1 for p in rep.get("plans", []) if not p.get("kill"))
    verdict = "ESCALATE" if ans is None else f"answer={ans}"
    print(f"\n{title}")
    print(f"  survivors={survivors}/{len(plans)} battery_ran={battery} -> {verdict}")
    for i, k in enumerate(kills):
        print(f"    plan[{i}] kill={k}")


# A1. two survivors that DISAGREE -> escalate (no silent pick)
show("A1  divergent beam (6*7=42  vs  6+7=13)  -> must ESCALATE", [
    {"quantities": [q("x", 6), q("y", 7)], "ops": [["mul", "x", "y", "p"]], "target": "p"},
    {"quantities": [q("x", 6), q("y", 7)], "ops": [["add", "x", "y", "p"]], "target": "p"},
])

# A2. two survivors that are the SAME FUNCTION (operands swapped) -> answer
show("A2  equivalent beam (x*y  vs  y*x)  -> AGREE on 42", [
    {"quantities": [q("x", 6), q("y", 7)], "ops": [["mul", "x", "y", "p"]], "target": "p"},
    {"quantities": [q("x", 6), q("y", 7)], "ops": [["mul", "y", "x", "p"]], "target": "p"},
])

# A3. same base answer but DIFFERENT SENSITIVITY -> battery catches it -> escalate
#     42 via 6*7  vs  42 via 21+21 (2*21). Both return 42, but perturbing quantities
#     moves them differently, so the battery refuses to conflate them.
show("A3  same answer, different structure (6*7  vs  21+21) -> battery ESCALATES", [
    {"quantities": [q("x", 6), q("y", 7)], "ops": [["mul", "x", "y", "p"]], "target": "p"},
    {"quantities": [q("a", 21), q("b", 21)], "ops": [["add", "a", "b", "p"]], "target": "p"},
])

# B. ExactDiv oracle: non-exact division killed answer-free; exact ordering survives.
show("B1  7 / 2 with exact_div constraint -> KILLED (non-exact, answer-free)", [
    {"quantities": [q("a", 7), q("b", 2)], "ops": [["div", "a", "b", "r"]],
     "target": "r", "constraints": [["exact_div", "a", "b"]]},
])
show("B2  8 / 2 with exact_div constraint -> survives -> 4", [
    {"quantities": [q("a", 8), q("b", 2)], "ops": [["div", "a", "b", "r"]],
     "target": "r", "constraints": [["exact_div", "a", "b"]]},
])
