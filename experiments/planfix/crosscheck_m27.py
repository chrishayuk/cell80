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


def ask(system, user, model=MODEL):
    body = {"model": model, "temperature": 0, "stream": False,
            "messages": [{"role": "system", "content": system},
                         {"role": "user", "content": user}]}
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


def third_derivation(problem, row_dir):
    if THIRD.startswith("model:"):
        reader = THIRD.split(":", 1)[1]
        return to_fn(ask(BASE + METHODS["inline"], f'Problem: "{problem}"', model=reader))
    rewrite = ask(PARAPHRASE, f'Problem: "{problem}"').strip()
    (row_dir / "paraphrase.txt").write_text(rewrite)
    return to_fn(ask(BASE + METHODS["inline"], f'Problem: "{rewrite}"'))


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
    print(f"model = {MODEL}   third reader = {third_desc}   gate = 2-of-3 registered rule\n")
    print(f"{'row':16s} {'want':>6s} {'got':>7s} {'gate':>10s}  verdict")
    print("-" * 78)
    uni_ok = uni_bad = maj_ok = maj_bad = esc_recoverable = esc_genuine = 0
    repair_tally = {}
    limit = int(os.environ.get("PILOT_LIMIT", "0")) or len(pilot.PROBLEMS)
    for name, problem, exp in pilot.PROBLEMS[:limit]:
        row_dir = DUMP / name
        row_dir.mkdir(parents=True, exist_ok=True)
        srcs = [
            to_fn(ask(BASE + METHODS["inline"], f'Problem: "{problem}"')) or "fn broken(",
            to_fn(ask(BASE + METHODS["composed"], f'Problem: "{problem}"')) or "fn broken(",
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
