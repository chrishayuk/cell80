"""Tiny JSONL dataset loader shared by both evals.

Datasets live next to this package in `cell-eval/datasets/` and are plain JSONL so they
diff cleanly in review and are trivial to extend — one case per line.
"""

from __future__ import annotations

import json
import pathlib

DATASETS_DIR = pathlib.Path(__file__).resolve().parents[2] / "datasets"


def load_jsonl(name_or_path: str | pathlib.Path) -> list[dict]:
    """Load a `.jsonl` file. A bare name resolves under `datasets/`; blank lines and
    `#`-comment lines are skipped so datasets can be annotated."""
    p = pathlib.Path(name_or_path)
    if not p.exists():
        cand = DATASETS_DIR / name_or_path
        cand = cand if cand.suffix else cand.with_suffix(".jsonl")
        p = cand
    if not p.exists():
        raise FileNotFoundError(f"dataset not found: {name_or_path} (looked in {DATASETS_DIR})")
    rows = []
    for i, line in enumerate(p.read_text().splitlines(), 1):
        s = line.strip()
        if not s or s.startswith("#"):
            continue
        try:
            rows.append(json.loads(s))
        except json.JSONDecodeError as e:
            raise ValueError(f"{p}:{i}: bad JSON: {e}") from e
    return rows
