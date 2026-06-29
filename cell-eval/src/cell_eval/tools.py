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
                "Run a cell by id. For a plain cell, pass `args` (integers, in signature "
                "order). For a STATE cell — one whose manifest lists `state` fields, e.g. "
                "manhattan — pass `fields` as {name: int} to drive it by name; the reply then "
                "includes the full post-run `state`. Returns the result plus an honest cost "
                "surface (cycles, trapped_ops). This is the verified answer — prefer it over "
                "computing the value yourself."
            ),
            "parameters": {
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "args": {
                        "type": "array",
                        "items": {"type": "integer"},
                        "description": "positional arguments in signature order (plain cells)",
                    },
                    "fields": {
                        "type": "object",
                        "description": "named state fields {name: int} (state cells)",
                        "additionalProperties": {"type": "integer"},
                    },
                },
                "required": ["id"],
            },
        },
    },
]

# The composition tool (offered only by the composition eval): wire cells into a graph.
GRAPH_TOOL = {
    "type": "function",
    "function": {
        "name": "cell_graph_run",
        "description": (
            "Compose cells into a graph and run it host-routed. `graph` is a manifest: "
            "{id, nodes: {node_name: cell_id}, wires: [{to: 'node.port', and ONE of "
            "from: 'node.port' | input: 'name' | const: int}], outputs: {name: 'node.port'}}. "
            "A node's input ports are the cell's params (value cell) or state fields (state "
            "cell); output ports are 'result' plus any state fields. `inputs` is the external "
            "{name: int}. The host type-checks the whole graph before running and returns "
            "{outputs, trace, cycles}. Use this to chain cells — one cell's output into "
            "another's input — instead of doing the arithmetic yourself."
        ),
        "parameters": {
            "type": "object",
            "properties": {
                "graph": {"type": "object", "description": "the graph manifest"},
                "inputs": {
                    "type": "object",
                    "additionalProperties": {"type": "integer"},
                    "description": "external graph inputs {name: int}",
                },
            },
            "required": ["graph"],
        },
    },
}


# The pipeline-authoring tool: the *easy* way to compose — no graph JSON, no port names.
COMPOSE_TOOL = {
    "type": "function",
    "function": {
        "name": "cell_compose",
        "description": (
            "Compose cells into a PIPELINE and run it — the EASY way to chain cells; no graph "
            "manifest, wires, or port names. `steps` is an ordered list of {cell, args}. Each "
            "arg is positional (the cell's signature order) and is ONE of: a number (a "
            "constant), \"$N\" (the result of step N — a 0-based EARLIER step), or a string (an "
            "external input by name). The last step's result is the answer. `inputs` is the "
            "external {name: int}. Example — clamp(weighted_sum(manhattan(x1,y1,x2,y2), risk, "
            "cost), 0, 10): steps=[{cell:'manhattan', args:['x1','y1','x2','y2']}, "
            "{cell:'weighted_sum', args:['$0','risk','cost']}, {cell:'clamp', args:['$1',0,10]}]"
            ". PREFER this over cell_graph_run for chaining."
        ),
        "parameters": {
            "type": "object",
            "properties": {
                "steps": {
                    "type": "array",
                    "description": "ordered pipeline steps",
                    "items": {
                        "type": "object",
                        "properties": {
                            "cell": {"type": "string"},
                            "args": {
                                "type": "array",
                                "description": "positional args: a number, \"$N\", or an input name",
                            },
                        },
                        "required": ["cell", "args"],
                    },
                },
                "inputs": {
                    "type": "object",
                    "additionalProperties": {"type": "integer"},
                    "description": "external inputs {name: int}",
                },
            },
            "required": ["steps"],
        },
    },
}


@dataclass
class ToolTrace:
    """Records what the model did with the tools during one episode."""

    searched: list[str] = field(default_factory=list)  # queries
    inspected: list[str] = field(default_factory=list)  # ids
    cells_run: list[str] = field(default_factory=list)  # ids actually executed
    run_results: list[int] = field(default_factory=list)  # results returned by cell_run
    graphs_run: list[int] = field(default_factory=list)  # node count per cell_graph_run call
    pipelines_run: list[int] = field(default_factory=list)  # step count per cell_compose call


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
            fields = args.get("fields")
            if fields:
                out = lib.run_state(cid, {k: int(v) for k, v in fields.items()})
            else:
                out = lib.run(cid, [int(a) for a in args.get("args", [])])
            trace.cells_run.append(cid)
            if isinstance(out.get("result"), int):
                trace.run_results.append(out["result"])
            return out
        if name == "cell_graph_run":
            graph = args.get("graph") or {}
            inputs = {k: int(v) for k, v in (args.get("inputs") or {}).items()}
            out = lib.run_graph(graph, inputs)
            # Node count → the "composed" signal (a graph with ≥2 nodes is real composition).
            trace.graphs_run.append(len(graph.get("nodes", {})) if isinstance(graph, dict) else 0)
            return out
        if name == "cell_compose":
            steps = args.get("steps") or []
            # Forgiving: a constant passed as a digit-string ("3") → 3; "$N"/input names stay.
            def _coerce(a):
                return int(a) if isinstance(a, str) and a.lstrip("-").isdigit() else a

            spec = {
                "steps": [
                    {"cell": s.get("cell"), "args": [_coerce(a) for a in (s.get("args") or [])]}
                    for s in steps
                ]
            }
            inputs = {k: int(v) for k, v in (args.get("inputs") or {}).items()}
            out = lib.run_pipeline(spec, inputs)
            # A pipeline is a host-routed graph (just easier to author); tracked separately from
            # raw cell_graph_run so the eval can attribute composition to the helper.
            trace.pipelines_run.append(len(steps) if isinstance(steps, list) else 0)
            return out
        return {"error": f"unknown tool `{name}`"}
    except Exception as e:  # ValueError from unknown id, etc.
        return {"error": f"{type(e).__name__}: {e}"}
