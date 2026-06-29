"""The composition agent loop, driven by the fake OpenAI-compatible client — no network.
Locks in the composition accounting (composed / used_graph / correct_via_composition)."""

import json

from _agentfake import FakeClient, _Msg, _ToolCall

from cell_eval.agent import AgentConfig
from cell_eval.composition import COMPOSITION_TOOLS, _run_one, run_composition
from cell_eval.datasets import load_jsonl
from cell_eval.library import open_library

# A real 2-node graph: abs_diff(x,y) -> clamp to [0,100].
GRAPH = {
    "id": "g",
    "nodes": {"d": "abs_diff", "b": "clamp"},
    "wires": [
        {"to": "d.a", "input": "x"},
        {"to": "d.b", "input": "y"},
        {"to": "b.x", "from": "d.result"},
        {"to": "b.lo", "const": 0},
        {"to": "b.hi", "const": 100},
    ],
    "outputs": {"out": "b.result"},
}


def test_composition_tools_include_the_graph_tool():
    names = [t["function"]["name"] for t in COMPOSITION_TOOLS]
    assert "cell_graph_run" in names and "cell_run" in names


def test_run_one_via_graph_counts_as_composition():
    cfg = AgentConfig(model="fake")
    script = [
        _Msg(tool_calls=[_ToolCall("cell_graph_run", {"graph": GRAPH, "inputs": {"x": 200, "y": 75}})]),
        _Msg(content="ANSWER: 100"),  # abs_diff(200,75)=125 -> clamp(125,0,100)=100
    ]
    r = _run_one(FakeClient(script), cfg, open_library(), {"id": "t", "prompt": "?", "expected": 100})
    assert r.answer == 100 and r.correct
    assert r.used_graph and r.composed and r.correct_via_composition
    assert r.as_dict()["correct_via_composition"] is True


def test_run_one_via_pipeline_counts_as_composition():
    # The ergonomic surface: a 2-step pipeline (positional args, "$0" chaining) instead of a
    # wire-level graph manifest. Same answer, and it counts as composition (a pipeline is a
    # host-routed graph) — this is the tool meant to lift `used_graph` off the floor.
    cfg = AgentConfig(model="fake")
    pipeline = [
        {"cell": "abs_diff", "args": [200, 75]},
        {"cell": "clamp", "args": ["$0", 0, 100]},  # abs_diff(200,75)=125 -> clamp=100
    ]
    script = [
        _Msg(tool_calls=[_ToolCall("cell_compose", {"steps": pipeline})]),
        _Msg(content="ANSWER: 100"),
    ]
    r = _run_one(FakeClient(script), cfg, open_library(), {"id": "t", "prompt": "?", "expected": 100})
    assert r.answer == 100 and r.correct
    # The pipeline registers as `used_pipeline` (not the raw-graph `used_graph`), and still
    # counts as composition.
    assert r.used_pipeline and not r.used_graph and r.composed and r.correct_via_composition


def test_run_one_chaining_two_cells_is_composition_without_graph():
    cfg = AgentConfig(model="fake")
    script = [
        _Msg(tool_calls=[_ToolCall("cell_run", {"id": "abs_diff", "args": [200, 75]})]),
        _Msg(tool_calls=[_ToolCall("cell_run", {"id": "clamp", "args": [125, 0, 100]})]),
        _Msg(content="ANSWER: 100"),
    ]
    r = _run_one(FakeClient(script), cfg, open_library(), {"id": "t", "prompt": "?", "expected": 100})
    assert r.composed and not r.used_graph  # ≥2 distinct cells, but no graph
    assert sorted(set(r.cells_run)) == ["abs_diff", "clamp"]


def test_run_one_direct_answer_is_not_composition():
    cfg = AgentConfig(model="fake")
    r = _run_one(FakeClient([_Msg(content="ANSWER: 100")]), cfg, open_library(),
                 {"id": "t", "prompt": "?", "expected": 100})
    assert r.correct and not r.composed and not r.correct_via_composition


def test_run_composition_end_to_end_offline(tmp_path, monkeypatch):
    import openai

    monkeypatch.setattr(
        openai, "OpenAI",
        lambda **_: FakeClient([
            _Msg(tool_calls=[_ToolCall("cell_graph_run", {"graph": GRAPH, "inputs": {"x": 200, "y": 75}})]),
            _Msg(content="ANSWER: 100"),
        ]),
    )
    ds = tmp_path / "c.jsonl"
    ds.write_text(json.dumps({"id": "t1", "prompt": "?", "expected": 100}) + "\n")
    rep = run_composition(dataset=str(ds), model="fake")
    o = rep.as_dict()["overall"]
    assert o["n"] == 1 and o["correct"] == 1.0 and o["composed"] == 1.0
    assert o["correct_via_composition"] == 1.0


def test_composition_dataset_is_well_formed():
    tasks = load_jsonl("composition_tasks")
    assert len(tasks) >= 5
    for t in tasks:
        assert isinstance(t["expected"], int)
        assert len(t["cells"]) >= 2  # composition tasks need at least two cells
