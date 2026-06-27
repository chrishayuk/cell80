"""The report renderers are pure formatting — test them directly (no network)."""

from cell_eval.adoption import AdoptionReport, TaskResult
from cell_eval.metrics import aggregate
from cell_eval.report import render_adoption, render_retrieval
from cell_eval.retrieval import CaseResult, RetrievalReport, run_retrieval


def test_render_retrieval_clean_run_has_no_misses_block():
    cases = [CaseResult("c1", "q", ["gcd"], "direct", ["gcd"], 1)]
    rep = RetrievalReport("x", 5, aggregate([1]), {"direct": aggregate([1])}, cases)
    out = render_retrieval(rep)
    assert "no misses" in out and "misses (top-1 wrong)" not in out


def test_render_retrieval_mentions_overall_and_misses():
    rep = run_retrieval()
    out = render_retrieval(rep)
    assert "OVERALL" in out
    assert "by category:" in out
    # The seed library has known paraphrase misses, so the misses block should show.
    assert "misses (top-1 wrong)" in out


def test_render_adoption_marks_correct_and_cell_use():
    rep = AdoptionReport(model="fake", base_url="http://x/v1")
    rep.tasks = [
        TaskResult("gcd-1", "p", 12, 12, True, ["gcd"], True, 3),
        TaskResult("max-1", "p", 42, 42, False, [], True, 1),
        TaskResult("bad-1", "p", 5, None, True, ["clamp"], False, 2),  # used a cell, still wrong
    ]
    out = render_adoption(rep)
    assert "model=fake" in out
    assert "adoption=0.67" in out  # two of three used a cell
    assert "✓" in out and "✗" in out  # both correctness marks render
    assert "cell+gcd" in out  # cell ids surfaced
