"""Locate a cell library and open it as the same warm `CellLibrary` the MCP server uses.

The eval drives `CellLibrary` directly rather than spawning the MCP server: the MCP tools
(`cell_search` / `cell_inspect` / `cell_run`) are thin wrappers over exactly this object,
so search/inspect/run go through the identical code path an agent gets over the wire — no
process, no transport, fully deterministic.
"""

from __future__ import annotations

import os
import pathlib

from cell80_mcp.library import CellLibrary


def seed_library_dir() -> pathlib.Path:
    """Find the seed cell library (`cell80/cells`).

    Resolution order: `$CELL_LIBRARY`, then walk up from here looking for
    `cell80/cells` (works from a source checkout regardless of cwd).
    """
    env = os.environ.get("CELL_LIBRARY")
    if env:
        return pathlib.Path(env)
    here = pathlib.Path(__file__).resolve()
    for parent in here.parents:
        candidate = parent / "cell80" / "cells"
        if candidate.is_dir():
            return candidate
    raise FileNotFoundError(
        "could not locate the seed cell library; set CELL_LIBRARY to a cells directory"
    )


def open_library(directory: str | os.PathLike | None = None) -> CellLibrary:
    """Open a `CellLibrary` over `directory` (default: the seed library)."""
    d = pathlib.Path(directory) if directory is not None else seed_library_dir()
    return CellLibrary(str(d))
