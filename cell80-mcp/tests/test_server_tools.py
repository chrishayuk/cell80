"""Exercise the MCP tool bodies and the entry point directly (no live server / transport)."""

import os
import pathlib

CELLS = pathlib.Path(__file__).resolve().parents[2] / "cell80" / "cells"
os.environ.setdefault("CELL_LIBRARY", str(CELLS))

from cell80_mcp import agent, server  # noqa: E402


def _handlers():
    mcp = server.build_server()
    return {t.name: t.handler for t in mcp.get_tools()}


def test_tool_bodies_cover_all_four_verbs():
    h = _handlers()
    assert any(r["id"] == "gcd" for r in h["cell_search"]("greatest common divisor", 3)["results"])
    # route by behaviour: (3,7)→3 / (10,3)→3 surfaces min and excludes its sibling max (which
    # can't match); a malformed example is reported as data, not raised.
    routed = h["cell_route_by_example"]([{"in": [3, 7], "out": 3}, {"in": [10, 3], "out": 3}])
    routed_ids = [r["id"] for r in routed["results"]]
    assert "min" in routed_ids and "max" not in routed_ids
    assert "error" in h["cell_route_by_example"]([{"bad": 1}])
    assert h["cell_inspect"]("gcd")["signature"] == "run(a: u16, b: u16) -> u16"
    assert "error" in h["cell_inspect"]("ghost")
    assert len(h["cell_list"]()["cells"]) == 145
    assert h["cell_run"]("gcd", [48, 36])["result"] == 12
    assert "result" in h["cell_run"]("gcd", None)  # None args → [] (the `args or []` branch)
    assert "error" in h["cell_run"]("ghost", [1])
    # cell_run drives a state cell by named fields, returning the full state.
    st = h["cell_run"]("manhattan", fields={"x1": 3, "y1": 4, "x2": 10, "y2": 8})
    assert st["result"] == 11 and st["state"]["dist"] == 11
    assert "error" in h["cell_run"]("manhattan", fields={"bogus": 1})
    # cell_compose: chain cells as a pipeline (positional args, "$N" refs, external inputs) —
    # the ergonomic surface, no wires or port names. Same move-ranker result as a graph.
    comp = h["cell_compose"](
        [
            {"cell": "manhattan", "args": ["x1", "y1", "x2", "y2"]},
            {"cell": "weighted_sum", "args": ["$0", "risk", "cost"]},
            {"cell": "clamp", "args": ["$1", 0, 10]},
        ],
        inputs={"x1": 3, "y1": 4, "x2": 10, "y2": 8, "risk": 2, "cost": 1},
    )
    assert comp["outputs"]["out"] == 10
    assert "error" in h["cell_compose"]([{"cell": "weighted_sum", "args": [1, 2]}])  # bad arity


def test_library_is_a_cached_singleton():
    assert server.library() is server.library()


def test_agent_main_stdio(monkeypatch):
    calls = {}
    fake = type("M", (), {"run": lambda self, **kw: calls.update(kw)})()
    monkeypatch.setattr(agent, "build_server", lambda: fake)
    monkeypatch.setenv("CELL_STDIO", "1")
    agent.main()
    assert calls == {"stdio": True}


def test_agent_main_http(monkeypatch):
    calls = {}
    fake = type("M", (), {"run": lambda self, **kw: calls.update(kw)})()
    monkeypatch.setattr(agent, "build_server", lambda: fake)
    monkeypatch.delenv("CELL_STDIO", raising=False)
    monkeypatch.setenv("CELL_HOST", "1.2.3.4")
    monkeypatch.setenv("CELL_PORT", "9999")
    agent.main()
    assert calls == {"host": "1.2.3.4", "port": 9999}


def test_cell_graph_run_handler_composes_and_validates():
    h = _handlers()
    graph = {
        "id": "g",
        "nodes": {"d": "manhattan", "b": "clamp"},
        "wires": [
            {"to": "d.x1", "input": "x1"}, {"to": "d.y1", "input": "y1"},
            {"to": "d.x2", "input": "x2"}, {"to": "d.y2", "input": "y2"},
            {"to": "b.x", "from": "d.dist"}, {"to": "b.lo", "const": 0}, {"to": "b.hi", "const": 5},
        ],
        "outputs": {"capped": "b.result"},
    }
    out = h["cell_graph_run"](graph, {"x1": 0, "y1": 0, "x2": 10, "y2": 0})
    assert out["outputs"]["capped"] == 5  # dist 10 → clamp(10,0,5) = 5
    # A structurally bad graph is reported as data, not raised.
    bad = {"nodes": {"x": "clamp"}, "wires": [{"to": "x.bogus", "const": 1}], "outputs": {}}
    assert "error" in h["cell_graph_run"](bad)


def test_escalation_is_a_typed_result_not_an_error(tmp_path, monkeypatch):
    """The escalation contract (roadmap 3.2) over the MCP surface: a `//! limits:` header
    lands in the inspect manifest, and a halt in the escalation band comes back as a
    typed hand-off (halt='escalate' + reason), distinct from {'error': ...}."""
    (tmp_path / "bounded_double.rs").write_text(
        "//! Double a small reading.\n"
        "//! tags: math, double\n"
        "//! limits: floats, inputs > 1000\n"
        "fn run(n: u16) -> u16 { if n > 1000 { halt(0xFF06u16); } n * 2 }\n"
    )
    from cell80_mcp.library import CellLibrary

    lib = CellLibrary(str(tmp_path))
    m = lib.inspect("bounded_double")
    assert m["limits"] == ["floats", "inputs > 1000"]

    ok = lib.run("bounded_double", [21])
    assert ok["result"] == 42 and ok["halt"] == "returned"

    esc = lib.run("bounded_double", [5000])
    assert "error" not in esc
    assert esc["halt"] == "escalate"
    assert esc["escalate"] == "out_of_domain"
    assert esc["escalate_code"] == 0xFF06
