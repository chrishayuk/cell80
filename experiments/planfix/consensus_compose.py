#!/usr/bin/env python3
"""Objective acceptance: replace the model's self-certified DONE with CONSENSUS.
Generate N independent compositions (temperature>0 for diversity), link+compile+run
each to a verified cell, and accept an answer only if a majority AGREE — else declare
NO CONSENSUS (escalate). The model's fallible self-judgment (it said DONE on a wrong
10) is removed from the trust path; agreement across independent attempts is the
signal. Light auto-fixes clear the dialect mechanics weak models get stuck on.
"""
import json
import os
import re
import urllib.request
from collections import Counter

from compose_link import execute, link_and_compile

OLLAMA = "http://localhost:11434/v1/chat/completions"
MODEL = os.environ.get("BAKEOFF_MODEL", "qwen2.5:3b")
N = int(os.environ.get("N", "3"))

SYS = """Write ONE Rust function `fn run() -> u16` that computes the numeric answer, then stop.
u16 integers only; NO floats/macros; multiply BEFORE dividing; `let`/`if`/`while`/`+ - * /`/comparisons ok;
call-arguments must be a plain name or literal (bind compound expressions with `let` first);
bake numbers in as literals; final expression (no semicolon) is the answer.
You MAY call named ops (max, min, gcd, lcm, abs_diff, is_gt) by intent — they resolve to verified cells.
Output ONLY the code."""

# a little diversity nudge so N attempts explore different formulations
NUDGES = ["", " Think step by step.", " Use the fewest operations.",
          " Name each intermediate clearly.", " Double-check the arithmetic."]


def ask(problem, temp, nudge):
    body = {"model": MODEL, "temperature": temp, "stream": False,
            "messages": [{"role": "system", "content": SYS},
                         {"role": "user", "content": f'Problem: "{problem}"{nudge}'}]}
    req = urllib.request.Request(OLLAMA, data=json.dumps(body).encode(),
                                headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=180) as r:
        return json.loads(r.read())["choices"][0]["message"]["content"]


def autofix(src):
    src = re.sub(r"^\s*[a-z_]\w*!\s*\([^;]*\)\s*;?\s*$", "", src, flags=re.M)  # drop statement macros
    for _ in range(3):
        src = re.sub(r"\(\(([^()]*)\)\)", r"(\1)", src)  # collapse redundant double parens
    return src


def extract_rust(text):
    text = re.sub(r"```\w*", "", text).strip()
    i, j = text.find("fn "), text.rfind("}")
    return autofix(text[i:j + 1]) if (i != -1 and j > i) else None


def evaluate(src):
    if not src:
        return None
    res = link_and_compile(src)
    return execute(res["cell"]) if res["ok"] else None


def consensus(problem, expect):
    print(f"\n{'='*78}\n{problem[:76]}  (expect {expect})")
    answers = []
    for i in range(N):
        src = extract_rust(ask(problem, 0.8, NUDGES[i % len(NUDGES)]))
        a = evaluate(src)
        answers.append(a)
        print(f"  attempt {i+1}: {a}")
    valid = [a for a in answers if a is not None]
    if not valid:
        print("  -> NO VALID COMPILE (escalate)")
        return None
    cnt = Counter(valid)
    top, k = cnt.most_common(1)[0]
    if k > N / 2:
        verdict = "CONSENSUS" if top == expect else "CONSENSUS-BUT-WRONG"
        print(f"  -> {verdict} on {top}  ({k}/{len(valid)} agree)")
        return top
    print(f"  -> NO CONSENSUS {dict(cnt)}  (escalate, don't emit a number)")
    return None


if __name__ == "__main__":
    consensus("A football team played 22 games and won 8 more than they lost. How many did they win?", 15)
    consensus("Shop A profit is 340, shop B profit is 275. The prize is double the larger profit. What is the prize?", 680)
    consensus("Two trucks earned 480 and 350 dollars. The winner's bonus is the difference in their earnings. What bonus?", 130)
