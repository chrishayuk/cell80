#!/usr/bin/env python3
"""Link-time resolution WITH a feedback round-trip (the better design).

The model writes freely, naming operations however it likes. We link its calls to
verified library cells, inline them, compile, and RUN — then hand back the REWRITTEN
program + the result. The model sees what each call resolved to, what the composed
cell actually computes, and the number it produced, and either accepts (DONE) or
rewrites. Catches mis-resolution AND the model's own logic errors; the model keeps
agency. No prompt-time constraint, no fixed vocabulary.
"""
import json
import os
import re
import sys
import urllib.request

from compose_link import execute, link_and_compile

OLLAMA = "http://localhost:11434/v1/chat/completions"
MODEL = os.environ.get("BAKEOFF_MODEL", "qwen2.5:3b")

SYS = """Write ONE Rust function `fn run() -> u16` that computes the numeric answer, then stop.
u16 integers only; NO floats; multiply BEFORE dividing; `let`/`if`/`while`/`+ - * /`/comparisons ok;
bake the numbers in as literals; final expression (no semicolon) is the answer.
You MAY call named operations (e.g. max, min, gcd, lcm, abs_diff, is_gt) by intent — they will be
resolved to already-verified library cells for you. Output ONLY the code."""


def chat(messages):
    body = {"model": MODEL, "temperature": 0, "stream": False, "messages": messages}
    req = urllib.request.Request(OLLAMA, data=json.dumps(body).encode(),
                                headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=180) as r:
        return json.loads(r.read())["choices"][0]["message"]["content"]


def extract_rust(text):
    text = re.sub(r"```\w*", "", text).strip()
    i, j = text.find("fn "), text.rfind("}")
    return text[i:j + 1] if (i != -1 and j > i) else None


def roundtrip(problem, expect, rounds=3):
    print(f"\n{'='*80}\nPROBLEM: {problem}\n(expect {expect})")
    messages = [{"role": "system", "content": SYS},
                {"role": "user", "content": f'Problem: "{problem}"'}]
    last = None
    for rnd in range(1, rounds + 1):
        reply = chat(messages)
        if "DONE" in reply.upper() and last is not None:
            print(f"  round {rnd}: model says DONE.")
            break
        src = extract_rust(reply)
        messages.append({"role": "assistant", "content": reply})
        if not src:
            messages.append({"role": "user", "content": "That wasn't a `fn run` — write the Rust function."})
            print(f"  round {rnd}: no code")
            continue
        res = link_and_compile(src)
        if res["ok"]:
            last = execute(res["cell"])
            resolved = "; ".join(f"{n}→cell '{c}'" for n, c in res["resolutions"]) or "(no library calls)"
            print(f"  round {rnd}: {re.sub(chr(10), ' ', src)[:70]}...")
            print(f"           resolved: {resolved}")
            print(f"           RAN → {last}   {'✓' if last == expect else '(want %s)' % expect}")
            fb = (f"I resolved your calls and ran the composed cell.\n"
                  f"REWRITTEN PROGRAM:\n{res['src']}\n"
                  f"RESOLVED: {resolved}\n"
                  f"RESULT: it returned {last}.\n"
                  f"If that correctly answers the problem, reply exactly: DONE\n"
                  f"Otherwise, rewrite `fn run` to fix it.")
        else:
            print(f"  round {rnd}: did not compile — {res['err']}")
            fb = f"It did not compile: {res['err']}\nRewrite `fn run` (call a different named op if a call was unresolved)."
        messages.append({"role": "user", "content": fb})
    print(f"  FINAL: {last}   {'✓ correct' if last == expect else '✗ want %s' % expect}")
    return last


if __name__ == "__main__":
    roundtrip("Two trucks earned 480 and 350 dollars. The winner's bonus is the difference in their earnings. What bonus?", 130)
    roundtrip("A football team played 22 games and won 8 more than they lost. How many did they win?", 15)
    roundtrip("Shop A profit is 340, shop B profit is 275. The prize is double the larger profit. What is the prize?", 680)
