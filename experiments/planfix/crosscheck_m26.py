#!/usr/bin/env python3
"""M2.8 item 1 — the registered M2.6 prediction check, on the NEW pipeline.

Same 20 pilot problems, same two-derivation cross-check (inline vs library-composed,
temp 0), but canonicalize + link + gate now live in the compiler: each pair of model
sources goes through `cell80 compose <cells> inline.rs composed.rs --json`, which
applies Full canonicalization (defer-division, constant folding, width), resolves
library calls (E0504-cued, confidence-floored), runs both derivations, and applies
the registered agreement gate.

The old harness's Python `autofix()` (macro-strip, paren-collapse) is deliberately
GONE — those repairs moved into the compiler's dialect normalizer, and this script
measures exactly that migration. Extraction (code-fence stripping, bare-body wrap)
stays harness-side: that's transport, not repair.

Registered prediction (docs/math-campaign-amendment.md §M2.6): on gemma4:e4b,
yield 80% -> >=90% at unchanged 100% precision — the two mechanical escalations
(row89 width, row93 trailing-let) recover; the two comprehension escalations
(row86, row101) correctly persist. If precision moves, the pass is defective and
gets reverted.

  BAKEOFF_MODEL=gemma4:e4b python3 crosscheck_m26.py
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
BIN = os.environ.get("CELL80_BIN", str(REPO / "target" / "release" / "cell80"))
CELLS = os.environ.get("CELL_LIBRARY", str(REPO / "cell80" / "cells"))

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
    with urllib.request.urlopen(req, timeout=300) as r:
        return json.loads(r.read())["choices"][0]["message"]["content"]


def to_fn(text):
    """Extraction only (fences, bare-body wrap) — repairs belong to the compiler now."""
    text = re.sub(r"```\w*", "", text).strip()
    if "fn " in text:
        i, j = text.find("fn "), text.rfind("}")
        return text[i:j + 1] if j > i else None
    lines = [l for l in text.splitlines() if l.strip() and not l.strip().startswith(("//", "#"))]
    return "fn run() -> u16 {\n" + "\n".join(lines) + "\n}" if lines else None


def compose(sources):
    """Run N derivation sources through `cell80 compose` — returns the report dict."""
    with tempfile.TemporaryDirectory() as d:
        paths = []
        for i, src in enumerate(sources):
            p = pathlib.Path(d) / f"d{i}.rs"
            p.write_text(src)
            paths.append(str(p))
        r = subprocess.run([BIN, "compose", CELLS, *paths, "--json"],
                           capture_output=True, text=True, timeout=60)
        if r.returncode != 0:
            return {"answer": None, "agreement": "escalate",
                    "derivations": [{"kill": r.stderr.strip() or r.stdout.strip()}]}
        return json.loads(r.stdout)


def run():
    print(f"model = {MODEL}   cross-check via `cell80 compose` (M2.5+M2.6 in-compiler)\n")
    print(f"{'row':16s} {'want':>6s} {'got':>7s} {'gate':>10s}  verdict")
    print("-" * 78)
    acc_ok = acc_bad = esc_recoverable = esc_genuine = 0
    repair_tally = {}
    limit = int(os.environ.get("PILOT_LIMIT", "0")) or len(pilot.PROBLEMS)
    for name, problem, exp in pilot.PROBLEMS[:limit]:
        srcs = []
        for method in ("inline", "composed"):
            fn = to_fn(ask(BASE + METHODS[method], problem))
            srcs.append(fn or "fn broken(")  # unparseable -> typed parse kill downstream
        rep = compose(srcs)
        got, gate = rep.get("answer"), rep.get("agreement")
        for dv in rep.get("derivations", []):
            for r_ in dv.get("repairs", []) or []:
                code = r_.split(":", 1)[0]
                repair_tally[code] = repair_tally.get(code, 0) + 1
        answers = [dv.get("answer") for dv in rep.get("derivations", [])]
        if got is not None and gate in ("unanimous", "majority"):
            ok = got == exp
            acc_ok += ok
            acc_bad += not ok
            verdict = "ACCEPT ok" if ok else "ACCEPT WRONG(!)"
        else:
            recoverable = exp in [a for a in answers if a is not None]
            esc_recoverable += recoverable
            esc_genuine += not recoverable
            verdict = "escalate" + (" (recoverable)" if recoverable else " (genuine)")
            kills = "; ".join(filter(None, (dv.get("kill") for dv in rep.get("derivations", []))))
            if kills:
                verdict += f"  [{kills[:60]}]"
        print(f"{name:16s} {exp:>6d} {str(got):>7s} {str(gate):>10s}  {verdict}")
    n = limit
    accepted = acc_ok + acc_bad
    print("-" * 78)
    print(f"accepted {accepted}/{n}   precision {acc_ok}/{accepted}   yield {acc_ok}/{n}"
          f"   escalations: {esc_recoverable} recoverable, {esc_genuine} genuine")
    print(f"SAFETY: accepted-but-wrong = {acc_bad} (must be 0)")
    print(f"repairs applied (E-code tally): {repair_tally or 'none'}")
    print("\nregistered prediction (gemma4): yield 80% -> >=90% at unchanged 100% precision")


if __name__ == "__main__":
    run()
