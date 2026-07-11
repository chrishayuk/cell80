"""The CLI dispatch (`cell_eval.__main__`). Retrieval runs offline; the adoption path is
exercised only as far as the config gate (no network)."""

from cell_eval.__main__ import main


def test_retrieval_text(capsys):
    assert main(["retrieval"]) == 0
    assert "OVERALL" in capsys.readouterr().out


def test_retrieval_json(capsys):
    assert main(["retrieval", "--json", "--k", "3"]) == 0
    out = capsys.readouterr().out
    assert '"eval": "retrieval"' in out and '"precision@1"' in out


def test_retrieval_fail_under_trips(capsys):
    # Seed P@1 is ~0.74, so a 0.99 threshold must fail; a low one must pass.
    assert main(["retrieval", "--fail-under", "0.99"]) == 1
    assert main(["retrieval", "--fail-under", "0.1"]) == 0


def test_adoption_no_model_exits_2(capsys, monkeypatch):
    monkeypatch.delenv("CELL_EVAL_MODEL", raising=False)
    assert main(["adoption"]) == 2
    assert "no model set" in capsys.readouterr().err


def test_adoption_success_path_renders(capsys, monkeypatch):
    """The CLI's adoption success branch (text + json), with the network stubbed out."""
    from cell_eval import adoption
    from cell_eval.adoption import AdoptionReport, TaskResult

    rep = AdoptionReport(model="fake", base_url="http://x/v1")
    rep.tasks = [TaskResult("t1", "p", 12, 12, True, ["gcd"], True, 2)]
    monkeypatch.setattr(adoption, "run_adoption", lambda **_: rep)

    assert main(["adoption", "--model", "fake"]) == 0
    assert "adoption=" in capsys.readouterr().out
    assert main(["adoption", "--model", "fake", "--json"]) == 0
    assert '"eval": "adoption"' in capsys.readouterr().out


def test_composition_no_model_exits_2(capsys, monkeypatch):
    monkeypatch.delenv("CELL_EVAL_MODEL", raising=False)
    assert main(["composition"]) == 2
    assert "no model set" in capsys.readouterr().err


def test_composition_success_path_renders(capsys, monkeypatch):
    """The CLI's composition success branch (text + json), network stubbed out."""
    from cell_eval import composition
    from cell_eval.composition import CompositionReport, TaskResult

    rep = CompositionReport(model="fake", base_url="http://x/v1")
    rep.tasks = [
        # task_id, prompt, expected, answer, composed, used_graph, used_pipeline, cells_run, correct, turns
        TaskResult("t1", "p", 100, 100, True, True, False, ["abs_diff", "clamp"], True, 3)
    ]
    monkeypatch.setattr(composition, "run_composition", lambda **_: rep)

    assert main(["composition", "--model", "fake"]) == 0
    out = capsys.readouterr().out
    assert "composed=" in out and "used_graph=" in out
    assert main(["composition", "--model", "fake", "--json"]) == 0
    assert '"eval": "composition"' in capsys.readouterr().out


def test_retrieval_examples_flag_over_tiny_library(tiny_setup, tmp_path, capsys):
    """The fused --examples branch: gen-examples then retrieval --examples, text +
    json + both --fail-under verdicts, over the synthetic library."""
    library, dataset = tiny_setup
    sidecar = tmp_path / "sidecar.jsonl"
    assert (
        main(
            ["--library", library, "gen-examples", "--dataset", dataset, "--out", str(sidecar)]
        )
        == 0
    )
    out = capsys.readouterr().out
    assert "equipped 5 case(s)" in out, out

    base = ["--library", library, "retrieval", "--dataset", dataset, "--examples", str(sidecar)]
    assert main(base) == 0
    out = capsys.readouterr().out
    assert "coverage:" in out and "fused" in out
    assert main(base + ["--json"]) == 0
    assert '"eval": "retrieval-examples"' in capsys.readouterr().out
    # Everything equipped hits rank 1 here, so a modest floor passes and an
    # impossible one trips (deployed P@1 gates the exit code).
    assert main(base + ["--fail-under", "0.5"]) == 0
    assert main(base + ["--fail-under", "1.01"]) == 1


class _StubReport:
    """as_dict()-shaped stand-in so the REAL renders run (report.py coverage)."""

    def __init__(self, model="fake", d=None):
        self.model = model
        self._d = d or {}

    def as_dict(self):
        return self._d


def test_tiers_success_and_import_error(capsys, monkeypatch):
    from cell_eval import tiers

    d = {
        "embed_model": "fake-embed",
        "theta": 0.14,
        "splits": {
            "direct": {
                "n": 2, "tier1_p1": 0.5, "tier2_p1": 1.0,
                "answer_rate": 0.5, "precision_on_answered": 1.0,
            }
        },
    }
    monkeypatch.setattr(tiers, "run_tiers", lambda **_: _StubReport(d=d))
    monkeypatch.setattr(
        tiers, "calibrate", lambda report, floor: {"chosen_theta": 0.14, "floor": floor}
    )
    assert main(["tiers"]) == 0
    out = capsys.readouterr().out
    assert "tiered retrieval" in out and "calibration: chosen θ=0.14" in out
    assert main(["tiers", "--json"]) == 0
    assert '"calibration"' in capsys.readouterr().out

    def _boom(**_):
        raise ImportError("model2vec not installed")

    monkeypatch.setattr(tiers, "run_tiers", _boom)
    assert main(["tiers"]) == 2
    assert "model2vec" in capsys.readouterr().err


