#!/usr/bin/env python3
"""The integration: 'cells calling cells', compiled to ONE verified cell.

Drive rustz80's own name-resolution as a fuzzy linker. Compile the model's source;
when it errors `unknown call target \`X\``, that's an unresolved symbol — SEARCH the
library for X (fuzzy: `maximum`/`pick_bigger` -> the `max` cell), pull that cell's
source, rename its entry `fn run` -> `fn X`, append, and recompile. Loop until it
compiles. Builtins (e.g. gcd) just resolve; everything else links from the library.
Result: a single re-checkable cell that composes verified cells. Zero compiler edits.
"""
import os
import re
import subprocess
import tempfile

WT = "/Users/christopherhay/chris-source/cell80-planfix"
WBIN = os.environ.get("CELL80_BIN", f"{WT}/target/release/cell80")
CELLS = os.environ.get("CELL_LIBRARY", f"{WT}/cell80/cells")

from cell80_mcp.library import CellLibrary

lib = CellLibrary(CELLS)
host = lib.host

UNKNOWN_RE = re.compile(r"unknown call target `([^`]+)`")


def sig_arity(cid):
    try:
        m = host.manifest(cid)
    except Exception:
        return None
    if isinstance(m.get("params"), list):
        return len(m["params"])
    inner = re.search(r"\(([^)]*)\)", m.get("signature", ""))
    return None if not inner else (0 if not inner.group(1).strip() else inner.group(1).count(",") + 1)


def resolve(name, nargs):
    """fuzzy name -> library cell id, preferring the overload whose arity matches."""
    hits = host.search_scored(name, 6)
    ids = [(m["id"] if isinstance(m, dict) else m) for _, m in hits]
    for cid in ids:
        if sig_arity(cid) == nargs:
            return cid
    return ids[0] if ids else None


def call_arity(src, name):
    m = re.search(re.escape(name) + r"\s*\(([^)]*)\)", src)
    if not m:
        return None
    inner = m.group(1).strip()
    return 0 if not inner else inner.count(",") + 1


def cell_fn_source(cell_id, as_name):
    path = os.path.join(CELLS, f"{cell_id}.rs")
    with open(path) as f:
        body = "".join(l for l in f if not l.lstrip().startswith("//!"))
    return re.sub(r"\bfn\s+run\b", f"fn {as_name}", body.strip(), count=1)


def compile_cell(src):
    rs = os.path.join(tempfile.gettempdir(), "_link.rs")
    cell = os.path.join(tempfile.gettempdir(), "_link.cell")
    with open(rs, "w") as f:
        f.write(src)
    r = subprocess.run([WBIN, "compile", rs, "-o", cell, "--id", "linked"],
                       capture_output=True, text=True, timeout=30)
    return (r.returncode == 0 and "wrote" in r.stdout), r.stdout + r.stderr, cell


def link_and_compile(model_src, max_iter=12):
    src, resolutions = model_src, []
    for _ in range(max_iter):
        ok, out, cell = compile_cell(src)
        if ok:
            funcs = re.search(r"code:\s*(\d+) bytes,\s*(\d+) functions", out)
            return {"ok": True, "src": src, "resolutions": resolutions,
                    "cell": cell, "size": funcs.group(0) if funcs else "?"}
        m = UNKNOWN_RE.search(out)
        if not m or m.group(1) in [r[0] for r in resolutions]:
            return {"ok": False, "src": src, "resolutions": resolutions, "err": out.strip().splitlines()[-1]}
        name = m.group(1)
        cid = resolve(name, call_arity(src, name))
        if not cid:
            return {"ok": False, "resolutions": resolutions, "err": f"no library match for {name}"}
        src += "\n\n" + cell_fn_source(cid, name)
        resolutions.append((name, cid))
    return {"ok": False, "resolutions": resolutions, "err": "max iterations"}


def execute(cell):
    r = subprocess.run([WBIN, "exec", cell], capture_output=True, text=True, timeout=30)
    m = re.search(r"result\s+(\d+)", r.stdout)
    return int(m.group(1)) if m else None


def demo(title, model_src, expect):
    print(f"\n### {title}")
    res = link_and_compile(model_src)
    for name, cid in res["resolutions"]:
        print(f"    unknown call `{name}`  ->  linked library cell '{cid}'")
    if res["ok"]:
        ans = execute(res["cell"])
        print(f"    compiled to ONE cell [{res['size']}]  ->  exec = {ans}  "
              f"{'ok' if ans == expect else '✗ want ' + str(expect)}")
    else:
        print(f"    FAILED: {res['err']}")


if __name__ == "__main__":
    demo("fuzzy `maximum` -> max cell; bonus = 2 x larger",
         "fn run() -> u16 {\n    maximum(340, 275) * 2\n}", 680)

    demo("two fuzzy calls: pick_bigger/smaller -> max/min",
         "fn run() -> u16 {\n    let hi = pick_bigger(48, 42);\n    let lo = smaller(48, 42);\n    hi - lo\n}", 6)

    demo("abs difference of two truck earnings",
         "fn run() -> u16 {\n    absolute_difference(480, 350)\n}", 130)

    demo("builtin gcd + linked lcm compose in one cell",
         "fn run() -> u16 {\n    let g = gcd(12, 8);\n    let l = least_common_multiple(4, 6);\n    g + l\n}", 16)
