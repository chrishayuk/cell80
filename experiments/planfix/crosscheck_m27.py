#!/usr/bin/env python3
"""M2.7 — decorrelate the gate: a THIRD derivation with a different *reading*.

Same 20 pilot problems as `crosscheck_m26.py`, same in-compiler pipeline
(`cell80 compose` owns canonicalization, linking, and the gate), plus a third
derivation whose defining property is a different reading of the problem, not just
a different encoding (docs/math-campaign-amendment.md §M2.7):

  d0 inline    — solve with inline arithmetic only (original text)
  d1 composed  — solve by calling library ops (original text)
  d2 third     — default: deterministic paraphrase-then-extract on the same model
                 (the model rewrites the problem, numbers pinned, then solves the
                 REWRITE inline — a misread of the original phrasing has to survive
                 a second, differently-worded reading to be accepted);
                 or THIRD=model:<name> for a second-model reader.

The registered acceptance rule is already in the compose gate: 3-way agreement →
accept (`unanimous`); 2-of-3 → accept AND flag (`majority` — reported separately so
precision can be audited at both strictness levels); no majority → escalate.

Every generated source (and the paraphrase) is dumped per row for replay and the
error chase — diagnosis starts from captured evidence, not fresh generations.

  BAKEOFF_MODEL=gemma4:e4b python3 crosscheck_m27.py
  THIRD=model:granite4.1:3b BAKEOFF_MODEL=gemma4:e4b python3 crosscheck_m27.py
"""
import json
import os
import pathlib
import re
import subprocess
import sys
import tempfile
import urllib.request

REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "experiments" / "gsm8k-small-model-pilot"))
import gsm8k_small_model_pilot as pilot  # noqa: E402

OLLAMA = "http://localhost:11434/v1/chat/completions"
MODEL = os.environ.get("BAKEOFF_MODEL", "gemma4:e4b")
THIRD = os.environ.get("THIRD", "paraphrase")
# Method a second-model reader (THIRD=model:<name>) solves with. Default `inline`
# (unchanged); `composed` lets a model whose inline arm is weak but composed arm is
# strong (qwen) read in its reliable mode.
READER_METHOD = os.environ.get("READER_METHOD", "inline")
# FEWSHOT=1: prepend a fixed 8-shot demonstration (problem -> cell) plus two
# repair-shaped pairs (bad output + E-code in the USER turn, fix in the assistant
# turn — the assistant never authors the disease). Uniform across models/arms:
# a prompt-shape config, not a per-model schema. Every demonstration cell is
# verified through the real `cell80 compose` before use.
FEWSHOT = os.environ.get("FEWSHOT") == "1"
# FORMAT=rust (default): the model writes a dialect cell. FORMAT=equations: the
# model writes bare equation lines (`name = expr`, last line `answer = ...`) and
# the harness wraps them into a cell mechanically (line -> `let`, last name ->
# tail) — transport, not repair; canon owns everything else. The bakeoff's
# "arithmetic" rung, never yet measured on granite.
FORMAT = os.environ.get("FORMAT", "rust")
BIN = os.environ.get("CELL80_BIN", str(REPO / "target" / "release" / "cell80"))
CELLS = os.environ.get("CELL_LIBRARY", str(REPO / "cell80" / "cells"))
DUMP = pathlib.Path(os.environ.get(
    "DUMP_DIR", str(pathlib.Path(__file__).parent / "m27_sources" / MODEL.replace(":", "_"))))

BASE = """Write ONE Rust function `fn run() -> u16` that computes the numeric answer, then stop.
u16 integers only; NO floats/macros; multiply BEFORE dividing; `let`/`if`/`while`/`+ - * /`/comparisons ok;
call-arguments must be a plain name or literal; bake numbers in as literals; final expression (no ;) is the answer.
Output ONLY the code."""

METHODS = {
    "inline": " Solve using ONLY inline arithmetic and if/else. Do NOT call any named functions.",
    "composed": " Solve by CALLING named library operations (max, min, gcd, lcm, abs_diff, is_gt) wherever one fits, plus arithmetic. They resolve to verified cells.",
}

PARAPHRASE = """Rewrite this word problem in completely different wording and sentence order.
Keep every number and every quantitative relationship EXACTLY the same. Do not solve it.
Output ONLY the rewritten problem."""

BASE_EQ = """Write the solution as simple equation lines, ONE per line: name = expression.
Integers only; NO floats (write 90% as *9/10); multiply BEFORE dividing; + - * / and comparisons ok;
bake the problem's numbers in as literals; the LAST line must be: answer = <name or expression>.
Output ONLY the equation lines."""

