"""Tests for the cell library + the MCP tool surface. Uses the seed library in
`cell80/cells` (relative to the repo root)."""

import os
import pathlib

CELLS = pathlib.Path(__file__).resolve().parents[2] / "cell80" / "cells"
os.environ.setdefault("CELL_LIBRARY", str(CELLS))

from cell80_mcp import server  # noqa: E402
from cell80_mcp.library import CellLibrary  # noqa: E402


def test_library_search_inspect_run_warm():
    lib = CellLibrary(str(CELLS))
    assert len(lib) == 163

    # search ranks by relevance. ("grid distance" now hits the whole distance family —
    # manhattan/chebyshev/euclid_sq — so the cell-specific name disambiguates.)
    assert lib.search("manhattan distance", 3)[0]["id"] == "manhattan"
    assert {"id", "summary", "tags", "signature"} <= set(lib.search("math", 1)[0])

    # route by BEHAVIOUR: min and max share a description, but their behaviour cleanly
    # separates the pair — min matches (3,7)→3 / (10,3)→3 and max can't, and vice versa. (Other
    # cells that happen to agree on these inputs — e.g. median-of-three with an implicit 0,
    # which equals min — also surface; behavioural routing returns every cell that matches,
    # so precision is in choosing discriminating examples.)
    min_ids = [r["id"] for r in lib.route([([3, 7], 3), ([10, 3], 3)])]
    max_ids = [r["id"] for r in lib.route([([3, 7], 7), ([10, 3], 10)])]
    assert "min" in min_ids and "max" not in min_ids
    assert "max" in max_ids and "min" not in max_ids

    # inspect carries the typed signature.
    g = lib.inspect("gcd")
    assert g["signature"] == "run(a: u16, b: u16) -> u16"
    assert g["params"] == [("a", "u16"), ("b", "u16")]  # PyO3 maps (name, ty) → tuples
    assert lib.inspect("ghost") is None

    # run — and reuse warm (same handle under the hood).
    assert lib.run("gcd", [48, 36])["result"] == 12
    assert lib.run("gcd", [100, 60])["result"] == 20
    assert lib.run("clamp", [50, 0, 10])["result"] == 10
    assert lib.run("weighted_sum", [5, 1, 9])["result"] == 34  # 5 + 1*2 + 9*3

    # unknown cell → an error, not a crash.
    try:
        lib.run("ghost", [1])
        assert False, "expected ValueError"
    except ValueError:
        pass


def test_library_run_state_by_name():
    lib = CellLibrary(str(CELLS))
    # manhattan is a state cell: drive it by named fields, read the full state back.
    r = lib.run_state("manhattan", {"x1": 3, "y1": 4, "x2": 10, "y2": 8})
    assert r["result"] == 11  # |3-10| + |4-8|
    assert r["state"]["dist"] == 11
    assert r["state"]["x1"] == 3  # inputs read back too
    # warm reuse with different inputs.
    assert lib.run_state("manhattan", {"x1": 0, "y1": 0, "x2": 5, "y2": 2})["result"] == 7
    # an unknown field errors (raised by the host).
    try:
        lib.run_state("manhattan", {"nope": 1})
        assert False, "expected an error for an unknown field"
    except Exception:
        pass


def test_mcp_surface_is_a_small_router():
    mcp = server.build_server()
    names = sorted(t.name for t in mcp.get_tools())
    assert names == [
        "cell_compose",
        "cell_facts_export",
        "cell_facts_import",
        "cell_facts_stats",
        "cell_graph_run",
        "cell_inspect",
        "cell_list",
        "cell_route_by_example",
        "cell_run",
        "cell_search",
    ]


# The move-ranker graph: manhattan -> weighted_sum -> clamp, host-routed.
MOVE_RANKER = {
    "id": "move_ranker.v1",
    "nodes": {"dist": "manhattan", "score": "weighted_sum", "bounded": "clamp"},
    "wires": [
        {"to": "dist.x1", "input": "x1"},
        {"to": "dist.y1", "input": "y1"},
        {"to": "dist.x2", "input": "x2"},
        {"to": "dist.y2", "input": "y2"},
        {"to": "score.a", "from": "dist.dist"},
        {"to": "score.b", "input": "risk"},
        {"to": "score.c", "input": "cost"},
        {"to": "bounded.x", "from": "score.result"},
        {"to": "bounded.lo", "const": 0},
        {"to": "bounded.hi", "const": 10},
    ],
    "outputs": {"ranked": "bounded.result"},
}


def test_library_run_graph_composes_cells():
    lib = CellLibrary(str(CELLS))
    inputs = {"x1": 3, "y1": 4, "x2": 10, "y2": 8, "risk": 2, "cost": 1}
    run = lib.run_graph(MOVE_RANKER, inputs)
    # dist=11 -> score = 11 + 2*2 + 1*3 = 18 -> clamp(18,0,10) = 10
    assert run["outputs"]["ranked"] == 10
    assert [t["result"] for t in run["trace"]] == [11, 18, 10]
    assert run["cycles"] > 0
    # An invalid graph (type mismatch / bad port) comes back as an error via the host.
    bad = {"nodes": {"a": "clamp"}, "wires": [{"to": "a.nope", "const": 1}], "outputs": {}}
    try:
        lib.run_graph(bad)
        assert False, "expected an error for a bad graph"
    except Exception:
        pass


def test_missing_library_dir_raises():
    try:
        CellLibrary("/no/such/cells")
        assert False, "expected FileNotFoundError"
    except FileNotFoundError:
        pass
