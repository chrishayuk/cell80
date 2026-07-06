#!/usr/bin/env python3
"""M2.8 item 2 — the PAL-Python baseline (H-M2: cells accuracy >= PAL accuracy).

Same 20 pilot problems, same models, temp 0: the model emits a Python function,
the harness executes it in a subprocess with a timeout and takes the PRINTED
result of calling it — the standard PAL shape. One derivation, no gate, no
verification: the answer is whatever the code computes, graded right-or-wrong.
This is the "structurally PAL with a different executor" comparison the amended
plan says cannot be deferred (docs/math-campaign-amendment.md §M2.8).

Note what this baseline does NOT have: no agreement gate, no typed escalation,
no precision guarantee — a wrong PAL answer is silent. Report accuracy AND how
many of its errors were silent (all of them, by construction, minus crashes).

  BAKEOFF_MODEL=gemma4:e4b python3 pal_baseline.py
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

SYSTEM = """Write ONE Python function `def solution():` that computes and returns the numeric answer.
Plain arithmetic only; no imports, no input(), no comments needed. Output ONLY the code."""


def ask(problem):
    body = {"model": MODEL, "temperature": 0, "stream": False,
            "messages": [{"role": "system", "content": SYSTEM},
                         {"role": "user", "content": f'Problem: "{problem}"'}]}
    req = urllib.request.Request(OLLAMA, data=json.dumps(body).encode(),
                                 headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=300) as r:
        return json.loads(r.read())["choices"][0]["message"]["content"]


def extract(text):
    text = re.sub(r"```\w*", "", text).strip()
    i = text.find("def ")
    return text[i:] if i >= 0 else None


def run_python(code):
    """Execute in a subprocess (timeout, no shared state); print(solution())."""
    prog = code + "\n\nprint(solution())\n"
    with tempfile.NamedTemporaryFile("w", suffix=".py", delete=False) as f:
        f.write(prog)
        path = f.name
    try:
        r = subprocess.run([sys.executable, "-I", path], capture_output=True,
                           text=True, timeout=10)
        if r.returncode != 0:
            return None, (r.stderr.strip().splitlines() or ["error"])[-1][:60]
        out = r.stdout.strip().splitlines()
        if not out:
            return None, "no output"
        v = float(out[-1])
        return (int(v) if v == int(v) else v), None
    except subprocess.TimeoutExpired:
        return None, "timeout"
    except ValueError:
        return None, "non-numeric output"
    finally:
        os.unlink(path)


def run():
    print(f"model = {MODEL}   PAL-Python baseline (one derivation, no gate)\n")
    right = wrong = crashed = 0
    for name, problem, exp in pilot.PROBLEMS:
        code = extract(ask(problem))
        if code is None:
            crashed += 1
            print(f"{name:16s} want {exp:>6d}  no code")
            continue
        got, err = run_python(code)
        if got == exp:
            right += 1
            verdict = "ok"
        elif got is None:
            crashed += 1
            verdict = f"crash [{err}]"
        else:
            wrong += 1
            verdict = "WRONG (silent)"
        print(f"{name:16s} want {exp:>6d}  got {str(got):>8s}  {verdict}", flush=True)
    n = len(pilot.PROBLEMS)
    print("-" * 60)
    print(f"accuracy {right}/{n} = {right/n:.0%}   silent-wrong {wrong}   crashed {crashed}")
    print("(every wrong PAL answer is silent by construction — no gate, no escalation)")


if __name__ == "__main__":
    run()