METHODS_EQ = {
    "inline": " Use ONLY arithmetic expressions. Do NOT call any named functions.",
    "composed": " CALL named library operations (max, min, gcd, lcm, abs_diff, is_gt) as functions wherever one fits, plus arithmetic. They resolve to verified cells.",
}


def prompt_for(method):
    if FORMAT == "equations":
        return BASE_EQ + METHODS_EQ[method]
    return BASE + METHODS[method]


# ---- the fixed 8-shot demonstration (FEWSHOT=1) ----------------------------------
# 8 original problems (NOT from the 20-row test slice) covering the measured failure
# surface: multi-step chains, rates, the row89 ratio class (`total/(k+1)`), percent
# as integer fractions, exact-division let-chains, a legitimate if-value choice, a
# difference, and a comparison (the composed variant renders the last two as named
# calls). Each entry: (problem, rust_inline, rust_composed, eq_inline, eq_composed).
SHOTS = [
    ("Sara buys 3 boxes of 12 eggs and uses 7 of the eggs. How many eggs are left?",
     "fn run() -> u16 {\n    let boxes = 3;\n    let per_box = 12;\n    boxes * per_box - 7\n}",
     None,
     "boxes = 3\nper_box = 12\nanswer = boxes * per_box - 7",
     None),
    ("A printer prints 12 pages per minute. How many pages does it print in 7 minutes?",
     "fn run() -> u16 {\n    let pages_per_min = 12;\n    let minutes = 7;\n    pages_per_min * minutes\n}",
     None,
     "pages_per_min = 12\nminutes = 7\nanswer = pages_per_min * minutes",
     None),
    ("A farm has 4 times as many chickens as ducks. There are 60 birds in total. How many ducks are there?",
     "fn run() -> u16 {\n    let total_birds = 60;\n    let parts = 4 + 1;\n    total_birds / parts\n}",
     None,
     "total_birds = 60\nparts = 4 + 1\nanswer = total_birds / parts",
     None),
    ("A jacket costs 80 dollars. The price rises by 15%. What is the new price in dollars?",
     "fn run() -> u16 {\n    let base = 80;\n    base + base * 15 / 100\n}",
     None,
     "base = 80\nanswer = base + base * 15 / 100",
     None),
    ("240 cookies are packed into bags of 8, and each bag sells for 3 dollars. How many dollars in total?",
     "fn run() -> u16 {\n    let bags = 240 / 8;\n    let dollars = bags * 3;\n    dollars\n}",
     None,
     "bags = 240 / 8\ndollars = bags * 3\nanswer = dollars",
     None),
    ("Shipping costs 5 dollars for orders under 50 dollars, otherwise it is free. An order is 42 dollars. What is the shipping cost?",
     "fn run() -> u16 {\n    let order = 42;\n    if order < 50 { 5 } else { 0 }\n}",
     None,
     "order = 42\nanswer = if order < 50 { 5 } else { 0 }",
     None),
    ("Tom has 23 marbles and Ann has 41 marbles. How many more marbles does Ann have than Tom?",
     "fn run() -> u16 {\n    let ann = 41;\n    let tom = 23;\n    ann - tom\n}",
     "fn run() -> u16 {\n    let ann = 41;\n    let tom = 23;\n    abs_diff(ann, tom)\n}",
     "ann = 41\ntom = 23\nanswer = ann - tom",
     "ann = 41\ntom = 23\nanswer = abs_diff(ann, tom)"),
    ("Two teams scored 58 and 45 points. What is the higher score?",
     "fn run() -> u16 {\n    let a = 58;\n    let b = 45;\n    if a > b { a } else { b }\n}",
     "fn run() -> u16 {\n    let a = 58;\n    let b = 45;\n    max(a, b)\n}",
     "a = 58\nb = 45\nanswer = if a > b { a } else { b }",
     "a = 58\nb = 45\nanswer = max(a, b)"),
]

