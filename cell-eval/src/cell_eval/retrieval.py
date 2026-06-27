"""The deterministic retrieval-precision eval.

For each query we ask the library to `search` and check where the acceptable cell(s) land
in the ranking. No model, no network — this reads index quality directly. It's the eval
that tells you whether the *paraphrase brittleness* the roadmap warns about is real for a
given library, and it's the gate for the type-led index work (roadmap item 3).
"""

from __future__ import annotations

from dataclasses import dataclass, field

from .datasets import load_jsonl
from .library import open_library
from .metrics import Aggregate, aggregate, best_rank


@dataclass
class CaseResult:
    case_id: str
    query: str
    expected: list[str]
    category: str
    returned: list[str]  # ranked ids actually returned (top-k)
    rank: int | None  # best rank of an acceptable id, or None

    def as_dict(self) -> dict:
        return {
            "case_id": self.case_id,
            "query": self.query,
            "expected": self.expected,
            "category": self.category,
            "returned": self.returned,
            "rank": self.rank,
            "hit@1": self.rank == 1,
            "hit@3": self.rank is not None and self.rank <= 3,
        }


@dataclass
class RetrievalReport:
    library: str
    k: int
    overall: Aggregate
    by_category: dict[str, Aggregate]
    cases: list[CaseResult] = field(default_factory=list)

    def as_dict(self) -> dict:
        return {
            "eval": "retrieval",
            "library": self.library,
            "k": self.k,
            "overall": self.overall.as_dict(),
            "by_category": {c: a.as_dict() for c, a in self.by_category.items()},
            "cases": [c.as_dict() for c in self.cases],
        }

    def misses(self) -> list[CaseResult]:
        """Cases where the top-1 result was wrong — the actionable list."""
        return [c for c in self.cases if c.rank != 1]


def _acceptable(case: dict) -> list[str]:
    exp = case.get("expected")
    if exp is None:
        raise ValueError(f"case {case.get('id')!r} has no 'expected'")
    return [exp] if isinstance(exp, str) else list(exp)


def run_retrieval(
    dataset: str = "retrieval",
    library_dir: str | None = None,
    k: int = 5,
) -> RetrievalReport:
    """Run the retrieval eval over `dataset` against `library_dir` (default: seed lib)."""
    lib = open_library(library_dir)
    cases = load_jsonl(dataset)

    results: list[CaseResult] = []
    for case in cases:
        expected = _acceptable(case)
        query = case["query"]
        returned = [r["id"] for r in lib.search(query, k)]
        rank = best_rank(returned, set(expected))
        results.append(
            CaseResult(
                case_id=str(case.get("id", query)),
                query=query,
                expected=expected,
                category=case.get("category", "uncategorized"),
                returned=returned,
                rank=rank,
            )
        )

    overall = aggregate([c.rank for c in results])
    cats = sorted({c.category for c in results})
    by_cat = {
        cat: aggregate([c.rank for c in results if c.category == cat]) for cat in cats
    }
    return RetrievalReport(
        library=str(library_dir or "seed"),
        k=k,
        overall=overall,
        by_category=by_cat,
        cases=results,
    )
