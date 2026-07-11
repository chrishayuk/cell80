"""The warm cell library — a thin Python wrapper over the PyO3 `cell80_py.CellHost`.

It loads a directory of cells once (`.rs` sources, metadata from a `//!` header; or
precompiled `.cell` cartridges), then serves `search` / `inspect` / `run`. `run` hides
handles: it lazily `load`s a warm runner per id and reuses it, so the model just names a
tool + args and the host keeps it warm across calls.
"""

from __future__ import annotations

import json
import pathlib

import cell80_py


def _parse_header(src: str) -> tuple[str, list[str], str | None, list[str], bool]:
    """Read a cell source's leading `//!` header → (summary, tags, entry, limits,
    kernel_bank). `limits` is the escalation contract: what the cell declares it
    can't do (`//! limits: floats, inputs > 65535`); `kernel_bank` compiles the
    cell against the resident softfloat bank (`//! kernel_bank: on`)."""
    summary, tags, entry, limits, kernel_bank = "", [], None, [], False
    for line in src.splitlines():
        s = line.strip()
        if s.startswith("//!"):
            r = s[3:].strip()
            if r.startswith("tags:"):
                tags = [t.strip() for t in r[5:].split(",") if t.strip()]
            elif r.startswith("entry:"):
                entry = r[6:].strip()
            elif r.startswith("limits:"):
                limits = [x.strip() for x in r[7:].split(",") if x.strip()]
            elif r.startswith("kernel_bank:"):
                kernel_bank = r[12:].strip() in ("on", "true", "1")
            elif not summary:
                summary = r
        elif s and not s.startswith("//"):
            break  # first code line — header done
    return summary, tags, entry, limits, kernel_bank