def test_tier3_success_and_error(capsys, monkeypatch):
    from cell_eval import tier3

    d = {
        "model": "fake", "embed_model": "fake-embed", "theta": 0.14,
        "splits": {"paraphrase": {"n": 3, "manifests_only": 0.33, "with_probes": 0.67}},
    }
    monkeypatch.setattr(tier3, "run_disambiguation", lambda **_: _StubReport(d=d))
    assert main(["tier3"]) == 0
    assert "tier-3 disambiguation" in capsys.readouterr().out
    assert main(["tier3", "--json"]) == 0
    assert '"splits"' in capsys.readouterr().out

    def _boom(**_):
        raise ValueError("no model set")

    monkeypatch.setattr(tier3, "run_disambiguation", _boom)
    assert main(["tier3"]) == 2
    assert "no model set" in capsys.readouterr().err


def test_repair_success_and_error(capsys, monkeypatch):
    from cell_eval import repair

    d = {
        "overall": {"n": 2, "compiled": 2, "repair_at_1": 0.5},
        "by_class": {"if_no_else": {"n": 2, "compiled": 2, "repair_at_1": 0.5}},
        "results": [
            {"id": "ok", "class": "if_no_else", "correct": True, "note": ""},
            {"id": "bad", "class": "if_no_else", "correct": False, "note": "wrong value"},
        ],
    }
    monkeypatch.setattr(repair, "run_repair", lambda **_: _StubReport(d=d))
    assert main(["repair"]) == 0
    out = capsys.readouterr().out
    assert "repair eval" in out and "✗ bad" in out
    assert main(["repair", "--json"]) == 0
    assert '"repair_at_1"' in capsys.readouterr().out

    def _boom(**_):
        raise RuntimeError("no model set")

    monkeypatch.setattr(repair, "run_repair", _boom)
    assert main(["repair"]) == 2
    assert "no model set" in capsys.readouterr().err


def test_potion_pairs_exit_codes_and_error(capsys, monkeypatch, tmp_path):
    from cell_eval import potion

    clean = {"validation_problems": [], "failed_cells": []}
    monkeypatch.setattr(potion, "generate_pairs", lambda **_: ([{"q": "x"}], dict(clean)))
    monkeypatch.setattr(potion, "write_pairs", lambda rows, stats, out: None)
    out_path = str(tmp_path / "pairs.jsonl")
    assert main(["potion-pairs", "--model", "fake", "--out", out_path]) == 0
    assert '"out"' in capsys.readouterr().out

    dirty = {"validation_problems": ["p"], "failed_cells": ["c"]}
    monkeypatch.setattr(potion, "generate_pairs", lambda **_: ([], dict(dirty)))
    assert main(["potion-pairs", "--model", "fake", "--out", out_path, "--cells", "a,b"]) == 1
    capsys.readouterr()

    def _boom(**_):
        raise ValueError("no model set")

    monkeypatch.setattr(potion, "generate_pairs", _boom)
    assert main(["potion-pairs"]) == 2
    assert "no model set" in capsys.readouterr().err


def test_cells_snapshot_and_compare_over_tiny_library(tiny_setup, tmp_path, capsys):
    import json as _json

    library, _ = tiny_setup
    snap = tmp_path / "snap.json"
    # NB: cells-snapshot declares its own --library, so the flag must follow the
    # subcommand (the subparser's default would clobber the global flag's value).
    assert main(["cells-snapshot", "--library", library, "--out", str(snap)]) == 0
    assert "snapshot:" in capsys.readouterr().out

    # Identical snapshots compare clean (text + json)...
    assert main(["cells-compare", str(snap), str(snap)]) == 0
    capsys.readouterr()
    assert main(["cells-compare", str(snap), str(snap), "--json"]) == 0
    assert '"identical": true' in capsys.readouterr().out
    # ...and any divergence exits 1.
    data = _json.loads(snap.read_text())
    outputs = data[sorted(data)[0]]["outputs"]
    outputs[sorted(outputs)[0]] = "tampered-row"
    tampered = tmp_path / "tampered.json"
    tampered.write_text(_json.dumps(data))
    assert main(["cells-compare", str(snap), str(tampered)]) == 1
    capsys.readouterr()


def test_curve_appends_a_checkpoint(capsys, monkeypatch, tmp_path):
    from cell_eval import curve

    monkeypatch.setattr(
        curve, "record_checkpoint", lambda **_: {"label": "tiny", "cells": 6}
    )
    monkeypatch.setattr(curve, "append_checkpoint", lambda record, out: out or "curve.json")
    assert main(["curve", "--label", "tiny", "--out", str(tmp_path / "c.json")]) == 0
    captured = capsys.readouterr()
    assert '"label": "tiny"' in captured.out and "appended checkpoint" in captured.err