# Repair-shaped negative examples: the USER turn carries the broken attempt and the
# compiler's E-code; the assistant answers only with the fix. Targets granite's two
# measured diseases: verify-not-compute/`then`-sugar, and decimal floats.
REPAIR_SHOTS_RUST = [
    ('Problem: "A club has 5 shelves with 4 trophies each. How many trophies?"\n'
     'A previous attempt was:\nfn run() -> u16 {\nif 5 * 4 == 20 then 20 else 0\n}\n'
     'The compiler rejected it: [E0501 parse] `then` is not Rust — and a self-check '
     'computes nothing. Derive the value directly. Write the corrected function.',
     "fn run() -> u16 {\n    let shelves = 5;\n    let per_shelf = 4;\n    shelves * per_shelf\n}"),
    ('Problem: "A 90-dollar item is discounted by 10%. What is the final price in dollars?"\n'
     'A previous attempt was:\nfn run() -> u16 {\n(90.0 * 0.9) as u16\n}\n'
     'The compiler rejected it: [E0304 requires_fractional_scale] float literal — '
     'write fractions as integer multiply-then-divide. Write the corrected function.',
     "fn run() -> u16 {\n    let price = 90;\n    price * 9 / 10\n}"),
]
REPAIR_SHOTS_EQ = [
    ('Problem: "A club has 5 shelves with 4 trophies each. How many trophies?"\n'
     'A previous attempt was:\ncheck = 5 * 4 == 20\n'
     'That verifies a guess instead of deriving the value. Write the corrected equations.',
     "shelves = 5\nper_shelf = 4\nanswer = shelves * per_shelf"),
    ('Problem: "A 90-dollar item is discounted by 10%. What is the final price in dollars?"\n'
     'A previous attempt was:\nanswer = 90.0 * 0.9\n'
     'Floats are rejected — write fractions as integer multiply-then-divide. Write the corrected equations.',
     "price = 90\nanswer = price * 9 / 10"),
]


def shot_messages(method):
    """The fixed demonstration as real chat pairs — the benchmark few-shot format."""
    if not FEWSHOT:
        return []
    msgs = []
    for prob, r_in, r_co, e_in, e_co in SHOTS:
        if FORMAT == "equations":
            cell = (e_co if method == "composed" else e_in) or e_in
        else:
            cell = (r_co if method == "composed" else r_in) or r_in
        msgs.append({"role": "user", "content": f'Problem: "{prob}"'})
        msgs.append({"role": "assistant", "content": cell})
    repairs = REPAIR_SHOTS_EQ if FORMAT == "equations" else REPAIR_SHOTS_RUST
    for user, fix in repairs:
        msgs.append({"role": "user", "content": user})
        msgs.append({"role": "assistant", "content": fix})
    return msgs


def ask(system, user, model=MODEL, shots=None):
    messages = ([{"role": "system", "content": system}]
                + (shots or [])
                + [{"role": "user", "content": user}])
    body = {"model": model, "temperature": 0, "stream": False, "messages": messages}
    req = urllib.request.Request(OLLAMA, data=json.dumps(body).encode(),
                                 headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=300) as r:
        return json.loads(r.read())["choices"][0]["message"]["content"]


def to_fn(text):
    """Extraction only (fences, bare-body wrap) — repairs belong to the compiler."""
    text = re.sub(r"```\w*", "", text).strip()
    if "fn " in text:
        i, j = text.find("fn "), text.rfind("}")
        return text[i:j + 1] if j > i else None
    lines = [l for l in text.splitlines() if l.strip() and not l.strip().startswith(("//", "#"))]
    return "fn run() -> u16 {\n" + "\n".join(lines) + "\n}" if lines else None


