"""Tests for the cell library + the MCP tool surface. Uses the seed library in
`rustz80/cells` (relative to the repo root)."""

import os
import pathlib

CELLS = pathlib.Path(__file__).resolve().parents[2] / "rustz80" / "cells"
os.environ.setdefault("CELL_LIBRARY", str(CELLS))

from cell80_mcp import server  # noqa: E402
from cell80_mcp.library import CellLibrary  # noqa: E402


def test_library_search_inspect_run_warm():
    lib = CellLibrary(str(CELLS))
    assert len(lib) == 8

    # search ranks by relevance.
    assert lib.search("grid distance", 3)[0]["id"] == "manhattan"
    assert {"id", "summary", "tags", "signature"} <= set(lib.search("math", 1)[0])

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
    assert names == ["cell_inspect", "cell_list", "cell_run", "cell_search"]


def test_missing_library_dir_raises():
    try:
        CellLibrary("/no/such/cells")
        assert False, "expected FileNotFoundError"
    except FileNotFoundError:
        pass
