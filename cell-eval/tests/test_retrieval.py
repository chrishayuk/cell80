"""Retrieval eval + metrics. Deterministic — drives the real seed library, no network."""

from cell_eval.metrics import aggregate, best_rank, hit_at_k, reciprocal_rank
from cell_eval.retrieval import run_retrieval


def test_metrics_math():
    acc = {"range_check"}
    assert best_rank(["gcd", "clamp", "range_check"], acc) == 3
    assert best_rank(["gcd", "clamp"], acc) is None
    assert reciprocal_rank(["range_check", "gcd"], acc) == 1.0
    assert reciprocal_rank(["gcd", "range_check"], acc) == 0.5
    assert reciprocal_rank(["gcd"], acc) == 0.0
    assert hit_at_k(["a", "b", "range_check"], acc, k=3) is True
    assert hit_at_k(["a", "b", "range_check"], acc, k=2) is False

    a = aggregate([1, 1, 2, None])
    assert a.n == 4
    assert a.precision_at_1 == 0.5  # two of four at rank 1
    assert a.hit_at_3 == 0.75  # three of four within top-3
    assert round(a.mrr, 4) == round((1 + 1 + 0.5 + 0) / 4, 4)


def test_retrieval_runs_over_seed_library():
    rep = run_retrieval()
    assert rep.overall.n >= 25  # the dataset has a healthy spread
    # Structural: every case carries a ranked list and a (maybe-None) rank.
    for c in rep.cases:
        assert isinstance(c.returned, list)
        assert c.rank is None or c.rank >= 1
    assert "direct" in rep.by_category


def test_direct_queries_are_strong():
    """Direct queries (library's own vocabulary) should land top-1 almost always — if this
    regresses, search broke. Paraphrase/adversarial are intentionally NOT asserted: those
    are the open problem the harness exists to measure."""
    rep = run_retrieval()
    assert rep.by_category["direct"].precision_at_1 >= 0.8
