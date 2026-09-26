"""The warm cell library — a thin Python wrapper over the PyO3 `cell80_py.CellHost`.

It loads a directory of cells once (`.rs` sources, metadata from a `//!` header; or
precompiled `.cell` cartridges), then serves `search` / `inspect` / `run`. `run` hides
handles: it lazily `load`s a warm runner per id and reuses it, so the model just names a
tool + args and the host keeps it warm across calls.
"""

from __future__ import annotations

import json
import math
import os
import pathlib

import cell80_py

from .retrieval import (
    DEFAULT_HYBRID_MARGIN,
    DEFAULT_LEXICAL_MARGIN,
    DEFAULT_MODEL,
    HybridRetriever,
)


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

    def __init__(
        self, directory: str, *, embedder=None, retrieval_model: str | None = None
    ):
        self.directory = directory
        self.host = cell80_py.CellHost()
        # Memoization on: repeated identical runs become hash lookups, and the
        # cached outcomes are exportable as a `.facts` file (docs/12).
        self.host.set_cache(True)
        self._handles: dict[str, int] = {}
        self._ids: list[str] = []
        self._load(pathlib.Path(directory))
        self._retriever = HybridRetriever(
            self.host,
            self.list,
            model=(
                retrieval_model
                if retrieval_model is not None
                else os.environ.get("CELL_RETRIEVAL_MODEL", DEFAULT_MODEL)
            ),
            embedder=embedder,
        )

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
                self._ids.append(f.stem)

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

    def search_decision(
        self,
        query: str,
        limit: int = 10,
        examples: list[dict] | None = None,
        min_margin: float | None = None,
    ) -> dict:
        """Search with a machine-readable selection/abstention decision.

        Text-only requests use a cached catalog-wide embedding rerank, followed
        by a calibrated margin gate. Candidate manifests stay in ``results`` on abstention.
        If the optional local embedding model is unavailable, the response visibly falls
        back to the calibrated lexical/type-led decision. Requests carrying examples keep
        the stronger fused behavioural ranking.
        """
        if min_margin is not None and (
            not isinstance(min_margin, (int, float))
            or not math.isfinite(min_margin)
            or min_margin < 0
        ):
            raise ValueError("min_margin must be a finite non-negative number")
        if not examples:
            ranked = self._retriever.rank(query, limit)
            if ranked is not None:
                threshold = (
                    DEFAULT_HYBRID_MARGIN if min_margin is None else float(min_margin)
                )
                results = [item["manifest"] for item in ranked]
                top_score = ranked[0]["score"] if ranked else None
                runner_score = ranked[1]["score"] if len(ranked) > 1 else None
                margin = (
                    top_score - runner_score
                    if runner_score is not None
                    else (top_score if top_score is not None else 0.0)
                )
                selected = results[0] if results and margin >= threshold else None
                status = (
                    "no_match"
                    if not results
                    else "confident"
                    if selected
                    else "low_margin"
                )
                return {
                    "results": results,
                    "selected": selected,
                    "confidence": {
                        "method": "hybrid_margin",
                        "ranking": "tfidf_embedding",
                        "model": self._retriever.model,
                        "alpha": self._retriever.alpha,
                        "type_bonus": self._retriever.type_bonus,
                        "type_rank_k": self._retriever.type_rank_k,
                        "catalog_size": self._retriever.catalog_size,
                        "status": status,
                        "abstained": selected is None,
                        "threshold": threshold,
                        "margin": margin,
                        "top_score": top_score,
                        "runner_up_score": runner_score,
                    },
                }

            lexical_margin = (
                DEFAULT_LEXICAL_MARGIN if min_margin is None else float(min_margin)
            )
            decision = dict(self.host.search_confident(query, limit, lexical_margin))
            # Keep the established live TypeLedIndex ordering. The confidence score stays
            # raw TF-IDF because that is the calibrated magnitude; if its top choice and the
            # live re-rank disagree, abstain instead of attaching one ranking's confidence to
            # another ranking's winner.
            results = self.search(query, limit)
            raw_selected = decision["selected"]
            live_top = results[0] if results else None
            confidence = decision["confidence"]
            confidence["method"] = "lexical_fallback"
            confidence["ranking"] = "type_led"
            confidence["fallback_reason"] = self._retriever.unavailable_reason
            if (
                not confidence["abstained"]
                and raw_selected is not None
                and live_top is not None
                and raw_selected["id"] != live_top["id"]
            ):
                confidence["status"] = "rank_disagreement"
                confidence["abstained"] = True
                confidence["raw_top"] = raw_selected["id"]
            decision["results"] = results
            decision["selected"] = None if confidence["abstained"] else live_top
            return decision

        results = self.search(query, limit, examples)
        selected = results[0] if results else None
        return {
            "results": results,
            "selected": selected,
            "confidence": {
                "method": "behaviour_fused",
                "status": "evidence_ranked" if selected else "no_match",
                "abstained": selected is None,
                "threshold": None,
                "margin": None,
                "top_score": None,
                "runner_up_score": None,
            },
        }

    def route(
        self, examples: list[tuple[list[int], int]], limit: int = 10
    ) -> list[dict]:
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
        """Run a plain cell; the reply's `cached` flag says whether this exact
        (cell, args) pair was served from the memo table (no execution) or run fresh —
        the hits/lookups delta straddling this one call (docs/12's per-call provenance)."""
        handle = self._handle(cell_id)
        before = self.host.cache_stats(handle)
        result = self.host.run(handle, list(args))
        after = self.host.cache_stats(handle)
        if before is not None and after is not None:
            result["cached"] = after[0] > before[0]
        return result

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