class CellLibrary:
    """A warm host over a directory of cells."""

    def __init__(self, directory: str):
        self.directory = directory
        self.host = cell80_py.CellHost()
        # Memoization on: repeated identical runs become hash lookups, and the
        # cached outcomes are exportable as a `.facts` file (docs/12).
        self.host.set_cache(True)
        self._handles: dict[str, int] = {}
        self._ids: list[str] = []
        self._load(pathlib.Path(directory))

    def _load(self, d: pathlib.Path) -> None:
        if not d.is_dir():
            raise FileNotFoundError(f"cell library dir not found: {d}")
        # Cells live in pack subdirectories (cell80/cells/<pack>/<id>.rs), so this walks
        # the tree recursively rather than assuming a flat directory.
        files = sorted(d.rglob("*.rs")) + sorted(d.rglob("*.cell"))
        for f in files:
            if f.suffix == ".rs":
                src = f.read_text()
                summary, tags, entry, limits, kernel_bank = _parse_header(src)
                self.host.add_source(f.stem, src, summary, tags, entry, limits, kernel_bank)
                self._ids.append(f.stem)
            elif f.suffix == ".cell":
                self.host.add_cell(f.read_bytes())

    # ── discover ────────────────────────────────────────────────────────────
    def search(
        self, query: str, limit: int = 10, examples: list[dict] | None = None
    ) -> list[dict]:
        """Rank by text relevance; with `examples`, fuse BEHAVIOUR into the ranking:
        cells reproducing the most examples first, plain-search order breaking ties —
        the same-shape-sibling separator (min vs max share every word; ([3,7], 3)
        separates them). Example forms match cell_route_by_example, plus `expect`:
        {in: [ints], out: int} for value cells; {fields: {name: int}, out: int,
        expect: {name: int}} for state cells — `expect` matches post-run fields, the
        separator for status-flag cells whose return is constant. Empty/None
        examples: plain text search, unchanged."""
        if not examples:
            return list(self.host.search(query, limit))
        if "fields" in examples[0]:
            triples = [
                (
                    dict(e["fields"]),
                    int(e["out"]) if "out" in e else None,
                    dict(e.get("expect", {})),
                )
                for e in examples
            ]
            return list(self.host.search_with_field_examples(query, triples, limit))
        pairs = [(list(e["in"]), int(e["out"])) for e in examples]
        return list(self.host.search_with_examples(query, pairs, limit))

    def route(self, examples: list[tuple[list[int], int]], limit: int = 10) -> list[dict]:
        """Discover by behaviour: rank cells by how many (inputs, expected_output)
        examples they reproduce on the VM — the phrasing-independent signal that tells
        confusable siblings (min vs max) apart where their descriptions can't."""
        return list(self.host.route([(list(i), o) for i, o in examples], limit))

    def route_fields(
        self, examples: list[tuple[dict, int]], limit: int = 10
    ) -> list[dict]:
        """Behavioural routing for STATE cells: each example is ({field: value},
        expected_result) — register probes can't drive named state."""
        return list(
            self.host.route_fields([(dict(f), o) for f, o in examples], limit)
        )

    def inspect(self, cell_id: str) -> dict | None:
        return self.host.manifest(cell_id)

    def list(self) -> list[dict]:
        return [m for i in self._ids if (m := self.host.manifest(i)) is not None]

    # ── run (warm, handles hidden) ────────────────────────────────────────────
    def _handle(self, cell_id: str) -> int:
        if cell_id not in self._handles:
            if self.host.manifest(cell_id) is None:
                raise ValueError(f"no cell `{cell_id}`")
            self._handles[cell_id] = self.host.load(cell_id)
        return self._handles[cell_id]

    def run(self, cell_id: str, args: list[int]) -> dict:
        return self.host.run(self._handle(cell_id), list(args))

    def run_state(self, cell_id: str, fields: dict) -> dict:
        """Drive a state cell by named fields → {result, state: {...}, cost...}."""
        return self.host.run_state(self._handle(cell_id), dict(fields))

    def run_graph(self, graph, inputs: dict | None = None) -> dict:
        """Validate + run a CellGraph (a manifest dict, or a JSON string) over the warm
        library → {id, outputs, trace, cycles, trapped_ops}."""
        g = graph if isinstance(graph, str) else json.dumps(graph)
        return self.host.run_graph(g, dict(inputs or {}))

    def run_pipeline(self, spec, inputs: dict | None = None) -> dict:
        """Author + run a *pipeline* spec — steps with positional args (a number is a const,
        "$N" is step N's result, any other string is an external input by name; ports are
        resolved from each cell's manifest). The host builds the wires, type-checks, and runs;
        same return shape as `run_graph`. Lets a caller compose without wire-level JSON."""
        s = spec if isinstance(spec, str) else json.dumps(spec)
        return self.host.run_pipeline(s, dict(inputs or {}))

    # ── the fact file (docs/12) ───────────────────────────────────────────────
    def export_facts(self, producer: str = "cell80-mcp") -> str:
        """Every cached outcome across the warm runners, as `.facts` JSONL text
        (a header line + one canonical claim per line)."""
        return self.host.export_facts(producer)

    def import_facts(
        self, text: str, verify_fraction: float = 0.01, quarantine: bool = False
    ) -> dict:
        """Import a `.facts` text with a spot-check: a locally-seeded sample is
        re-executed under each fact's own claimed cost; one caught lie rejects the
        whole file (quarantine salvages the verified remainder). Returns the
        import report — an agent can *read* "N accepted, 1 falsified" and act."""
        return self.host.import_facts(text, verify_fraction, quarantine)

    def facts_stats(self) -> dict:
        """Per-loaded-cell cache economics: hits/lookups and the local-vs-imported
        provenance split of the hits."""
        cells = {}
        for cell_id, h in self._handles.items():
            stats = self.host.cache_stats(h)
            split = self.host.cache_split(h)
            if stats is None:
                continue
            hits, lookups = stats
            local, imported = split or (0, 0)
            cells[cell_id] = {
                "hits": hits,
                "lookups": lookups,
                "hits_local": local,
                "hits_imported": imported,
            }
        return {"cells": cells}

    def solve(self, plans, cycles: int = 2_000_000) -> dict:
        """The minimal `cell_solve` loop: candidate plans (dict or list of dicts)
        render to tiny deterministic programs, compile, run, and get verified/killed;
        disagreeing survivors face the counterfactual battery. A re-seen schema is
        retrieved (not recompiled) and repeats serve from the memo table."""
        import json as _json

        text = plans if isinstance(plans, str) else _json.dumps(plans)
        return self.host.solve(text, cycles)

    def __len__(self) -> int:
        return len(self.host)
