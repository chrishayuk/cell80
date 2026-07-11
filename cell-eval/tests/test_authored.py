"""The agent-authored-example lane, offline: parsing, validity, the behavioural
equivalence machinery (incl. the false-unique detector), the report math, and the
CLI dispatch — a scripted 'model' stands in so no network or weights are needed."""

import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))

from cell_eval.__main__ import main
from cell_eval.authored import (
    AuthoredCase,
    parse_examples_reply,
    run_authored,
)
from cell_eval.report import render_authored


def test_parse_examples_reply_shapes():
    ok = parse_examples_reply('{"examples": [{"in": [3, 7], "out": 3}]}', 2)
    assert ok == [([3, 7], 3)]
    # Fenced/prosey replies still parse; junk shapes do not.
    assert parse_examples_reply('sure! ```json\n{"examples":[{"in":[5],"out":6}]}\n```', 1) == [
        ([5], 6)
    ]
    assert parse_examples_reply(None, 2) is None
    assert parse_examples_reply("no json here", 2) is None
    assert parse_examples_reply('{"examples": []}', 2) is None
    assert parse_examples_reply('{"examples": [{"in": [3], "out": 3}]}', 2) is None  # arity
    assert parse_examples_reply('{"examples": [{"in": [3, 99999], "out": 3}]}', 2) is None
    assert parse_examples_reply('{"examples": [{"in": [3, 7], "out": -1}]}', 2) is None
    # More than 3 examples are clipped, not rejected.
    many = json.dumps({"examples": [{"in": [i, i], "out": i} for i in range(5)]})
    assert len(parse_examples_reply(many, 2)) == 3


class _AuthorClient:
    """Answers per scripted query: a dict of query → reply text (or Exception)."""

    def __init__(self, script: dict):
        outer = self

        class _C:
            def create(self, **kw):
                q = kw["messages"][1]["content"].splitlines()[0].removeprefix("The function: ")
                item = outer._script[q]
                if isinstance(item, Exception):
                    raise item
                msg = type("M", (), {"content": item})()
                return type("R", (), {"choices": [type("C2", (), {"message": msg})()]})()

        self._script = script
        self.chat = type("Chat", (), {"completions": _C()})()


def _ex(rows):
    return json.dumps({"examples": rows})


def test_run_authored_covers_validity_equivalence_and_retrieval(tiny_setup, tmp_path):
    library, dataset = tiny_setup
    # An authored-lane dataset over the tiny library's VALUE cells only.
    rows = [
        {"id": "a-lo", "query": "the smaller of a pair", "expected": "pick_lo", "category": "paraphrase"},
        {"id": "a-hi", "query": "the larger of a pair", "expected": "pick_hi", "category": "paraphrase"},
        {"id": "a-bad", "query": "smaller value wrong output", "expected": "pick_lo", "category": "paraphrase"},
        {"id": "a-junk", "query": "smaller value junk reply", "expected": "pick_lo", "category": "paraphrase"},
        # State-cell case: outside the schema-free population, never asked.
        {"id": "a-state", "query": "combine two fields", "expected": "f_add", "category": "paraphrase"},
    ]
    ds = tmp_path / "authored-cases.jsonl"
    ds.write_text("\n".join(json.dumps(r) for r in rows) + "\n")
    # Oracle sidecar: only for the comparison column; give it one row.
    sidecar = tmp_path / "sidecar.jsonl"
    sidecar.write_text(
        json.dumps({"id": "a-lo", "examples": [{"in": [3, 7], "out": 3}], "co_match": [], "form": "in"})
        + "\n"
    )

    client = _AuthorClient(
        {
            # Correct, discriminating examples → valid, expected uniquely pinned.
            "the smaller of a pair": _ex([{"in": [3, 7], "out": 3}, {"in": [9, 4], "out": 4}]),
            "the larger of a pair": _ex([{"in": [3, 7], "out": 7}, {"in": [9, 4], "out": 9}]),
            # WRONG outputs (max instead of min): invalid — and pick_hi satisfies
            # them, so the top behavioural class is a singleton that is NOT the
            # expected cell: the dangerous false-unique failure.
            "smaller value wrong output": _ex([{"in": [3, 7], "out": 7}, {"in": [9, 4], "out": 9}]),
            # Unparseable reply → not well-formed, no examples to route on.
            "smaller value junk reply": "I think min is nice",
        }
    )
    rep = run_authored(
        dataset=str(ds), examples=str(sidecar), library_dir=library,
        model="fake", client=client,
    )
    # Population: 4 value-cell cases of 5 total; the state case never reaches the model.
    assert rep.population == 4 and rep.total_cases == 5
    by = {c.case_id: c for c in rep.cases}
    assert set(by) == {"a-lo", "a-hi", "a-bad", "a-junk"}

    lo = by["a-lo"]
    assert lo.well_formed and lo.valid
    assert lo.top_class == ["pick_lo"] and lo.expected_in_top_class
    assert lo.authored_rank == 1 and lo.oracle_rank == 1
    assert not lo.false_unique

    hi = by["a-hi"]
    assert hi.valid and hi.top_class == ["pick_hi"] and hi.authored_rank == 1
    assert hi.oracle_rank is None  # no sidecar row for it

    bad = by["a-bad"]
    assert bad.well_formed and not bad.valid
    assert bad.top_class == ["pick_hi"] and bad.false_unique, bad.top_class
    assert bad.authored_rank != 1  # confidently wrong — the correlated failure

    junk = by["a-junk"]
    assert not junk.well_formed and junk.examples == []
    assert junk.authored_rank is None and junk.top_class == []

    d = rep.as_dict()
    o = d["overall"]
    assert o["n"] == 4 and o["well_formed"] == 0.75 and o["valid"] == 0.5
    assert o["false_unique_rate"] == 0.25
    assert o["correlated_failure"] == 0.5  # a-bad and a-junk
    text = render_authored(rep)
    assert "authored-examples eval" in text and "false_unique" in text


