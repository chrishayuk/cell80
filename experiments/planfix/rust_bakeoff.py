#!/usr/bin/env python3
"""Race: get the model to emit our restricted-Rust cell dialect DIRECTLY (compile
with cell80, run) vs. emit arithmetic that we transpile via ast. Same model, same
problems. Tests whether the Python transpiler is needed or the model can hit the
dialect itself."""
import json
import os
import pathlib
import re
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

RUST_SYS = """Write ONE Rust function `fn run() -> u32` that computes the numeric answer to the problem, then stop.
STRICT RULES (a restricted Rust dialect — violating these fails to compile):
- Integers only: use `u32`. NO floats, NO decimals, NO f64 (write `x * 9 / 10`, never `x * 0.9`).
- Integer division TRUNCATES, so always multiply BEFORE you divide: write `total * 7 / 20`, never `7 / 20 * total`.
- Allowed: `let`, `if/else` (as an expression: `let m = if a > b { a } else { b };`), `while`, `+ - * /`, comparisons.
- NOT allowed: for-loops over iterators, Vec, String, arrays, .iter(), .sum(), function calls other than run.
- Bake the problem's numbers in as `u32` literals. The final line is the answer expression with NO semicolon.
- Output ONLY the code. No markdown fences, no comments, no prose.

Example:
Problem: Janet's ducks lay 16 eggs per day. She eats 3 for breakfast and bakes 4 into muffins. She sells the rest at $2 per egg. How many dollars does she make daily?
fn run() -> u32 {
    let eggs: u32 = 16;
    let after_breakfast = eggs - 3;
    let sold = after_breakfast - 4;
    sold * 2
}
"""

RESULT_RE = re.compile(r"result\s+(\d+)")


def extract_rust(text):
    text = text.strip()
    m = re.search(r"```(?:rust)?\s*(.*?)\s*```", text, re.DOTALL)
    if m:
        text = m.group(1).strip()
    i = text.find("fn ")
    if i == -1:
        return None
    j = text.rfind("}")
    if j == -1 or j < i:
        return None
    return text[i:j + 1]


def rust_arm(problem):
    raw = ask(RUST_SYS, problem)
    src = extract_rust(raw)
    if not src:
        return None, "no-fn"
    rs = os.path.join(tempfile.gettempdir(), "_rustarm.rs")
    cell = os.path.join(tempfile.gettempdir(), "_rustarm.cell")
    with open(rs, "w") as f:
        f.write(src)
    c = subprocess.run([CELL80, "compile", rs, "-o", cell, "--id", "rarm"],
                       capture_output=True, text=True, timeout=30)
    if c.returncode != 0:
        return None, "compile-fail"
    e = subprocess.run([CELL80, "exec", cell], capture_output=True, text=True, timeout=30)
    m = RESULT_RE.search(e.stdout)
    return (int(m.group(1)) if m else None), ("ok" if m else "run-fail")


def solve_plan(plan):
    path = os.path.join(tempfile.gettempdir(), "_rb.json")
    with open(path, "w") as f:
        json.dump([plan], f)
    out = subprocess.run([CELL80, "solve", path, "--json"], capture_output=True, text=True, timeout=30)
    if out.returncode != 0:
        return None
    try:
        return json.loads(out.stdout).get("answer")
    except json.JSONDecodeError:
        return None


def eq_arm(problem):
    raw = ask(EQ_SYS, problem)
    try:
        return solve_plan(equations_to_plan(raw)), "ok"
    except ParseFail as e:
        return None, f"parsefail"


def run():
    print(f"model = {MODEL}\n")
    print(f"{'row':16s} {'want':>6s} {'equations':>10s} {'':2s} {'direct-rust':>12s} {'':2s} notes")
    print("-" * 84)
    eqb = rub = 0
    for name, problem, exp in pilot.PROBLEMS:
        ea, en = eq_arm(problem)
        ra, rn = rust_arm(problem)
        ec, rc = (ea == exp), (ra == exp)
        eqb += ec
        rub += rc
        print(f"{name:16s} {str(exp):>6s} {str(ea):>10s} {'ok' if ec else '  ':>2s} "
              f"{str(ra):>12s} {'ok' if rc else '  ':>2s} {rn if rn!='ok' else ''}")
    n = len(pilot.PROBLEMS)
    print("-" * 84)
    print(f"equations(ast)  : {eqb}/{n} ({100*eqb//n}%)")
    print(f"direct-rust     : {rub}/{n} ({100*rub//n}%)")


if __name__ == "__main__":
    run()
