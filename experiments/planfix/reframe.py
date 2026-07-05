#!/usr/bin/env python3
"""Close the loop: search generates candidate answers (grounded, already computed
by solve), the model only has to RECOGNISE the right one given the problem text.
This is the model back in the loop as a *judge over grounded options*, not a
blank-page extractor.

For each comprehension-miss row: build the interpretation beam -> distinct
candidate values -> ask a small model which one answers the problem.
"""
import json
import urllib.request

from comprehension_beam import (CORPUS, EXPECTED, TEXT, interpretation_beam,
                                planfix, solve_one)

OLLAMA = "http://localhost:11434/v1/chat/completions"
MODEL = "qwen2.5:3b"  # fast, non-thinking, 3B — the "small model" spirit

SYS = ("You are given a grade-school math word problem and a list of candidate "
       "numeric answers a calculator already computed. Exactly one is correct. "
       "Reply with ONLY that number — no words, no units, no explanation.")


def judge(problem, candidates):
    body = {
        "model": MODEL,
        "messages": [
            {"role": "system", "content": SYS},
            {"role": "user",
             "content": f"Problem: {problem}\nCandidate answers: {candidates}\nThe correct answer is:"},
        ],
        "temperature": 0,
        "stream": False,
    }
    req = urllib.request.Request(OLLAMA, data=json.dumps(body).encode(),
                                headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=120) as r:
        txt = json.loads(r.read())["choices"][0]["message"]["content"]
    import re
    m = re.search(r"-?\d+", txt.replace(",", ""))
    return int(m.group()) if m else None


def run():
    print(f"judge model = {MODEL}\n")
    print(f"{'row':16s} {'want':>6s} {'model1shot':>10s} {'picked':>7s}  outcome")
    print("-" * 92)
    recovered = 0
    total_miss = 0
    for rec in CORPUS:
        name = rec["name"]
        if "plan" not in rec:
            continue
        exp = EXPECTED[name]
        first = rec.get("answer")
        if first == exp:
            continue  # model already right first-shot
        total_miss += 1
        try:
            plans, _ = planfix.normalize(rec["plan"])
            beam = interpretation_beam(plans[0])
        except planfix.Reject as e:
            print(f"{name:16s} {str(exp):>6s} {str(first):>10s} {'--':>7s}  no beam (REJECT:{e.code})")
            continue
        vals = sorted({v for v in (solve_one(c) for c in beam.values()) if v is not None})
        if not vals:
            print(f"{name:16s} {str(exp):>6s} {str(first):>10s} {'--':>7s}  no candidates ran")
            continue
        in_beam = exp in vals
        picked = judge(TEXT[name], vals)
        ok = picked == exp
        recovered += ok
        note = "RECOVERED" if ok else ("pickable-but-missed" if in_beam else "answer NOT in beam (needs re-extraction)")
        print(f"{name:16s} {str(exp):>6s} {str(first):>10s} {str(picked):>7s}  {note}  cands={vals}")
    print("-" * 92)
    print(f"comprehension misses recovered by beam+judge: {recovered}/{total_miss}")


if __name__ == "__main__":
    run()
