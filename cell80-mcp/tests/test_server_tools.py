"""Exercise the MCP tool bodies and the entry point directly (no live server / transport)."""

import os
import pathlib

# The seed library by default, overridable via CELL_LIBRARY (e.g. a committed-cells
# snapshot while a concurrent session has in-flight cells in the working tree).
# CELLS must be the dir the library actually loads, or the count assertions drift.
os.environ.setdefault(
    "CELL_LIBRARY",
    str(pathlib.Path(__file__).resolve().parents[2] / "cell80" / "cells"),
)
CELLS = pathlib.Path(os.environ["CELL_LIBRARY"])

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
    assert len(h["cell_list"]()["cells"]) == len(list(CELLS.rglob("*.rs")))
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


def test_cell_run_drives_array_state_fields_as_lists():
    # The `.cell` v11 array-state surface end-to-end through the MCP tool body:
    # a sliding-window cell's window rides as a JSON list, reads back as one, and
    # feeding the returned state into the next call persists it — avg 10, then 15.
    h = _handlers()
    first = h["cell_run"]("simple_moving_average", fields={"value": 10})
    assert first["result"] == 10
    assert first["state"]["window"] == [10, 0, 0, 0, 0, 0, 0, 0]
    feed = dict(first["state"])
    feed["value"] = 20
    second = h["cell_run"]("simple_moving_average", fields=feed)
    assert second["result"] == 15  # (10 + 20) / 2 — the window persisted by name
    assert second["state"]["window"] == [10, 20, 0, 0, 0, 0, 0, 0]


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


def test_facts_verbs_export_import_stats():
    # The sharing loop through the MCP surface (docs/12 §4): run → export → the
    # claims re-import cleanly, a tampered digit is caught, and the stats verb
    # shows the provenance split.
    h = _handlers()
    assert h["cell_run"]("gcd", [48, 36])["result"] == 12
    exported = h["cell_facts_export"]("test@mcp")
    assert exported["count"] >= 1
    assert '"args":[48,36]' in exported["facts"]

    # A clean import verifies and accepts (fraction 1.0 → every line re-executed).
    rep = h["cell_facts_import"](exported["facts"], verify_fraction=1.0)
    assert rep["accepted"] >= 1 and not rep["file_failed"] and not rep["failures"]

    # One flipped digit: the file is rejected and the report names the line.
    tampered = exported["facts"].replace('"r":[12,', '"r":[13,')
    rep = h["cell_facts_import"](tampered, verify_fraction=1.0)
    assert rep["file_failed"] and rep["failures"][0]["line"] > 1

    # Stats: gcd has been run and hit at least once by the verification replays.
    stats = h["cell_facts_stats"]()
    assert "gcd" in stats["cells"]
    assert stats["cells"]["gcd"]["lookups"] >= 1


def test_cell_solve_plans_exactly():
    # The cell_solve rung: a good plan answers; candidate plans that disagree get
    # the counterfactual battery; a bad plan is killed with the reason, never a
    # wrong number.
    h = _handlers()
    lego = {
        "quantities": [
            {"id": "lego_sets", "value": 13, "unit": "count"},
            {"id": "lego_price", "value": 1500, "unit": "cents_per_count"},
        ],
        "ops": [["mul", "lego_sets", "lego_price", "lego_money"]],
        "target": "lego_money",
    }
    rep = h["cell_solve"](lego)
    assert rep["answer"] == 13 * 1500
    assert rep["plans"][0]["kill"] is None

    # Same schema, new numbers: retrieved, not recompiled (procedural memory).
    lego2 = dict(lego)
    lego2["quantities"] = [
        {"id": "lego_sets", "value": 4, "unit": "count"},
        {"id": "lego_price", "value": 1500, "unit": "cents_per_count"},
    ]
    rep = h["cell_solve"](lego2)
    assert rep["answer"] == 4 * 1500
    assert rep["plans"][0]["retrieved"] is True

    # A unit-mismatched plan dies at render, reported as a kill.
    bad = {
        "quantities": [
            {"id": "money", "value": 5, "unit": "cents"},
            {"id": "wait", "value": 2, "unit": "hours"},
        ],
        "ops": [["add", "money", "wait", "x"]],
        "target": "x",
    }
    rep = h["cell_solve"](bad)
    assert rep["answer"] is None
    assert "unit mismatch" in rep["plans"][0]["kill"]


def test_route_by_field_examples_drives_state_cells():
    # The structured routing form: named fields in, expected result out — the
    # signal register probes can't produce for Struct::run cells.
    h = _handlers()
    routed = h["cell_route_by_example"](
        [{"fields": {"x1": 3, "y1": 4, "x2": 10, "y2": 8}, "out": 11}]
    )
    ids = [r["id"] for r in routed["results"]]
    assert "manhattan" in ids and "chebyshev" not in ids, ids
    # Flip the expected output to the chebyshev answer: max(7,4) = 7.
    routed = h["cell_route_by_example"](
        [{"fields": {"x1": 3, "y1": 4, "x2": 10, "y2": 8}, "out": 7}]
    )
    ids = [r["id"] for r in routed["results"]]
    assert "chebyshev" in ids and "manhattan" not in ids, ids
    assert "error" in h["cell_route_by_example"]([{"fields": "bad"}])


def test_cell_search_fuses_examples_into_the_ranking():
    # The fused path (WS-F): examples pull the behavioural match to the top where
    # text alone can't separate same-shape siblings; no examples = plain search.
    h = _handlers()
    plain = h["cell_search"]("pick one of two numbers", 10)["results"]
    # Three examples, one above the i16 range: small positives can't separate `min`
    # from `min_i16` (identical behaviour there) — 40000 reads as negative in i16,
    # so only the unsigned cell survives all three.
    fused = h["cell_search"](
        "pick one of two numbers",
        10,
        [{"in": [3, 7], "out": 3}, {"in": [9, 4], "out": 4}, {"in": [40000, 7], "out": 7}],
    )["results"]
    assert fused[0]["id"] == "min", [r["id"] for r in fused[:3]]
    # Same tool, mirrored behaviour → the sibling.
    fused = h["cell_search"](
        "pick one of two numbers",
        10,
        [{"in": [3, 7], "out": 7}, {"in": [9, 4], "out": 9}, {"in": [40000, 7], "out": 40000}],
    )["results"]
    assert fused[0]["id"] == "max", [r["id"] for r in fused[:3]]
    # No/empty examples: byte-identical to plain search.
    assert h["cell_search"]("pick one of two numbers", 10, [])["results"] == plain
    # State-cell form with `expect`: post-run fields separate status-flag siblings —
    # smag_add and smag_sub both return 1; only mag/neg differ. |9| + (-|4|) = 5.
    ex = {
        "fields": {"mag_a": 9, "neg_a": 0, "mag_b": 4, "neg_b": 1},
        "out": 1,
        "expect": {"mag": 5, "neg": 0},
    }
    fused = h["cell_search"]("combine two signed magnitudes", 10, [ex])["results"]
    assert fused[0]["id"] == "smag_add", [r["id"] for r in fused[:3]]
    # Malformed examples are reported as data, not raised.
    assert "error" in h["cell_search"]("anything", 5, [{"bad": 1}])
