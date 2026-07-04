"""Behavioural snapshot / diff for a cell library — "these cells behave identically
across a source change", as a measurement.

The codegen golden (`cell80/tests/golden/`) pins *bytes*; this pins *behaviour*: every
cell is compiled and run over a deterministic, edge-heavy input battery (per arity;
state cells driven by field name), and two snapshots compare equal only if every output,
halt, and post-run state matches and no signature changed. It exists because source
"improvements" can silently change semantics — the 2026-07 modernization pass caught a
clamp rewrite that flipped which bound wins on inverted ranges, an error no compile or
eyeball catches.

Pairs with the admission gate (roadmap 2.2) for library growth (2.3): the gate asks
"is a NEW cell retrievable?"; this asks "did EXISTING cells keep their contract?" —
run it around any pass that edits `cell80/cells` sources.

    cell-eval cells-snapshot --out before.json
    # ... edit cell sources ...
    cell-eval cells-snapshot --out after.json
    cell-eval cells-compare before.json after.json     # exit 1 on any divergence
"""

from __future__ import annotations

import json
import pathlib
from dataclasses import dataclass, field

from .library import open_library

# The edge-heavy value set: 0/1/2 (degenerate), small primes and composites, byte and
# sign boundaries, u16 extremes. Deterministic — no RNG, so snapshots diff cleanly.
VALUES = [0, 1, 2, 3, 7, 10, 16, 100, 255, 256, 1000, 32767, 32768, 65534, 65535]


def battery(arity: int) -> list[list[int]]:
    """Argument sets for a value cell of the given arity — full grid for 1–2 args,
    a strided grid plus known-tricky rows for 3+ (bounded, still edge-heavy)."""
    if arity == 0:
        return [[]]
    if arity == 1:
        return [[v] for v in VALUES]
    if arity == 2:
        grid = [[a, b] for a in VALUES[::2] for b in VALUES[::2]]
        grid += [[3, 7], [7, 3], [48, 36], [65535, 1], [1, 65535], [0, 0]]
        return grid
    out = []
    for a in VALUES[::4]:
        for b in VALUES[::4]:
            for c in VALUES[::4]:
                out.append(([a, b, c] + [5] * arity)[:arity])
    out += [[5, 1, 10] + [5] * (arity - 3), [0, 65535, 1] + [2] * (arity - 3)]
    return out


def snapshot(library_dir: str | None = None) -> dict:
    """Compile every cell in the library and record its behaviour over the battery.
    Returns `{cell_id: {"signature": ..., "outputs": {args_json: [result, halt, ...]}}}`
    — value cells record `[result, halt]`, state cells `[result, halt, state]` with
    every field driven uniformly per battery value."""
    lib = open_library(library_dir)
    snap: dict = {}
    for m in lib.list():
        cid = m["id"]
        outputs: dict = {}
        state_fields = [n for n, _ in (m.get("state") or [])]
        if state_fields:
            for v in VALUES[::3]:
                fields = {n: v for n in state_fields}
                r = lib.run_state(cid, fields)
                key = json.dumps(fields, sort_keys=True)
                outputs[key] = [r["result"], r["halt"], r.get("state")]
        else:
            for args in battery(len(m.get("params") or [])):
                r = lib.run(cid, args)
                outputs[json.dumps(args)] = [r["result"], r["halt"]]
        snap[cid] = {"signature": m["signature"], "outputs": outputs}
    return snap


@dataclass
class DiffReport:
    """The comparison verdict: identical iff every list is empty."""

    cells: int = 0
    divergent: list[dict] = field(default_factory=list)  # {cell, inputs: [...]}
    signature_changed: list[str] = field(default_factory=list)
    missing: list[str] = field(default_factory=list)  # in before, not after
    added: list[str] = field(default_factory=list)  # in after, not before

    @property
    def identical(self) -> bool:
        return not (self.divergent or self.signature_changed or self.missing or self.added)

    def as_dict(self) -> dict:
        return {"identical": self.identical, **self.__dict__}

    def render(self) -> str:
        if self.identical:
            return f"OK — {self.cells} cells behave identically on the battery."
        lines = ["FAIL — behaviour is not identical:"]
        for d in self.divergent:
            shown = ", ".join(d["inputs"][:3])
            more = f" (+{len(d['inputs']) - 3} more)" if len(d["inputs"]) > 3 else ""
            lines.append(f"  {d['cell']}: output divergence on {shown}{more}")
        lines += [f"  {c}: signature changed" for c in self.signature_changed]
        lines += [f"  {c}: missing after the change" for c in self.missing]
        lines += [f"  {c}: new cell appeared" for c in self.added]
        return "\n".join(lines)


def compare(before: dict, after: dict) -> DiffReport:
    """Diff two [`snapshot`]s. Any output/halt/state difference on any battery row is a
    divergence — the contract is byte-identical behaviour, not "close"."""
    rep = DiffReport(cells=len(before))
    for cid in sorted(before):
        if cid not in after:
            rep.missing.append(cid)
            continue
        b, a = before[cid], after[cid]
        if b["signature"] != a["signature"]:
            rep.signature_changed.append(cid)
        bad = [k for k in b["outputs"] if b["outputs"][k] != a["outputs"].get(k)]
        if bad:
            rep.divergent.append({"cell": cid, "inputs": bad})
    rep.added = sorted(set(after) - set(before))
    return rep


def load_snapshot(path: str | pathlib.Path) -> dict:
    return json.loads(pathlib.Path(path).read_text())


def save_snapshot(snap: dict, path: str | pathlib.Path) -> None:
    pathlib.Path(path).write_text(json.dumps(snap, indent=0, sort_keys=True) + "\n")