def test_max_cases_caps_model_calls_but_counts_population(tiny_setup, tmp_path):
    library, dataset = tiny_setup
    rows = [
        {"id": f"m-{i}", "query": "the smaller of a pair", "expected": "pick_lo", "category": "direct"}
        for i in range(4)
    ]
    ds = tmp_path / "cap.jsonl"
    ds.write_text("\n".join(json.dumps(r) for r in rows) + "\n")
    sidecar = tmp_path / "empty-sidecar.jsonl"
    sidecar.write_text("\n")
    client = _AuthorClient({"the smaller of a pair": _ex([{"in": [3, 7], "out": 3}])})
    rep = run_authored(
        dataset=str(ds), examples=str(sidecar), library_dir=library,
        model="fake", client=client, max_cases=2,
    )
    assert rep.population == 4 and len(rep.cases) == 2


def test_authored_cli_dispatch(tiny_setup, tmp_path, capsys, monkeypatch):
    from cell_eval import authored as mod

    rep = type(
        "R", (), {"as_dict": lambda self: {"eval": "authored-examples", "model": "fake",
                                           "library": "seed", "k": 5, "population": 1,
                                           "total_cases": 2, "population_fraction": 0.5,
                                           "overall": _SPLIT, "by_category": {"direct": _SPLIT}}}
    )()
    monkeypatch.setattr(mod, "run_authored", lambda **_: rep)
    assert main(["authored", "--model", "fake"]) == 0
    assert "authored-examples eval" in capsys.readouterr().out
    assert main(["authored", "--model", "fake", "--json"]) == 0
    assert '"authored-examples"' in capsys.readouterr().out

    def _boom(**_):
        raise ValueError("no model set")

    monkeypatch.setattr(mod, "run_authored", _boom)
    assert main(["authored"]) == 2
    assert "no model set" in capsys.readouterr().err


_SPLIT = {
    "n": 1, "well_formed": 1.0, "valid": 1.0, "false_unique_rate": 0.0,
    "ambiguity_rate": 0.0, "correlated_failure": 0.0,
    "plain": {"n": 1, "precision@1": 0.0, "hit@3": 1.0, "hit@5": 1.0, "mrr": 0.5},
    "oracle": {"n": 1, "precision@1": 1.0, "hit@3": 1.0, "hit@5": 1.0, "mrr": 1.0},
    "authored": {"n": 1, "precision@1": 1.0, "hit@3": 1.0, "hit@5": 1.0, "mrr": 1.0},
}
