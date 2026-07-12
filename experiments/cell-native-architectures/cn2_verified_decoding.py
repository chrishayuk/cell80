#!/usr/bin/env python3
"""CN-2 slice-0 — verified decoding, measurement only (no injection/resampling yet).

Sends a small GSM8K-style arithmetic-word-problem battery to a running LARQL
server's OpenAI-compatible /v1/chat/completions endpoint, asking the model to
show each arithmetic step as "A op B = C". Every such span is independently
re-derived by cell80 (via cell80-py's CellHost.solve, the plan-IR path used by
cell80/examples/m3_gsm8k_smoketest.rs) and compared bit-exact. This measures
the wrong-number rate baseline the CN-2 gate (experiments/cell-native-
architectures.md) compares "after resampling" against — no changes to LARQL
itself.

Prereq: a LARQL server running against a gemma3-4b vindex, e.g.:
  LARQL_SPIN_POOL=0 larql serve \
    /Users/christopherhay/chris-source/larql/output/gemma3-4b-q4k-v2.vindex --port 8080
(LARQL_SPIN_POOL=0 is required until the spin_pool concurrency bug documented
in ../cell-native-architectures-findings.md is root-caused — the default
dispatch path crashes the server under real traffic.)

Run: python3 cn2_verified_decoding.py [--url http://localhost:8080]
"""
from __future__ import annotations

import argparse
import json
import re
import time
from pathlib import Path

import cell80_py
import requests

OUT = Path(__file__).resolve().parent / "cn2_verified_decoding_results.json"

# 15 hand-authored problems (natural multi-step arithmetic word problems,
# each with an unambiguous single decomposition and a known final answer)
# plus 45 generated ones (_gen_battery below, larger numbers/more steps -
# the hand-authored 15 alone were too easy: the model went 15/15 on them
# in the slice-0 pilot, giving the wrong-number-rate measurement nothing to
# catch). Final answer is a sanity cross-check; CN-2 verifies per-SPAN
# arithmetic, not the final answer.
BATTERY = [
    {"q": "Sam has 12 apples. He buys 7 more, then gives 5 to his friend. How many apples does Sam have now?", "final": 14},
    {"q": "A classroom has 8 rows of 6 desks each. How many desks are in the classroom?", "final": 48},
    {"q": "Maria earns $15 per hour and works 6 hours. She then spends $40 on groceries. How much money does she have left?", "final": 50},
    {"q": "A baker makes 84 cookies and packs them into boxes of 12. How many boxes does he need?", "final": 7},
    {"q": "There are 23 red marbles and 17 blue marbles in a jar. If 9 marbles are removed, how many marbles are left?", "final": 31},
    {"q": "A train travels 60 miles per hour for 3 hours, then 40 miles per hour for 2 hours. How many miles does it travel in total?", "final": 260},
    {"q": "Tom had 90 dollars. He spent 25 dollars on a book and then earned 15 dollars mowing a lawn. How much money does Tom have now?", "final": 80},
    {"q": "A garden has 9 rows with 8 plants in each row. If 5 plants die, how many plants are left?", "final": 67},
    {"q": "Jenny read 18 pages on Monday and twice as many pages on Tuesday. How many pages did she read in total over the two days?", "final": 54},
    {"q": "A factory produces 144 toys and ships them in crates of 16. How many crates are needed?", "final": 9},
    {"q": "Liam has 50 dollars. He buys 3 notebooks at 6 dollars each. How much money does he have left?", "final": 32},
    {"q": "A pool holds 500 liters. It is filled at 25 liters per minute. How many minutes does it take to fill?", "final": 20},
    {"q": "There are 14 boys and 19 girls in a class. Each student gets 3 pencils. How many pencils are given out in total?", "final": 99},
    {"q": "A shop had 120 shirts. It sold 45 in the morning and 38 in the afternoon. How many shirts are left?", "final": 37},
    {"q": "Ana saves 12 dollars a week. After 5 weeks she spends 30 dollars on a gift. How much has she saved?", "final": 30},
]


