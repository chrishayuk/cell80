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
