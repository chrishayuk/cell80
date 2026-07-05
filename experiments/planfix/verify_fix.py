#!/usr/bin/env python3
"""Deterministically verify the defer-division + decimal->fraction transpiler fix
against the qwen2.5:3b equation outputs captured by eq_diagnose.py. No model call —
same text, new parser — so any change is purely the transpiler improvement."""
import json
import os
import subprocess
import tempfile

from equations_to_plan import ParseFail, equations_to_plan

CELL80 = os.environ.get("CELL80_BIN", "/Users/christopherhay/chris-source/cell80/target/release/cell80")

# verbatim model outputs captured 2026-07-05 (qwen2.5:3b), name -> (expected, raw)
CAPTURED = {
    "row11_downloads": (366, """first_month_downloads = 60
second_month_downloads = first_month_downloads * 3
third_month_downloads = second_month_downloads - 30/100 * second_month_downloads
total_downloads = first_month_downloads + second_month_downloads + third_month_downloads
answer = total_downloads"""),
    "row85_football": (15, """games_played = 22
losses = games_played / 2
wins = losses + 8
answer = wins"""),
    "row89_marilyn": (8000, """first_record_sales = 10 * harald_sales
total_sales = first_record_sales + harald_sales
answer = total_sales / 11"""),
    "row94_lee": (36, """lee_time = 38
original_lee_time = lee_time + 2
gerald_original_time = original_lee_time
gerald_improved_diet_time = gerald_original_time * .9
answer = gerald_improved_diet_time"""),
    "row97_harry": (3, """sleep_harry = 9
sleep_james = 2 / 3 * sleep_harry
difference = sleep_harry - sleep_james
answer = difference"""),
    "row101_jerome": (175, """first_friend_rings = 20
second_friend_rings = first_friend_rings + (1/4)*first_friend_rings
fourth_friend_rings = 60
third_friend_rings = fourth_friend_rings + 10
total_rings = first_friend_rings + second_friend_rings + third_friend_rings + fourth_friend_rings
answer = total_rings"""),
    "row117_katy": (42, """ratio_factor = 13/7
total_parts = 7 + 13
teaspoons_water = 120 / total_parts
teaspoons_sugar = teaspoons_water * ratio_factor
answer = teaspoons_sugar"""),
    "row122_morisette": (27, """morisette_fruits = 5 + 8
kael_apples = morisette_fruits
kael_oranges = morisette_fruits / 2
kael_fruits = kael_apples + kael_oranges
total_fruits = morisette_fruits + kael_fruits
answer = total_fruits"""),
}


def solve(plan):
    path = os.path.join(tempfile.gettempdir(), "_vfix.json")
    with open(path, "w") as f:
        json.dump([plan], f)
    out = subprocess.run([CELL80, "solve", path, "--json"], capture_output=True, text=True, timeout=30)
    if out.returncode != 0:
        return None
    try:
        return json.loads(out.stdout).get("answer")
    except json.JSONDecodeError:
        return None


def run():
    print("defer-division + decimal->fraction, re-parsing CAPTURED qwen2.5:3b outputs\n")
    fixed = 0
    for name, (exp, raw) in CAPTURED.items():
        try:
            ans = solve(equations_to_plan(raw))
        except ParseFail as e:
            ans = f"parsefail:{e}"
        ok = ans == exp
        fixed += ok
        print(f"  {name:18s} want={str(exp):>5s} now={str(ans):>6s}  {'FIXED' if ok else 'still off'}")
    print(f"\n  fixed by transpiler change alone: {fixed}/{len(CAPTURED)}")
    print("  (the rest are comprehension [row85/117/122] or algebraic [row89] — model-side, not ours)")


if __name__ == "__main__":
    run()