def _gen_battery(n, seed):
    # Larger numbers and 3-4 step chains than the hand-authored 15 above -
    # those were "too easy" (model got every final answer right unaided in
    # slice-0), giving CN-2's wrong-number-rate measurement nothing to
    # catch. Ground truth computed here, not by hand, so it's trustworthy
    # at this volume.
    import random

    rng = random.Random(seed)
    names = ["Maria", "Jordan", "Priya", "Wei", "Carlos", "Nina", "Omar", "Ivy", "Deshawn", "Sofia"]
    items = ["boxes", "crates", "bags", "cartons", "pallets"]
    rows = []

    def name():
        return rng.choice(names)

    templates = [
        # A*B - C - D (two purchases then a loss)
        lambda: (
            (a := rng.randint(23, 87)),
            (b := rng.randint(14, 39)),
            (c := rng.randint(50, 400)),
            (d := rng.randint(20, 300)),
            f"{name()} has {a} rows of {b} items in a warehouse. {c} items are sold, then "
            f"{d} more are sold. How many items are left?",
            a * b - c - d,
        )[-2:],
        # (A+B+C) * D  (three-day totals scaled)
        lambda: (
            (a := rng.randint(120, 480)),
            (b := rng.randint(90, 350)),
            (c := rng.randint(60, 300)),
            (d := rng.randint(3, 9)),
            f"{name()} reads {a} pages on Monday, {b} pages on Tuesday, and {c} pages on "
            f"Wednesday. Over the next {d} weeks, {name()} reads that same three-day total "
            f"every week. How many pages in total over those {d} weeks?",
            (a + b + c) * d,
        )[-2:],
        # A*H1 + B*H2 - S  (rates over time, then a deduction)
        lambda: (
            (a := rng.randint(18, 45)),
            (h1 := rng.randint(6, 9)),
            (b := rng.randint(20, 60)),
            (h2 := rng.randint(4, 8)),
            (s := rng.randint(80, 500)),
            f"{name()} earns ${a} per hour for the first {h1} hours of a shift and ${b} per "
            f"hour for the next {h2} hours. {name()} then pays ${s} in expenses. How much "
            f"money is left?",
            a * h1 + b * h2 - s,
        )[-2:],
        # (A - B) items into groups of C, exact division
        lambda: (
            (c := rng.choice([6, 7, 8, 9, 11, 12, 13, 14])),
            (k := rng.randint(30, 90)),
            (a := c * k + (b := rng.randint(50, 400))),
            f"A factory makes {a} parts. {b} are defective and discarded. The rest are packed "
            f"into {rng.choice(items)} of {c} parts each. How many {rng.choice(items)} are needed?",
            (a - b) // c,
        )[-2:],
        # A*B + C*D  (two group totals combined)
        lambda: (
            (a := rng.randint(12, 34)),
            (b := rng.randint(18, 55)),
            (c := rng.randint(9, 27)),
            (d := rng.randint(25, 70)),
            f"A school orders {a} boxes of {b} pencils each for one grade, and {c} boxes of "
            f"{d} pencils each for another grade. How many pencils were ordered in total?",
            a * b + c * d,
        )[-2:],
        # A - N*P - M*Q  (budget, two kinds of purchases)
        lambda: (
            (a := rng.randint(400, 1200)),
            (n := rng.randint(3, 9)),
            (p := rng.randint(15, 60)),
            (m := rng.randint(2, 6)),
            (q := rng.randint(20, 90)),
            f"{name()} has ${a}. {name()} buys {n} shirts at ${p} each and {m} jackets at "
            f"${q} each. How much money is left?",
            a - n * p - m * q,
        )[-2:],
    ]

    for i in range(n):
        text, final = templates[i % len(templates)]()
        rows.append({"q": text, "final": final})
    return rows


BATTERY = BATTERY + _gen_battery(45, seed=20260712)

SYSTEM = (
    "Solve the problem step by step. For every arithmetic calculation you perform, "
    "write it on its own line using digits and a real math symbol - never the word "
    "'op' or any other placeholder, always the actual symbol +, -, *, or /. "
    "For example, if you compute 12 plus 7, write the line '12 + 7 = 19' (not "
    "'12 op 7 = 19'). Do not skip this notation for any step. After all steps, "
    "write a final line exactly as 'Answer: N'."
)

SPAN_RE = re.compile(r"(-?\d+(?:\.\d+)?)\s*([+\-*/x×])\s*(-?\d+(?:\.\d+)?)\s*=\s*(-?\d+(?:\.\d+)?)")
OP_MAP = {"+": "add", "-": "sub", "*": "mul", "x": "mul", "×": "mul", "/": "div"}


def chat(url: str, prompt: str, max_tokens: int = 400, timeout: int = 600) -> str:
    payload = {
        "messages": [
            {"role": "system", "content": SYSTEM},
            {"role": "user", "content": prompt},
        ],
        "max_tokens": max_tokens,
        "temperature": 0.0,
        "stream": False,
    }
    r = requests.post(f"{url}/v1/chat/completions", json=payload, timeout=timeout)
    r.raise_for_status()
    data = r.json()
    return data["choices"][0]["message"]["content"] or ""


_OPERATOR_CHARS = set("+-*/x×")


