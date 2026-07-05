#!/usr/bin/env python3
"""First end-to-end 'cells calling cells': a model-written algorithm whose named
operations (max, gcd, is_gt, abs_diff, ...) are each RESOLVED to a verified library
cell by search+arity and RUN. The model supplies structure + naming; every named
operation executes as an already-verified cell. Reuse, not regeneration.

(Prototype: arithmetic glue is evaluated inline; the NAMED calls go through real
cells. The deeper move — compiling the whole thing to one composed cell via the
rustz80 syn AST — comes next.)
"""
import ast
import os
import re
import urllib.request
import json

LIB = os.environ.get("CELL_LIBRARY", "/Users/christopherhay/chris-source/cell80/cell80/cells")
from cell80_mcp.library import CellLibrary

lib = CellLibrary(LIB)
host = lib.host

OP = {ast.Add: lambda a, b: a + b, ast.Sub: lambda a, b: a - b,
      ast.Mult: lambda a, b: a * b, ast.Div: lambda a, b: a // b,
      ast.FloorDiv: lambda a, b: a // b}


def sig_arity(cid):
    try:
        m = host.manifest(cid)
    except Exception:
        return None
    if isinstance(m.get("params"), list):
        return len(m["params"])
    inner = re.search(r"\(([^)]*)\)", m.get("signature", ""))
    if not inner:
        return None
    s = inner.group(1).strip()
    return 0 if not s else s.count(",") + 1


def resolve(name, nargs):
    for _, m in host.search_scored(name, 6):
        cid = m["id"] if isinstance(m, dict) else m
        if sig_arity(cid) == nargs:
            return cid
    hits = host.search_scored(name, 1)
    return (hits[0][1]["id"] if hits else None)


def run_cell(cid, args):
    r = lib.run(cid, args)
    return r.get("result") if isinstance(r, dict) else r


class Composer:
    def __init__(self):
        self.env = {}
        self.trace = []

    def ev(self, node):
        if isinstance(node, ast.Constant):
            return int(node.value)
        if isinstance(node, ast.Name):
            return self.env[node.id]
        if isinstance(node, ast.BinOp):
            return OP[type(node.op)](self.ev(node.left), self.ev(node.right))
        if isinstance(node, ast.Call):
            name = node.func.id
            args = [self.ev(a) for a in node.args]
            cid = resolve(name, len(args))
            res = run_cell(cid, args) if cid else None
            self.trace.append((name, cid, args, res))
            return res
        raise ValueError(f"unsupported node {type(node).__name__}")

    def run(self, src):
        last = None
        for stmt in ast.parse(src).body:
            if isinstance(stmt, ast.Assign) and isinstance(stmt.targets[0], ast.Name):
                name = stmt.targets[0].id
                self.env[name] = self.ev(stmt.value)
                last = name
        return self.env.get("answer", self.env.get(last))


def show(title, src):
    print(f"\n### {title}")
    for ln in src.strip().splitlines():
        print(f"    {ln}")
    c = Composer()
    ans = c.run(src)
    print("  call resolution (named op -> verified cell -> result):")
    for name, cid, args, res in c.trace:
        print(f"    {name}{tuple(args)}  ->  cell '{cid}'  ->  {res}")
    print(f"  ANSWER = {ans}")
    return ans


# 1. hand-authored: a COMPARISON problem the arithmetic-only Plan IR could NOT express
show("comparison — bonus = 2 x larger profit (max is a library cell)", """
a = 340
b = 275
best = max(a, b)
answer = best * 2
""")

# 2. hand-authored: reuse the verified gcd cell to simplify a ratio
show("ratio — simplify 1071/462 by its gcd (gcd is a library cell)", """
num = 1071
den = 462
g = gcd(num, den)
answer = num / g
""")

# 3. real model: ask it to solve a comparison problem USING named ops
OLLAMA = "http://localhost:11434/v1/chat/completions"
MODEL = os.environ.get("BAKEOFF_MODEL", "qwen2.5:3b")
SYS = """Solve the problem with assignment lines. You MAY call these operations as functions:
max(x,y), min(x,y), gcd(x,y), abs_diff(x,y), is_gt(x,y). Use them for comparisons/choices.
Use integers only, one assignment per line, last line `answer = ...`. Output ONLY the lines."""
PROB = ("Two food trucks competed. The taco truck earned 480 dollars, the burger truck earned 350 dollars. "
        "The winner is whoever earned more. The festival pays the winner a bonus equal to the difference "
        "between the two trucks' earnings. What bonus does the winner get?")  # abs_diff = 130
try:
    body = {"model": MODEL, "temperature": 0, "stream": False,
            "messages": [{"role": "system", "content": SYS}, {"role": "user", "content": PROB}]}
    req = urllib.request.Request(OLLAMA, data=json.dumps(body).encode(), headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=120) as r:
        raw = json.loads(r.read())["choices"][0]["message"]["content"]
    raw = re.sub(r"```\w*", "", raw)
    src = "\n".join(l for l in raw.splitlines() if re.match(r"^\s*[A-Za-z_]\w*\s*=", l))
    show(f"MODEL ({MODEL}) solved with named ops (expect bonus 130)", src)
except Exception as e:
    print(f"\n(model step skipped: {e})")
