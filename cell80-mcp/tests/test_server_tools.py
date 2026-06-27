"""Exercise the MCP tool bodies and the entry point directly (no live server / transport)."""

import os
import pathlib

CELLS = pathlib.Path(__file__).resolve().parents[2] / "rustz80" / "cells"
os.environ.setdefault("CELL_LIBRARY", str(CELLS))

from cell80_mcp import agent, server  # noqa: E402


def _handlers():
    mcp = server.build_server()
    return {t.name: t.handler for t in mcp.get_tools()}


def test_tool_bodies_cover_all_four_verbs():
    h = _handlers()
    assert any(r["id"] == "gcd" for r in h["cell_search"]("greatest common divisor", 3)["results"])
    assert h["cell_inspect"]("gcd")["signature"] == "run(a: u16, b: u16) -> u16"
    assert "error" in h["cell_inspect"]("ghost")
    assert len(h["cell_list"]()["cells"]) == 8
    assert h["cell_run"]("gcd", [48, 36])["result"] == 12
    assert "result" in h["cell_run"]("gcd", None)  # None args → [] (the `args or []` branch)
    assert "error" in h["cell_run"]("ghost", [1])


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
