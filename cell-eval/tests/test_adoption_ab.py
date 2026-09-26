from _agentfake import FakeClient, _Msg, _ToolCall

from cell_eval.adoption_ab import (
    ABTaskResult,
    AdoptionABReport,
    _run_one,
    leakage_collisions,
)
from cell_eval.agent import AgentConfig


class FakeLibrary:
    def search_decision(self, query, limit):
        manifest = {"id": "gcd", "summary": "gcd", "tags": [], "signature": "run"}
        return {
            "results": [manifest],
            "selected": manifest,
            "confidence": {
                "method": "hybrid_margin",
                "status": "confident",
                "abstained": False,
            },
        }

    def inspect(self, cell_id):
        return {"id": cell_id, "signature": "run(a: u16, b: u16) -> u16"}

    def run(self, cell_id, args):
        return {"result": 12}


def test_heldout_dataset_has_no_exact_prompt_leakage():
    assert leakage_collisions() == []


def test_ab_episode_records_decision_run_and_correctness():
    script = [
        _Msg(tool_calls=[_ToolCall("cell_search", {"query": "shared factor"})]),
        _Msg(tool_calls=[_ToolCall("cell_run", {"id": "gcd", "args": [48, 36]})]),
        _Msg(content="ANSWER: 12"),
    ]
    result = _run_one(
        FakeClient(script),
        AgentConfig(model="fake"),
        FakeLibrary(),
        "hybrid",
        {"id": "t", "prompt": "?", "expected": 12, "cell": "gcd"},
    )

    assert result.correct and result.expected_cell_run and not result.wrong_runs
    assert result.search_decisions[0]["selected"] == "gcd"
    assert result.search_decisions[0]["method"] == "hybrid_margin"


def test_ab_report_summarises_wrong_runs_and_abstention_recovery():
    lexical = ABTaskResult(
        arm="lexical",
        task_id="a",
        expected=12,
        expected_cell="gcd",
        answer=12,
        correct=True,
        cells_run=["gcd"],
        inspected=[],
        search_decisions=[
            {"abstained": False, "latency_ms": 1.0, "method": "lexical_fallback"}
        ],
        turns=3,
        episode_ms=10.0,
    )
    hybrid = ABTaskResult(
        arm="hybrid",
        task_id="a",
        expected=12,
        expected_cell="gcd",
        answer=12,
        correct=True,
        cells_run=["max", "gcd"],
        inspected=["gcd"],
        search_decisions=[
            {"abstained": True, "latency_ms": 2.0, "method": "hybrid_margin"}
        ],
        turns=4,
        episode_ms=20.0,
    )
    report = AdoptionABReport(
        model="fake",
        base_url="local",
        dataset="adoption_ab_heldout",
        dataset_sha256="abc",
        retrieval_methods={"lexical": "lexical_fallback", "hybrid": "hybrid_margin"},
        results=[lexical, hybrid],
    ).as_dict()

    assert report["arms"]["lexical"]["correct"] == 1.0
    assert report["arms"]["hybrid"]["wrong_runs"] == 1
    assert report["arms"]["hybrid"]["abstention_recovery_rate"] == 1.0
    assert report["retrieval_methods"]["hybrid"] == "hybrid_margin"
