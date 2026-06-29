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


def _parse_header(src: str) -> tuple[str, list[str], str | None]:
    """Read a cell source's leading `//!` header → (summary, tags, entry)."""
    summary, tags, entry = "", [], None
    for line in src.splitlines():
        s = line.strip()
        if s.startswith("//!"):
            r = s[3:].strip()
            if r.startswith("tags:"):
                tags = [t.strip() for t in r[5:].split(",") if t.strip()]
            elif r.startswith("entry:"):
                entry = r[6:].strip()
            elif not summary:
                summary = r
        elif s and not s.startswith("//"):
            break  # first code line — header done
    return summary, tags, entry


class CellLibrary:
    """A warm host over a directory of cells."""

    def __init__(self, directory: str):
        self.directory = directory
        self.host = cell80_py.CellHost()
        self._handles: dict[str, int] = {}
        self._ids: list[str] = []
        self._load(pathlib.Path(directory))

    def _load(self, d: pathlib.Path) -> None:
        if not d.is_dir():
            raise FileNotFoundError(f"cell library dir not found: {d}")
        for f in sorted(d.iterdir()):
            if f.suffix == ".rs":
                src = f.read_text()
                summary, tags, entry = _parse_header(src)
                self.host.add_source(f.stem, src, summary, tags, entry)
                self._ids.append(f.stem)
            elif f.suffix == ".cell":
                self.host.add_cell(f.read_bytes())

    # ── discover ────────────────────────────────────────────────────────────
    def search(self, query: str, limit: int = 10) -> list[dict]:
        return list(self.host.search(query, limit))

    def route(self, examples: list[tuple[list[int], int]], limit: int = 10) -> list[dict]:
        """Discover by behaviour: rank cells by how many (inputs, expected_output)
        examples they reproduce on the VM — the phrasing-independent signal that tells
        confusable siblings (min vs max) apart where their descriptions can't."""
        return list(self.host.route([(list(i), o) for i, o in examples], limit))

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

    def __len__(self) -> int:
        return len(self.host)
