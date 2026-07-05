#!/usr/bin/env python3
"""Prompt-time retrieval — 'we already have this in our cells'. For each problem we
SEARCH the library (the cells' own retrieval) and hand the model the REAL cell
signatures; it calls them by EXACT name. Then link -> compile -> run one verified
cell. This uses retrieval where it's strongest (before generation), so linking is
exact and the model sees u16 signatures (no width guessing).
"""
import json
import os
import pathlib
import re
import sys
import urllib.request

from compose_link import execute, host, link_and_compile

REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "experiments" / "gsm8k-small-model-pilot"))
import gsm8k_small_model_pilot as pilot  # noqa: E402

OLLAMA = "http://localhost:11434/v1/chat/completions"
MODEL = os.environ.get("BAKEOFF_MODEL", "qwen2.5:3b")
# a small always-visible toolbox of common ops (the "standard library")
CORE = ["max", "min", "gcd", "lcm", "abs_diff", "is_gt", "is_ge"]


def present(m):
    sig = re.sub(r"\brun\b", m["id"], m.get("signature", ""), count=1)
    return f"  {sig}   // {m.get('summary', '')}"


def available_cells(problem, k=8):
    seen = {}
    for _, m in host.search_scored(problem, k):
        if isinstance(m, dict):
            seen[m["id"]] = m
    for c in CORE:
        if c not in seen:
            try:
                man = host.manifest(c)
                man.setdefault("id", c)
                seen[c] = man
            except Exception:
                pass
    # only u16, 2-arg-ish scalar ops are safe to offer here
    return [m for m in seen.values() if "u32" not in m.get("signature", "")]


def build_sys(cells):
    listing = "\n".join(present(m) for m in cells)
    return f"""Write ONE Rust function `fn run() -> u16` that computes the numeric answer, then stop.
Rules: u16 integers only; NO floats; multiply BEFORE you divide (integer division truncates);
`let`, `if/else`, `while`, `+ - * /`, comparisons allowed; bake the problem's numbers in as u16
literals; the final expression (no semicolon) is the answer.
You MAY call these ALREADY-VERIFIED library functions by their EXACT name:
{listing}
Output ONLY the code — no markdown fences, no prose.

Example:
fn run() -> u16 {{
    let a = 480;
    let b = 350;
    abs_diff(a, b)
}}"""


def ask(system, problem):
    body = {"model": MODEL, "temperature": 0, "stream": False,
            "messages": [{"role": "system", "content": system},
                         {"role": "user", "content": f'Problem: "{problem}"'}]}
    req = urllib.request.Request(OLLAMA, data=json.dumps(body).encode(),
                                headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=180) as r:
        return json.loads(r.read())["choices"][0]["message"]["content"]


def extract_rust(text):
    text = re.sub(r"```\w*", "", text).strip()
    i = text.find("fn ")
    j = text.rfind("}")
    return text[i:j + 1] if (i != -1 and j > i) else None


def run():
    print(f"model = {MODEL}  (prompt-time retrieval → exact calls → link → run)\n")
    correct = used_cell = 0
    for name, problem, exp in pilot.PROBLEMS:
        cells = available_cells(problem)
        src = extract_rust(ask(build_sys(cells), problem))
        if not src:
            print(f"{name:16s} want={exp:<5} no-code")
            continue
        res = link_and_compile(src)
        ans = execute(res["cell"]) if res["ok"] else None
        links = [f"{n}->{c}" for n, c in res.get("resolutions", [])]
        # did the model call any library op (linked OR a builtin like gcd)?
        called = re.findall(r"\b([a-z_][a-z0-9_]*)\s*\(", src)
        libcalls = [c for c in called if c in CORE or c in [r[0] for r in res.get("resolutions", [])] or c == "gcd"]
        used_cell += bool(libcalls)
        ok = ans == exp
        correct += ok
        note = ("links=" + ",".join(links)) if links else ("calls=" + ",".join(set(libcalls)) if libcalls else "")
        if not res["ok"]:
            note = "COMPILE-FAIL " + note
        print(f"{name:16s} want={str(exp):<5} got={str(ans):<6} {'ok' if ok else '  '}  {note}")
    n = len(pilot.PROBLEMS)
    print(f"\ncorrect: {correct}/{n} ({100*correct//n}%)   |   problems that called a library op: {used_cell}/{n}")


if __name__ == "__main__":
    run()
