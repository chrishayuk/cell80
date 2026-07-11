"""Retrieval eval + metrics. Deterministic — drives the real seed library, no network."""

import json
import pathlib

from cell_eval.datasets import DATASETS_DIR
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
    are the open problem the harness exists to measure.

    Floor history: 0.80 -> 0.79 at 263 cells (2026-07-06, measured 0.7934 — wide-sibling
    IDF dilution, miss list in baselines/retrieval-direct-misses-263cells-2026-07-06.txt),
    then RESTORED to 0.80 (2026-07-07) after width-aware routing landed in
    CellHost::search (width-intent queries stably rank wide cells first; measured direct
    p@1 0.8413 at 263 cells). If this floor is hit again: measure the misses, commit the
    list to baselines/, and re-price or fix search — never edit rows to pass."""
    rep = run_retrieval()
    assert rep.by_category["direct"].precision_at_1 >= 0.8


def test_checkpoint1_cohort_ratchet(tmp_path):
    """The text-only fallback ratchet (round-4 re-registration, 2026-07-11): on the
    FIXED checkpoint-1 query cohort (228 cases authored when the library had 114
    cells), text-only paraphrase P@1 must never fall below the 0.4247 the cohort
    scored at origin. This replaces the moving-mix kill gate: the library-wide
    paraphrase number falls as deliberately sibling-dense packs add harder queries
    (composition), which is not erosion — the dilution analysis (2026-07-11, in
    baselines/README.md) measured this cohort at 0.4795 after 5.7x growth, ABOVE
    origin. If this trips, growth genuinely degraded existing retrievability: stop
    and diagnose, never edit rows or the floor without a registered analysis."""
    curve = json.loads(
        (pathlib.Path(DATASETS_DIR).parent / "baselines" / "library-scale-curve.json").read_text()
    )
    ch1 = curve[0]
    assert ch1["cell_count"] == 114, "checkpoint-1 must stay the cohort anchor"
    cohort_ids = {c["case_id"] for c in ch1["retrieval"]["cases"]}
    rows = [
        json.loads(l)
        for l in (pathlib.Path(DATASETS_DIR) / "retrieval.jsonl").read_text().splitlines()
        if l.strip() and not l.startswith("#")
    ]
    subset = [r for r in rows if str(r.get("id")) in cohort_ids]
    assert len(subset) >= 200, "cohort rows must remain in retrieval.jsonl"
    ds = tmp_path / "ch1-cohort.jsonl"
    ds.write_text("\n".join(json.dumps(r) for r in subset) + "\n")
    rep = run_retrieval(dataset=str(ds))
    p1 = rep.by_category["paraphrase"].precision_at_1
    assert p1 >= 0.4247, f"checkpoint-1 cohort paraphrase P@1 {p1:.4f} fell below origin"
