"""The example-equipped retrieval eval — WS-F's F2 measurement.

Same cases as the plain retrieval eval, but each case with a sidecar row
(`gen-examples`) runs the **fused** path too — `lib.search(query, k,
examples=…)`, behaviour first, text order breaking ties — so the report
carries the plain-vs-equipped delta side by side. "paraphrase / equipped
(fused)" P@1 is the F2 number; the gate in the roadmap is ≥ 0.80 against the
~0.39 text-only baseline.

Reading the numbers honestly: the expected cell reproduces its own examples by
construction (see `examples_gen`), so a fused hit means the fusion + text
tiebreak beat every sibling that ALSO reproduced them (`co_match`) — and since
ties preserve text order and no cell can out-hit the expected one, the fused
rank is never worse than the plain rank per query. The interesting number is
how much of the text-unfixable same-shape residue behaviour actually recovers.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from .datasets import load_jsonl
from .library import open_library
from .metrics import Aggregate, aggregate, best_rank
from .retrieval import _acceptable


@dataclass
class ExamplesCaseResult:
    case_id: str
    query: str
    expected: list[str]
    category: str
    equipped: bool
    plain_returned: list[str]
    plain_rank: int | None
    fused_returned: list[str] | None  # None when unequipped
    fused_rank: int | None
    co_match: list[str] = field(default_factory=list)

    @property
    def deployed_rank(self) -> int | None:
        """The rank the deployed path gets: fused when examples exist, else plain."""
        return self.fused_rank if self.equipped else self.plain_rank

    def as_dict(self) -> dict:
        return {
            "case_id": self.case_id,
            "query": self.query,
            "expected": self.expected,
            "category": self.category,
            "equipped": self.equipped,
            "plain_rank": self.plain_rank,
            "fused_rank": self.fused_rank,
            "plain_returned": self.plain_returned,
            "fused_returned": self.fused_returned,
            "co_match": self.co_match,
        }


@dataclass
class RetrievalExamplesReport:
    library: str
    k: int
    examples_dataset: str
    cases: list[ExamplesCaseResult] = field(default_factory=list)

    def _subset(self, category: str | None, equipped: bool | None) -> list[ExamplesCaseResult]:
        return [
            c
            for c in self.cases
            if (category is None or c.category == category)
            and (equipped is None or c.equipped == equipped)
        ]

    def coverage(self, category: str | None = None) -> float:
        rows = self._subset(category, None)
        return sum(c.equipped for c in rows) / len(rows) if rows else 0.0

    def plain(self, category: str | None = None, equipped: bool | None = None) -> Aggregate:
        return aggregate([c.plain_rank for c in self._subset(category, equipped)])

    def fused(self, category: str | None = None) -> Aggregate:
        """Fused metrics over the equipped subset of `category`."""
        return aggregate([c.fused_rank for c in self._subset(category, True)])

    def deployed(self, category: str | None = None) -> Aggregate:
        """Fused where examples exist, plain elsewhere — the shipped behaviour."""
        return aggregate([c.deployed_rank for c in self._subset(category, None)])

    def categories(self) -> list[str]:
        return sorted({c.category for c in self.cases})

    def regressions(self) -> list[ExamplesCaseResult]:
        """Equipped cases where fusion ranked worse than plain search. The fused
        contract says this is impossible; a non-empty list is a bug report."""
        return [
            c
            for c in self.cases
            if c.equipped
            and c.plain_rank is not None
            and (c.fused_rank is None or c.fused_rank > c.plain_rank)
        ]

    def as_dict(self) -> dict:
        return {
            "eval": "retrieval-examples",
            "library": self.library,
            "k": self.k,
            "examples_dataset": self.examples_dataset,
            "overall": {
                "coverage": round(self.coverage(), 4),
                "plain": self.plain().as_dict(),
                "deployed": self.deployed().as_dict(),
            },
            "by_category": {
                cat: {
                    "coverage": round(self.coverage(cat), 4),
                    "plain_all": self.plain(cat).as_dict(),
                    "plain_equipped": self.plain(cat, True).as_dict(),
                    "fused_equipped": self.fused(cat).as_dict(),
                }
                for cat in self.categories()
            },
            "regressions": [c.as_dict() for c in self.regressions()],
            "cases": [c.as_dict() for c in self.cases],
        }


def load_sidecar(examples: str = "retrieval-examples") -> dict[str, dict]:
    """Sidecar rows keyed by case id."""
    return {row["id"]: row for row in load_jsonl(examples)}


def run_retrieval_examples(
    dataset: str = "retrieval",
    examples: str = "retrieval-examples",
    library_dir: str | None = None,
    k: int = 5,
) -> RetrievalExamplesReport:
    """Run plain + fused retrieval over `dataset`, fusing wherever the sidecar
    equips a case."""
    lib = open_library(library_dir)
    cases = load_jsonl(dataset)
    sidecar = load_sidecar(examples)

    out = RetrievalExamplesReport(
        library=str(library_dir or "seed"), k=k, examples_dataset=examples
    )
    for case in cases:
        expected = _acceptable(case)
        query = case["query"]
        case_id = str(case.get("id", query))
        plain = [r["id"] for r in lib.search(query, k)]
        row = sidecar.get(case_id)
        fused = (
            [r["id"] for r in lib.search(query, k, examples=row["examples"])]
            if row
            else None
        )
        out.cases.append(
            ExamplesCaseResult(
                case_id=case_id,
                query=query,
                expected=expected,
                category=case.get("category", "uncategorized"),
                equipped=row is not None,
                plain_returned=plain,
                plain_rank=best_rank(plain, set(expected)),
                fused_returned=fused,
                fused_rank=best_rank(fused, set(expected)) if fused is not None else None,
                co_match=list(row.get("co_match", [])) if row else [],
            )
        )
    return out
