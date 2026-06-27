"""Adoption-eval pieces that don't need a live model: answer parsing, config, datasets.
The network path (`run_adoption`) is exercised by you against Ollama, not in CI."""

import pytest

from cell_eval.adoption import AdoptionConfig, _parse_answer
from cell_eval.datasets import load_jsonl


def test_parse_answer_takes_last_integer():
    assert _parse_answer("blah\nANSWER: 12") == 12
    assert _parse_answer("ANSWER: 1\n...\nANSWER: 0") == 0  # last wins
    assert _parse_answer("answer: -5") == -5  # case-insensitive
    assert _parse_answer("no answer here") is None


def test_config_requires_a_model(monkeypatch):
    monkeypatch.delenv("CELL_EVAL_MODEL", raising=False)
    with pytest.raises(ValueError):
        AdoptionConfig.from_env()
    cfg = AdoptionConfig.from_env("qwen2.5")
    assert cfg.model == "qwen2.5"
    assert cfg.base_url.endswith("/v1")  # Ollama OpenAI-compatible default


def test_tasks_dataset_is_well_formed():
    tasks = load_jsonl("tasks")
    assert len(tasks) >= 5
    for t in tasks:
        assert isinstance(t["expected"], int)
        assert t["prompt"] and t["id"]
