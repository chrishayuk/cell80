"""Dataset loading edge cases + seed-library resolution (env / walk-up / not-found)."""

import json

import pytest

import cell_eval.library as L
from cell_eval.datasets import load_jsonl
from cell_eval.metrics import aggregate
from cell_eval.retrieval import _acceptable


# ── datasets ──────────────────────────────────────────────────────────────────
def test_load_jsonl_skips_comments_and_blanks(tmp_path):
    p = tmp_path / "d.jsonl"
    p.write_text("# a comment\n\n" + json.dumps({"a": 1}) + "\n")
    assert load_jsonl(str(p)) == [{"a": 1}]


def test_load_jsonl_missing_raises(tmp_path):
    with pytest.raises(FileNotFoundError):
        load_jsonl(str(tmp_path / "nope.jsonl"))


def test_load_jsonl_bad_json_raises(tmp_path):
    p = tmp_path / "bad.jsonl"
    p.write_text("{not valid json}\n")
    with pytest.raises(ValueError):
        load_jsonl(str(p))


# ── library resolution ──────────────────────────────────────────────────────────
def test_seed_dir_from_env(monkeypatch, tmp_path):
    monkeypatch.setenv("CELL_LIBRARY", str(tmp_path))
    assert L.seed_library_dir() == tmp_path


def test_seed_dir_walk_up(monkeypatch):
    monkeypatch.delenv("CELL_LIBRARY", raising=False)
    d = L.seed_library_dir()
    assert d.name == "cells" and d.is_dir()


def test_seed_dir_not_found(monkeypatch):
    monkeypatch.delenv("CELL_LIBRARY", raising=False)
    monkeypatch.setattr(L, "__file__", "/nonexistent/deep/path/x.py")
    with pytest.raises(FileNotFoundError):
        L.seed_library_dir()


# ── metric / retrieval edges ────────────────────────────────────────────────────
def test_aggregate_empty():
    a = aggregate([])
    assert a.n == 0 and a.mrr == 0.0 and a.precision_at_1 == 0.0


def test_acceptable_normalises_and_validates():
    assert _acceptable({"expected": "gcd"}) == ["gcd"]
    assert _acceptable({"expected": ["a", "b"]}) == ["a", "b"]
    with pytest.raises(ValueError):
        _acceptable({"id": "x"})  # no 'expected'
