"""Retrieval metrics.

Each query has one or more *acceptable* cell ids (usually one). Given the ranked list of
returned ids we compute the standard small set:

* **hit@k**   — is an acceptable id within the top-k? (== recall@k when there's one
  relevant cell, which is the common case here)
* **rank**    — 1-based rank of the best acceptable id, or `None` if it never appears
* **reciprocal rank** — 1/rank (0 if absent); the per-query term of MRR

`precision@1` is just `hit@1`. We deliberately don't report a graded precision@k: with a
single relevant cell per query, precision@k just rescales hit@k by 1/k and adds no signal.
"""

from __future__ import annotations

from dataclasses import dataclass


def best_rank(ranked_ids: list[str], acceptable: set[str]) -> int | None:
    """1-based rank of the first acceptable id in `ranked_ids`, or None."""
    for i, cid in enumerate(ranked_ids, 1):
        if cid in acceptable:
            return i
    return None


def reciprocal_rank(ranked_ids: list[str], acceptable: set[str]) -> float:
    r = best_rank(ranked_ids, acceptable)
    return 0.0 if r is None else 1.0 / r


def hit_at_k(ranked_ids: list[str], acceptable: set[str], k: int) -> bool:
    r = best_rank(ranked_ids[:k], acceptable)
    return r is not None


@dataclass
class Aggregate:
    """Aggregate retrieval metrics over a set of queries."""

    n: int
    precision_at_1: float
    hit_at_3: float
    hit_at_5: float
    mrr: float

    def as_dict(self) -> dict:
        return {
            "n": self.n,
            "precision@1": round(self.precision_at_1, 4),
            "hit@3": round(self.hit_at_3, 4),
            "hit@5": round(self.hit_at_5, 4),
            "mrr": round(self.mrr, 4),
        }


def aggregate(ranks: list[int | None]) -> Aggregate:
    """Aggregate from a list of best-ranks (one per query; None = not found)."""
    n = len(ranks)
    if n == 0:
        return Aggregate(0, 0.0, 0.0, 0.0, 0.0)
    p1 = sum(1 for r in ranks if r == 1) / n
    h3 = sum(1 for r in ranks if r is not None and r <= 3) / n
    h5 = sum(1 for r in ranks if r is not None and r <= 5) / n
    mrr = sum(0.0 if r is None else 1.0 / r for r in ranks) / n
    return Aggregate(n, p1, h3, h5, mrr)
