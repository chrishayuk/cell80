"""Locations for disposable cell-native experiment artifacts."""

from __future__ import annotations

import os
from pathlib import Path


HERE = Path(__file__).resolve().parent
ROOT = Path(os.environ.get(
    "CELL80_CELL_NATIVE_ARTIFACT_ROOT",
    os.environ.get("CELL80_ARTIFACT_ROOT", HERE / "artifacts"),
)).expanduser()


def _input(kind: str, value: str | Path) -> Path:
    path = Path(value).expanduser()
    if path.is_absolute() or path.parent != Path("."):
        return path
    artifact = ROOT / kind / path
    if artifact.exists():
        return artifact
    legacy = HERE / path
    return legacy if legacy.exists() else artifact


def _output(kind: str, name: str | Path) -> Path:
    path = Path(name)
    if path.is_absolute() or path.parent != Path("."):
        path.parent.mkdir(parents=True, exist_ok=True)
        return path
    parent = ROOT / kind
    parent.mkdir(parents=True, exist_ok=True)
    return parent / path


def dataset_input(value: str | Path) -> Path:
    return _input("datasets", value)


def dataset_output(name: str | Path) -> Path:
    return _output("datasets", name)


def checkpoint_input(value: str | Path) -> Path:
    return _input("checkpoints", value)


def checkpoint_output(name: str | Path) -> Path:
    return _output("checkpoints", name)


def log_output(name: str | Path) -> Path:
    return _output("logs", name)
