"""The library-scale curve (Phase 2.3): track retrieval — and adoption/composition when a
model endpoint is configured — as `cell80/cells/` grows toward ~1,000 cells.

There's no "snapshot history" mechanism: a checkpoint literally runs `retrieval`/
`adoption`/`composition` against `cell80/cells/` as it stands right now, at this commit, so
the history *is* the git history. Call `cell-eval curve` once per checkpoint (after a batch
of cells lands, not per-cell) and it appends one record to
`cell-eval/baselines/library-scale-curve.json`, following the existing baselines
convention (committed on purpose, one bullet added to `baselines/README.md` per file/entry
of interest).

Adoption/composition need a reachable OpenAI-compatible/Ollama endpoint. When one isn't
configured, the checkpoint still records retrieval (deterministic, always available) and
marks the other two `{"skipped": "<reason>"}` rather than fabricating a number.
"""

from __future__ import annotations

import json
import pathlib
import subprocess

from .adoption import run_adoption
from .composition import run_composition
from .library import open_library
from .retrieval import run_retrieval

CURVE_PATH = (
    pathlib.Path(__file__).resolve().parents[2] / "baselines" / "library-scale-curve.json"
)


def _git_commit() -> str | None:
    try:
        out = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            capture_output=True,
            text=True,
            timeout=5,
            check=True,
        )
        return out.stdout.strip()
    except Exception:
        return None


def record_checkpoint(
    label: str | None = None,
    library_dir: str | None = None,
    model: str | None = None,
) -> dict:
    """Run retrieval (always) + adoption/composition (best-effort) against `library_dir`,
    returning one checkpoint record — does not write anything, see `append_checkpoint`."""
    lib = open_library(library_dir)
    cell_count = len(lib)

    retrieval = run_retrieval(library_dir=library_dir).as_dict()

    try:
        adoption = run_adoption(library_dir=library_dir, model=model).as_dict()
    except (ValueError, RuntimeError) as e:
        adoption = {"skipped": str(e)}

    try:
        composition = run_composition(library_dir=library_dir, model=model).as_dict()
    except (ValueError, RuntimeError) as e:
        composition = {"skipped": str(e)}

    return {
        "label": label or f"{cell_count}-cells",
        "cell_count": cell_count,
        "commit": _git_commit(),
        "retrieval": retrieval,
        "adoption": adoption,
        "composition": composition,
    }


def append_checkpoint(record: dict, path: pathlib.Path | None = None) -> pathlib.Path:
    """Append `record` to the curve file (creating it if needed) and return its path."""
    p = path or CURVE_PATH
    existing = json.loads(p.read_text()) if p.exists() else []
    existing.append(record)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(existing, indent=2) + "\n")
    return p
