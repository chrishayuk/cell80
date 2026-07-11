"""The MCP surface over a warm cell library — a thin router, **not** a tool per cell.

The whole point: a library can hold millions of cells, but MCP exposes only a few fixed
verbs. The agent `cell_search`es to surface a handful of candidate manifests, `cell_inspect`s
the typed interface, and `cell_run`s the one it picks — so the model's context never holds
more than the few cells it's actually considering. The host (index + warm runners) stays
alive in this process; `cell_run` keeps a runner warm per cell across calls.

Built on `chuk-mcp-server` (`@mcp.tool`); the same tool bodies would back a socket daemon.
The session/warmth lives in `CellLibrary`; tools carry no policy.
"""

from __future__ import annotations

import os

from chuk_mcp_server import ChukMCPServer

from cell80_mcp.library import CellLibrary

_LIBRARY: CellLibrary | None = None


def library() -> CellLibrary:
    """The process-wide warm library (lazily built from `$CELL_LIBRARY`)."""
    global _LIBRARY
    if _LIBRARY is None:
        _LIBRARY = CellLibrary(os.environ.get("CELL_LIBRARY", "cell80/cells"))
    return _LIBRARY


def build_server() -> ChukMCPServer:
    mcp = ChukMCPServer(
        name="cell80-mcp",
        version="0.1.0",
        description="Discover and run deterministic micro-tools (cells): search, inspect, run.",
    )
    lib = library()

    @mcp.tool(
        read_only_hint=True,
        description="Search the cell library by relevance; returns brief manifests "
        "(id, summary, tags, signature). Inspect/run only the few you pick — the library "
        "may hold far more cells than belong in context. Optionally attach input→output "
        "`examples` (same forms as cell_route_by_example, plus {..., expect: {field: int}} "
        "to match post-run state fields) to fuse behaviour into the ranking — use this "
        "when confusable cells share a description: it tells min from max where text "
        "can't, without dropping text-relevant results.",
    )
    def cell_search(query: str, limit: int = 10, examples: list[dict] | None = None) -> dict:
        try:
            return {"results": lib.search(query, limit, examples)}
        except (KeyError, TypeError, ValueError):
            return {
                "error": "each example needs {in: [ints], out: int} or "
                "{fields: {name: int}, out: int, expect: {name: int}} (out/expect optional, "
                "at least one required)"
            }

    @mcp.tool(
        read_only_hint=True,
        description="Discover by BEHAVIOUR, not words: given input→output examples, return the "
        "cells that actually reproduce them on the VM. `examples` is a list of "
        "{in: [ints], out: int} — e.g. [{in:[3,7],out:3},{in:[10,3],out:3}] finds `min` — or, "
        "for STATE cells, {fields: {name: int}, out: int} — e.g. "
        "[{fields:{x1:3,y1:4,x2:10,y2:8}, out:11}] finds `manhattan`. Use this when the "
        "wording is ambiguous or confusable cells share a description: it tells min from max "
        "where text can't. Empty results mean no cell in the library does this.",
    )
    def cell_route_by_example(examples: list[dict]) -> dict:
        try:
            if examples and "fields" in examples[0]:
                # State-cell form: {fields: {name: int}, out: int} — drives named
                # state fields, which register probes can't reach.
                pairs = [(dict(e["fields"]), int(e["out"])) for e in examples]
                return {"results": lib.route_fields(pairs)}
            pairs = [(list(e["in"]), int(e["out"])) for e in examples]
        except (KeyError, TypeError, ValueError):
            return {"error": "each example needs {in: [ints], out: int} or {fields: {name: int}, out: int}"}
        return {"results": lib.route(pairs)}

    @mcp.tool(
        read_only_hint=True,
        description="Full manifest for a cell id: typed signature (params/ret/state), "
        "abi version, source hash, and `limits` — the cell's declared boundary (what it "
        "CAN'T do); a request past it comes back as halt='escalate', not an error.",
    )
    def cell_inspect(id: str) -> dict:
        m = lib.inspect(id)
        return m if m is not None else {"error": f"no cell `{id}`"}

    @mcp.tool(
        read_only_hint=True,
        description="List every cell in the library (brief manifests).",
    )
    def cell_list() -> dict:
        return {"cells": lib.list()}

    @mcp.tool(
        description="Run a cell by id. For a plain cell pass `args` (u16 ints, in signature "
        "order). For a STATE cell — one whose manifest lists `state` fields, e.g. manhattan — "
        "pass `fields` as {name: int} to drive it by name; the reply then includes the full "
        "post-run `state`. Returns result + regs + cost (cycles, trapped_ops) + halt. "
        "halt='escalate' is a typed hand-off, NOT an error: the cell declares the request "
        "exceeds its kernel class (`escalate` names why — needs_strings/needs_floats/"
        "needs_io/...); route the request to a more capable tool instead of retrying. The "
        "runner stays warm across calls.",
    )
    def cell_run(
        id: str, args: list[int] | None = None, fields: dict | None = None
    ) -> dict:
        try:
            if fields:
                return lib.run_state(id, fields)
            return lib.run(id, args or [])
        except ValueError as e:
            return {"error": str(e)}

    @mcp.tool(
        description="Compose cells: run a CellGraph. `graph` is a manifest "
        "{id, nodes:{node:cell}, wires:[{to:'node.port', and one of from:'node.port' | "
        "input:'name' | const:int}], outputs:{name:'node.port'}}; `inputs` is the external "
        "{name:int}. The host wires one cell's typed output into another's typed input, "
        "type-checks the WHOLE graph before running a cycle, runs nodes in topological order, "
        "and returns {id, outputs, cycles, trapped_ops, trace}. Cells never see each other.",
    )
    def cell_graph_run(graph: dict, inputs: dict | None = None) -> dict:
        try:
            return lib.run_graph(graph, inputs or {})
        except Exception as e:  # bad manifest / type mismatch / cycle → data, not a crash
            return {"error": str(e)}

    @mcp.tool(
        description="Compose cells the EASY way: run a PIPELINE — no graph manifest, wires, or "
        "port names. `steps` is an ordered list of {cell, args}; each arg is positional (the "
        "cell's signature order) and is a number (constant), \"$N\" (the result of earlier step "
        "N, 0-based), or a string (an external input by name). The last step's result is the "
        "answer. `inputs` is the external {name:int}. The host resolves ports from each cell's "
        "manifest, type-checks, and runs — returning {id, outputs, cycles, trapped_ops, trace}. "
        "Prefer this over cell_graph_run for chaining.",
    )
    def cell_compose(steps: list, inputs: dict | None = None) -> dict:
        try:
            return lib.run_pipeline({"steps": steps}, inputs or {})
        except Exception as e:  # bad spec / type mismatch → data, not a crash
            return {"error": str(e)}

    @mcp.tool(
        read_only_hint=True,
        description="Export every memoized outcome across the warm cells as a `.facts` file "
        "(JSONL text): one line per claim — artifact hash, entry, inputs, outcome, and its "
        "cycle cost. The file is trustless by design: a receiver verifies claims by "
        "RE-EXECUTING a sample, never by trusting the sender.",
    )
    def cell_facts_export(producer: str = "cell80-mcp") -> dict:
        text = lib.export_facts(producer)
        return {"facts": text, "count": max(0, len(text.splitlines()) - 1)}

    @mcp.tool(
        description="Import a `.facts` text with a spot-check: an unpredictably-sampled "
        "fraction of the claims is re-executed under each fact's own claimed cost (a fact "
        "that runs long is a lie even if the result matches). One caught lie rejects the "
        "whole file — set quarantine=true to salvage the verified remainder instead. "
        "Accepted facts serve future runs as cache hits (see cell_facts_stats for the "
        "local-vs-imported split). Returns the import report: read it — 'accepted N, "
        "1 falsified at line L' is data to act on, not an error.",
    )
    def cell_facts_import(
        facts: str, verify_fraction: float = 0.01, quarantine: bool = False
    ) -> dict:
        try:
            return lib.import_facts(facts, verify_fraction, quarantine)
        except ValueError as e:
            return {"error": str(e)}

    @mcp.tool(
        read_only_hint=True,
        description="Cache economics per warm cell: hits/lookups, and how many hits were "
        "served from IMPORTED facts vs computed locally — the provenance split that shows "
        "shared facts doing real work.",
    )
    def cell_facts_stats() -> dict:
        return lib.facts_stats()

    @mcp.tool(
        description="Solve a word-problem PLAN (or several candidate plans) exactly: extract "
        "quantities/ops/target yourself, then pass the plan IR — "
        "{quantities:[{id,value,unit}...], ops:[[\"add|sub|mul|div\",a,b,out]...], target, "
        "constraints:[[\"nonneg\",x]|[\"exact_div\",a,b]...]}. Units are checked "
        "symbolically (cents + hours is rejected before it runs); every op is "
        "overflow/negative/exact-checked — a bad plan is KILLED with the reason, never a "
        "wrong number. Pass several candidate plans: survivors that disagree face a "
        "counterfactual perturbation battery and the consistent majority wins. "
        "answer=None means escalate: solve it yourself and consider register-back. "
        "Values are integers — use cents (never dollars-with-decimals) and basis points "
        "(never percent floats).",
    )
    def cell_solve(plans, cycles: int = 2_000_000) -> dict:
        try:
            return lib.solve(plans, cycles)
        except ValueError as e:
            return {"error": str(e)}

    return mcp
