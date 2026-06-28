"""The adoption agent loop, driven by a fake OpenAI-compatible client — no network.

This locks in the loop's behaviour (multi-turn tool dispatch, answer parsing, adoption
accounting, error capture) so a refactor can't silently break it. The live-model path is
the same code; only the client differs."""

import json
import sys

import pytest
from _agentfake import FakeClient, _Msg, _ToolCall

from cell_eval.adoption import AdoptionConfig, _run_one, run_adoption
from cell_eval.library import open_library


def test_run_one_search_inspect_run_answer():
    cfg = AdoptionConfig(model="fake")
    lib = open_library()
    script = [
        _Msg(tool_calls=[_ToolCall("cell_search", {"query": "gcd"})]),
        _Msg(tool_calls=[_ToolCall("cell_run", {"id": "gcd", "args": [48, 36]})]),
        _Msg(content="The gcd is 12.\nANSWER: 12"),
    ]
    r = _run_one(FakeClient(script), cfg, lib, {"id": "gcd-1", "prompt": "?", "expected": 12})
    assert r.answer == 12 and r.correct and r.used_cell and r.correct_via_cell
    assert r.cells_run == ["gcd"] and r.turns == 3
    assert r.as_dict()["correct_via_cell"] is True


def test_run_one_direct_answer_counts_as_no_adoption():
    cfg = AdoptionConfig(model="fake")
    r = _run_one(
        FakeClient([_Msg(content="easy. ANSWER: 42")]),
        cfg,
        open_library(),
        {"id": "max-1", "prompt": "?", "expected": 42},
    )
    assert r.answer == 42 and r.correct and not r.used_cell and r.turns == 1


def test_run_one_records_endpoint_error():
    cfg = AdoptionConfig(model="fake")
    r = _run_one(
        FakeClient([RuntimeError("boom")]),
        cfg,
        open_library(),
        {"id": "x", "prompt": "?", "expected": 1},
    )
    assert r.error and "boom" in r.error and r.answer is None and not r.correct


def test_run_adoption_end_to_end_offline(tmp_path, monkeypatch):
    import openai

    monkeypatch.setattr(openai, "OpenAI", lambda **_: FakeClient([_Msg(content="ANSWER: 12")]))
    ds = tmp_path / "one.jsonl"
    ds.write_text(json.dumps({"id": "t1", "prompt": "gcd 48 36", "expected": 12}) + "\n")
    rep = run_adoption(dataset=str(ds), model="fake")
    o = rep.as_dict()["overall"]
    assert o["n"] == 1 and o["correct"] == 1.0 and o["adoption"] == 0.0


def test_run_one_tolerates_malformed_tool_arguments():
    cfg = AdoptionConfig(model="fake")
    bad = _ToolCall("cell_list", {})
    bad.function.arguments = "{not json"  # force JSONDecodeError → args = {}
    script = [_Msg(tool_calls=[bad]), _Msg(content="ANSWER: 1")]
    r = _run_one(FakeClient(script), cfg, open_library(), {"id": "x", "prompt": "?", "expected": 1})
    assert r.answer == 1 and r.turns == 2


def test_from_env_reads_overrides(monkeypatch):
    monkeypatch.setenv("CELL_EVAL_MODEL", "m1")
    monkeypatch.setenv("CELL_EVAL_BASE_URL", "http://h:1/v1")
    monkeypatch.setenv("CELL_EVAL_API_KEY", "k")
    monkeypatch.setenv("CELL_EVAL_MAX_TURNS", "3")
    cfg = AdoptionConfig.from_env()
    assert (cfg.model, cfg.base_url, cfg.api_key, cfg.max_turns) == ("m1", "http://h:1/v1", "k", 3)


def test_run_adoption_without_openai_is_a_clear_error(monkeypatch):
    monkeypatch.setitem(sys.modules, "openai", None)  # make `import openai` raise
    with pytest.raises(RuntimeError):
        run_adoption(model="fake")