def extract_spans(text: str):
    spans = []
    for raw_line in text.splitlines():
        line = raw_line.strip()
        for m in SPAN_RE.finditer(line):
            # Reject a chain continuation: "437 + 127 + 207 = 771" partially
            # matches as the two-operand substring "127 + 207 = 771" - that
            # slice is real arithmetic (127+207=334) but was never claimed to
            # equal 771 (the line's actual claim is a 3-operand sum), so
            # verifying it against 771 is a false-positive mismatch, not a
            # caught model error. Detected by checking the non-whitespace
            # character immediately before the match: an arithmetic operator
            # there means this operand is itself a continuation of an
            # earlier term in the same expression. A label prefix like
            # "Total marbles initially = 23 + 17 = 40" doesn't trip this
            # (the preceding non-whitespace char is "=", not an operator),
            # so genuine single-equation lines with descriptive prefixes are
            # still accepted.
            prefix = line[: m.start()].rstrip()
            if prefix and prefix[-1] in _OPERATOR_CHARS:
                continue
            # Mirror check on the suffix: a self-verification decomposition
            # ("6 * 578 = 6 * (500 + 70 + 8) = ... = 3468") or a degenerate
            # repetition loop ("359 + 144 = 499 + 1 = 500 + 1 = 500...")
            # continues the expression *after* the matched "C" with another
            # operator or another "=" - in both cases the matched "C" is an
            # intermediate fragment, not the model's actual final claim for
            # this "line" (the real claim is later, or the line never
            # resolves to one). An operator or "=" immediately following
            # (skipping whitespace) means this match isn't a standalone
            # equation.
            suffix = line[m.end() :].lstrip()
            if suffix and (suffix[0] in _OPERATOR_CHARS or suffix[0] == "="):
                continue
            a_s, op_s, b_s, c_s = m.groups()
            if "." in a_s or "." in b_s or "." in c_s:
                continue  # cell80 plan IR here is integer-only; skip fractional spans
            spans.append({"a": int(a_s), "op": OP_MAP.get(op_s.lower(), op_s), "b": int(b_s), "c": int(c_s), "line": line})
    return spans


def verify_span(host: cell80_py.CellHost, span: dict) -> dict:
    if span["op"] not in ("add", "sub", "mul", "div"):
        return {**span, "verdict": "unparseable_op"}
    plan = {
        "quantities": [
            {"id": "x", "value": span["a"], "unit": "count"},
            {"id": "y", "value": span["b"], "unit": "count"},
        ],
        "ops": [[span["op"], "x", "y", "z"]],
        "target": "z",
    }
    try:
        rep = host.solve(json.dumps(plan), 2_000_000)
    except Exception as e:  # noqa: BLE001 - report, don't crash the batch
        return {**span, "verdict": "solve_error", "error": str(e)}
    answer = rep.get("answer")
    if answer is None:
        kill = rep["plans"][0].get("kill") if rep.get("plans") else None
        return {**span, "verdict": "escalated", "kill": kill}
    if int(answer) == span["c"]:
        return {**span, "verdict": "match", "cell80_answer": int(answer)}
    return {**span, "verdict": "mismatch", "cell80_answer": int(answer)}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--url", default="http://localhost:8080")
    ap.add_argument("--max-tokens", type=int, default=120)
    ap.add_argument("--limit", type=int, default=None, help="only run the first N battery problems")
    ap.add_argument("--timeout", type=int, default=600)
    args = ap.parse_args()

    host = cell80_py.CellHost()
    host.set_cache(True)

    battery = BATTERY[: args.limit] if args.limit else BATTERY
    t0 = time.time()
    rows = []
    for i, item in enumerate(battery):
        try:
            text = chat(args.url, item["q"], args.max_tokens, args.timeout)
        except requests.RequestException as e:
            print(f"[{i}] HTTP error: {e}", flush=True)
            rows.append({"question": item["q"], "final_expected": item["final"], "error": str(e), "spans": []})
            continue
        spans = extract_spans(text)
        verified = [verify_span(host, s) for s in spans]
        final_m = re.search(r"Answer:\s*(-?\d+(?:\.\d+)?)", text)
        final_stated = float(final_m.group(1)) if final_m else None
        rows.append({
            "question": item["q"],
            "final_expected": item["final"],
            "final_stated": final_stated,
            "final_correct": (final_stated is not None and float(final_stated) == float(item["final"])),
            "completion": text,
            "spans": verified,
        })
        n_match = sum(1 for s in verified if s["verdict"] == "match")
        print(f"[{i}] {len(verified)} spans, {n_match} matched, final_correct={rows[-1]['final_correct']}  ({time.time()-t0:.1f}s)", flush=True)

    all_spans = [s for r in rows for s in r["spans"]]
    n_total = len(all_spans)
    n_match = sum(1 for s in all_spans if s["verdict"] == "match")
    n_mismatch = sum(1 for s in all_spans if s["verdict"] == "mismatch")
    n_escalated = sum(1 for s in all_spans if s["verdict"] == "escalated")
    n_other = n_total - n_match - n_mismatch - n_escalated

    summary = {
        "url": args.url,
        "n_problems": len(battery),
        "n_spans": n_total,
        "n_match": n_match,
        "n_mismatch": n_mismatch,
        "n_escalated": n_escalated,
        "n_other": n_other,
        "agreement_rate": round(n_match / n_total, 3) if n_total else None,
        "wrong_number_rate": round(n_mismatch / n_total, 3) if n_total else None,
        "final_answer_accuracy": round(sum(r.get("final_correct", False) for r in rows) / len(rows), 3),
    }
    OUT.write_text(json.dumps({"summary": summary, "rows": rows}, indent=2))

    print("\n=== CN-2 slice-0 summary (baseline, no resampling) ===")
    print(json.dumps(summary, indent=2))
    print(f"\nwrote {OUT} ({time.time()-t0:.1f}s)")


if __name__ == "__main__":
    main()
