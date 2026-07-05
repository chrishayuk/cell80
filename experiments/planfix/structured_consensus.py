#!/usr/bin/env python3
"""Structured cross-check (temp 0): derive the answer two genuinely DIFFERENT ways
and accept only if they agree. Diversity comes from the METHOD, not from temperature
noise — so the weak model stays reliable AND we get an independent check.

  method A (inline)   : solve with ONLY inline arithmetic / if-else, no named calls
  method B (composed) : solve by CALLING library ops (max/min/gcd/abs_diff/...) where one applies

Both compile+run to a verified cell. Agree -> accept (two code paths concur).
Disagree / only-one-valid -> escalate (no number). No model self-certification.
"""
import json
import os
import re
import urllib.request

from compose_link import execute, link_and_compile

OLLAMA = "http://localhost:11434/v1/chat/completions"
MODEL = os.environ.get("BAKEOFF_MODEL", "qwen2.5:3b")

BASE = """Write ONE Rust function `fn run() -> u16` that computes the numeric answer, then stop.
u16 integers only; NO floats/macros; multiply BEFORE dividing; `let`/`if`/`while`/`+ - * /`/comparisons ok;
call-arguments must be a plain name or literal; bake numbers in as literals; final expression (no ;) is the answer.
Output ONLY the code."""

METHODS = {
    "inline": " Solve using ONLY inline arithmetic and if/else. Do NOT call any named functions.",
    "composed": " Solve by CALLING named library operations (max, min, gcd, lcm, abs_diff, is_gt) wherever one fits, plus arithmetic. They resolve to verified cells.",
}


def ask(system, problem):
    body = {"model": MODEL, "temperature": 0, "stream": False,
            "messages": [{"role": "system", "content": system},
                         {"role": "user", "content": f'Problem: "{problem}"'}]}
    req = urllib.request.Request(OLLAMA, data=json.dumps(body).encode(),
                                headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=180) as r:
        return json.loads(r.read())["choices"][0]["message"]["content"]


def autofix(src):
    src = re.sub(r"^\s*[a-z_]\w*!\s*\([^;]*\)\s*;?\s*$", "", src, flags=re.M)
    for _ in range(3):
        src = re.sub(r"\(\(([^()]*)\)\)", r"(\1)", src)
    return src


def to_fn(text):
    text = re.sub(r"```\w*", "", text).strip()
    if "fn " in text:
        i, j = text.find("fn "), text.rfind("}")
        return text[i:j + 1] if j > i else None
    # model dropped the wrapper and returned a bare body — wrap it
    lines = [l for l in text.splitlines() if l.strip() and not l.strip().startswith(("//", "#"))]
    return "fn run() -> u16 {\n" + "\n".join(lines) + "\n}" if lines else None


def solve(problem, method_instr):
    src = to_fn(ask(BASE + method_instr, problem))
    if not src:
        return None, "no-code"
    res = link_and_compile(autofix(src))
    if not res["ok"]:
        return None, "compile-fail"
    calls = ",".join(f"{n}->{c}" for n, c in res["resolutions"]) or "inline"
    return execute(res["cell"]), calls


def cross_check(problem, expect):
    print(f"\n{'='*80}\n{problem[:78]}  (expect {expect})")
    results = {}
    for name, instr in METHODS.items():
        ans, how = solve(problem, instr)
        results[name] = ans
        print(f"  {name:9s}: {str(ans):<6} ({how})")
    valid = {k: v for k, v in results.items() if v is not None}
    vals = set(valid.values())
    if len(valid) >= 2 and len(vals) == 1:
        a = vals.pop()
        print(f"  -> AGREE on {a}  {'✓ correct' if a == expect else '✗ (want %s)' % expect}  [two independent derivations concur]")
        return a
    if len(valid) >= 2:
        print(f"  -> DISAGREE {results}  -> escalate (no number)")
    else:
        print(f"  -> only {len(valid)} path compiled -> escalate (need 2 to cross-check)")
    return None


if __name__ == "__main__":
    cross_check("A football team played 22 games and won 8 more than they lost. How many did they win?", 15)
    cross_check("Shop A profit is 340, shop B profit is 275. The prize is double the larger profit. What is the prize?", 680)
    cross_check("Two trucks earned 480 and 350 dollars. The winner's bonus is the difference in their earnings. What bonus?", 130)
    cross_check("Rosie can run 10 miles per hour for 3 hours, then 5 miles per hour. How far in 5 hours total?", 40)
    cross_check("Cody eats three times as many cookies as Amir. If Amir eats 5, how many do both eat together?", 20)
