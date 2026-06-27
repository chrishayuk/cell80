"""The cell tool surface, as OpenAI-format function tools.

These mirror the MCP tools (`cell_search` / `cell_inspect` / `cell_list` / `cell_run`)
one-to-one, but as OpenAI `tools=[...]` schemas so the adoption eval can drive any
OpenAI-compatible endpoint — Ollama by default. `dispatch()` runs a tool call against a
`CellLibrary` and returns a JSON-serializable result, recording which cells were *run* so
the eval can measure adoption (did the model actually execute a cell, or just talk?).
"""

from __future__ import annotations

from dataclasses import dataclass, field

from cell80_mcp.library import CellLibrary

# OpenAI function-tool schemas — the exact surface the model sees.
TOOLS = [
    {
        "type": "function",
        "function": {
            "name": "cell_search",
            "description": (
                "Search the library of tiny verified tools ('cells') by natural-language "
                "query. Returns ranked candidates with id, summary, tags and typed signature."
            ),
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "what the tool should do"},
                    "limit": {"type": "integer", "description": "max results (default 5)"},
                },
                "required": ["query"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "cell_inspect",
            "description": (
                "Inspect one cell by id: full manifest including the typed signature "
                "(parameter names + types and return type). Use this to learn how to call it."
            ),
            "parameters": {
                "type": "object",
                "properties": {"id": {"type": "string"}},
                "required": ["id"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "cell_list",
            "description": "List every cell in the library (id, summary, tags, signature).",
            "parameters": {"type": "object", "properties": {}},
        },
    },
    {
        "type": "function",
        "function": {
            "name": "cell_run",
            "description": (
                "Run a cell by id with integer arguments, in signature order. Returns the "
                "result plus an honest cost surface (cycles, trapped_ops). This is the "
                "verified answer — prefer it over computing the value yourself."
            ),
            "parameters": {
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "args": {
                        "type": "array",
                        "items": {"type": "integer"},
                        "description": "arguments in signature order",
                    },
                },
                "required": ["id", "args"],
            },
        },
    },
]


@dataclass
class ToolTrace:
    """Records what the model did with the tools during one episode."""

    searched: list[str] = field(default_factory=list)  # queries
    inspected: list[str] = field(default_factory=list)  # ids
    cells_run: list[str] = field(default_factory=list)  # ids actually executed
    run_results: list[int] = field(default_factory=list)  # results returned by cell_run


def dispatch(lib: CellLibrary, name: str, args: dict, trace: ToolTrace) -> dict:
    """Execute one tool call against the library; never raises — errors come back as data
    so the agent loop can let the model recover."""
    try:
        if name == "cell_search":
            q = args["query"]
            trace.searched.append(q)
            return {"results": lib.search(q, int(args.get("limit", 5)))}
        if name == "cell_inspect":
            trace.inspected.append(args["id"])
            m = lib.inspect(args["id"])
            return m if m is not None else {"error": f"no cell `{args['id']}`"}
        if name == "cell_list":
            return {"cells": lib.list()}
        if name == "cell_run":
            cid = args["id"]
            out = lib.run(cid, [int(a) for a in args.get("args", [])])
            trace.cells_run.append(cid)
            if isinstance(out.get("result"), int):
                trace.run_results.append(out["result"])
            return out
        return {"error": f"unknown tool `{name}`"}
    except Exception as e:  # ValueError from unknown id, etc.
        return {"error": f"{type(e).__name__}: {e}"}