EQ_LINE = re.compile(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(?!=)(.+?)\s*$")


def eq_to_fn(text):
    """Equation lines -> a cell, mechanically: each `name = expr` becomes a `let`,
    the last assigned name becomes the tail. Transport, not repair — non-assignment
    lines (prose, fences, bare stated answers) are simply not equations, and canon
    (SSA rebind, folding, checked lane, linking) owns everything downstream."""
    text = re.sub(r"```\w*", "", text)
    lets, last = [], None
    for line in text.splitlines():
        m = EQ_LINE.match(line)
        if not m:
            continue
        name, expr = m.groups()
        lets.append(f"    let {name} = {expr};")
        last = name
    if last is None:
        return None
    return "fn run() -> u16 {\n" + "\n".join(lets) + f"\n    {last}\n}}"


def extract(text):
    return eq_to_fn(text) if FORMAT == "equations" else to_fn(text)


def third_derivation(problem, row_dir):
    if THIRD.startswith("model:"):
        reader = THIRD.split(":", 1)[1]
        return extract(ask(prompt_for(READER_METHOD), f'Problem: "{problem}"',
                           model=reader, shots=shot_messages(READER_METHOD)))
    rewrite = ask(PARAPHRASE, f'Problem: "{problem}"').strip()
    (row_dir / "paraphrase.txt").write_text(rewrite)
    return extract(ask(prompt_for("inline"), f'Problem: "{rewrite}"',
                       shots=shot_messages("inline")))


def compose(sources):
    with tempfile.TemporaryDirectory() as d:
        paths = []
        for i, src in enumerate(sources):
            p = pathlib.Path(d) / f"d{i}.rs"
            p.write_text(src)
            paths.append(str(p))
        r = subprocess.run([BIN, "compose", CELLS, *paths, "--json"],
                           capture_output=True, text=True, timeout=90)
        if r.returncode != 0:
            return {"answer": None, "agreement": "escalate",
                    "derivations": [{"kill": r.stderr.strip() or r.stdout.strip()}]}
        return json.loads(r.stdout)


def run():
    third_desc = THIRD if THIRD != "paraphrase" else "paraphrase-then-extract (same model)"
    if THIRD.startswith("model:"):
        third_desc += f" (reads via {READER_METHOD})"
    cfg = f"format = {FORMAT}   fewshot = {'8+2' if FEWSHOT else 'off'}"
    print(f"model = {MODEL}   third reader = {third_desc}   {cfg}   gate = 2-of-3 registered rule\n")
    print(f"{'row':16s} {'want':>6s} {'got':>7s} {'gate':>10s}  verdict")
    print("-" * 78)
    uni_ok = uni_bad = maj_ok = maj_bad = esc_recoverable = esc_genuine = 0
    repair_tally = {}
    limit = int(os.environ.get("PILOT_LIMIT", "0")) or len(pilot.PROBLEMS)
    for name, problem, exp in pilot.PROBLEMS[:limit]:
        row_dir = DUMP / name
        row_dir.mkdir(parents=True, exist_ok=True)
        srcs = [
            extract(ask(prompt_for("inline"), f'Problem: "{problem}"',
                        shots=shot_messages("inline"))) or "fn broken(",
            extract(ask(prompt_for("composed"), f'Problem: "{problem}"',
                        shots=shot_messages("composed"))) or "fn broken(",
            third_derivation(problem, row_dir) or "fn broken(",
        ]
        for i, s in enumerate(srcs):
            (row_dir / f"d{i}.rs").write_text(s)
        rep = compose(srcs)
        (row_dir / "report.json").write_text(json.dumps(rep, indent=1))
        got, gate = rep.get("answer"), rep.get("agreement")
        for dv in rep.get("derivations", []):
            for r_ in dv.get("repairs", []) or []:
                code = r_.split(":", 1)[0]
                repair_tally[code] = repair_tally.get(code, 0) + 1
        answers = [dv.get("answer") for dv in rep.get("derivations", [])]
        if got is not None and gate == "unanimous":
            ok = got == exp
            uni_ok += ok
            uni_bad += not ok
            verdict = "ACCEPT ok" if ok else "ACCEPT WRONG(!)"
        elif got is not None and gate == "majority":
            ok = got == exp
            maj_ok += ok
            maj_bad += not ok
            verdict = ("ACCEPT ok" if ok else "ACCEPT WRONG(!)") + " [flagged 2-of-3]"
        else:
            recoverable = exp in [a for a in answers if a is not None]
            esc_recoverable += recoverable
            esc_genuine += not recoverable
            verdict = "escalate" + (" (recoverable)" if recoverable else " (genuine)")
            kills = "; ".join(filter(None, (dv.get("kill") for dv in rep.get("derivations", []))))
            if kills:
                verdict += f"  [{kills[:60]}]"
        print(f"{name:16s} {exp:>6d} {str(got):>7s} {str(gate):>10s}  {verdict}", flush=True)
    n = limit
    acc = uni_ok + uni_bad + maj_ok + maj_bad
    ok_all = uni_ok + maj_ok
    print("-" * 78)
    print(f"accepted {acc}/{n}   yield {ok_all}/{n}   escalations: "
          f"{esc_recoverable} recoverable, {esc_genuine} genuine")
    print(f"precision strict (unanimous only): {uni_ok}/{uni_ok + uni_bad}"
          f"   with majority: {ok_all}/{acc}")
    print(f"SAFETY: accepted-but-wrong = {uni_bad + maj_bad} (unanimous {uni_bad}, majority {maj_bad})")
    print(f"repairs applied (E-code tally): {repair_tally or 'none'}")
    print(f"sources dumped under {DUMP}")


if __name__ == "__main__":
    run()
