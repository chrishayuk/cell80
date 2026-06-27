"""The cell tool surface used by the adoption eval — exercised without any LLM.

This proves the agent's tools work end-to-end (search → inspect → run) against the real
library, so an adoption-eval failure can be attributed to the model/steering, not the tools.
"""

from cell_eval.library import open_library
from cell_eval.tools import TOOLS, ToolTrace, dispatch


def test_tool_schemas_mirror_the_mcp_surface():
    names = sorted(t["function"]["name"] for t in TOOLS)
    assert names == ["cell_inspect", "cell_list", "cell_run", "cell_search"]


def test_dispatch_search_inspect_run_and_trace():
    lib = open_library()
    trace = ToolTrace()

    s = dispatch(lib, "cell_search", {"query": "greatest common divisor", "limit": 3}, trace)
    assert any(r["id"] == "gcd" for r in s["results"])
    assert trace.searched == ["greatest common divisor"]

    i = dispatch(lib, "cell_inspect", {"id": "gcd"}, trace)
    assert i["signature"] == "run(a: u16, b: u16) -> u16"
    assert trace.inspected == ["gcd"]

    r = dispatch(lib, "cell_run", {"id": "gcd", "args": [48, 36]}, trace)
    assert r["result"] == 12
    assert trace.cells_run == ["gcd"]
    assert trace.run_results == [12]


def test_dispatch_errors_come_back_as_data():
    lib = open_library()
    trace = ToolTrace()
    assert "error" in dispatch(lib, "cell_inspect", {"id": "ghost"}, trace)
    assert "error" in dispatch(lib, "cell_run", {"id": "ghost", "args": [1]}, trace)
    assert "error" in dispatch(lib, "no_such_tool", {}, trace)
    assert trace.cells_run == []  # a failed run is not counted as adoption
