"""Tiered retrieval + the **margin gate** — escalation-ladder Item 1.

The ladder's retrieval rung is not "a better index", it's a *calibrated* one: answer
from the cheap tiers only when the score margin says the answer is safe, and
**escalate** the rest to the next rung (behavioural routing / synthesis / a brain)
instead of false-firing a confident-but-wrong top-1.

Tiers here:

* **Tier 1 — lexical**: the Rust `TfidfIndex` (word + char-3-gram cosine), scores kept
  (`search_scored`). Microseconds, no model.
* **Tier 2 — static-embedding rerank**: a `model2vec` potion model (static token
  vectors, ~µs per query on CPU, loads from the local HF cache) embeds the manifest
  docs and reranks tier 1's top-K by a **blended score** (`α·tfidf + (1−α)·embed`;
  α swept on the seed library — 0.25 dominates either signal alone: the embedding
  lifts adversarial 0.31 → 0.50 while the lexical term holds direct at 0.97).
* **The gate**: answer with the reranked top-1 iff `top1 − top2 ≥ θ` (the margin);
  otherwise **escalate**. θ is not a magic number: `calibrate` sweeps it and the
  **adversarial split is the calibration set** — the operating point is chosen so
  adversarial queries fall into the escalate path rather than false-firing, and the
  whole curve (answer-rate vs precision-on-answered, per split) is the deliverable,
  checked into `results/` so re-runs catch drift as the library grows.

Every number is reported on all three splits (direct / paraphrase / adversarial) —
a single blended P@1 hides exactly the failure this rung exists to catch.

Gated imports: `model2vec` (and the HF cache) load lazily in `Embedder`, so the
gate-math tests run without any model.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from .datasets import load_jsonl
from .library import open_library

# The code default runs anywhere (µs, CPU, offline — the latency floor); the
# recommended production tier-2 where an Ollama endpoint exists is nomic-embed-text —
# see baselines/embed-bakeoff.json for the seven-model bake-off backing the choice
# (nomic: best answered-coverage per millisecond; qwen3-embedding:0.6b: the quality
# ceiling when the latency budget is looser; granite-embedding measured below both).
DEFAULT_EMBED_MODEL = "minishlab/potion-retrieval-32M"
RECOMMENDED_EMBED_MODEL = "ollama:nomic-embed-text"
TIER1_K = 10  # tier 1 hands this many candidates to the rerank

# The operating points chosen by `calibrate` on the seed library (see
# baselines/tier-calibration.json for the curves backing these numbers). Re-run
# `cell-eval tiers` after growing the library; drift shows up as the adversarial
# precision-on-answered falling below the floor at the chosen θ.
BLEND_ALPHA = 0.25
# Calibrated operating points per embedder (adversarial floor 0.75, seed library) —
# the margin scale depends on the embedding geometry, so θ is per-model.
OPERATING_POINTS = {
    "minishlab/potion-retrieval-32M": 0.14,
    "cell-potion": 0.11,  # domain-trained static (cell-eval/potion/, earned in 2026-07-03)
    "ollama:granite-embedding": 0.06,
    "ollama:nomic-embed-text": 0.05,
    "ollama:embeddinggemma": 0.06,
    "ollama:qwen3-embedding:0.6b": 0.05,
    "ollama:mxbai-embed-large": 0.06,
    "ollama:snowflake-arctic-embed2": 0.09,
}
OPERATING_MARGIN = OPERATING_POINTS[DEFAULT_EMBED_MODEL]


def _doc(m: dict) -> str:
    """The embedding document for a manifest — the same fields the tf-idf indexes."""
    tags = " ".join(m.get("tags", []))
    return f"{m.get('id', '')} {m.get('summary', '')} {tags}"


class Embedder:
    """A pluggable embedding backend, selected by the model name:

    * `minishlab/potion-*` (or any HF id) → `model2vec` static vectors — **µs on
      CPU**, no server. The latency floor of the ladder's tier 2.
    * `ollama:<model>` (e.g. `ollama:nomic-embed-text`) → the local Ollama
      OpenAI-compatible `/v1/embeddings` endpoint — **~ms per query**, a real
      transformer, the widely-supported path.

    Both L2-normalise, so dot = cosine. The calibration curve is the arbiter
    between them, not the model's reputation — same gate, same floor, compare
    answered-coverage."""

    def __init__(self, model: str = DEFAULT_EMBED_MODEL):
        self.name = model
        if model == "cell-potion":  # the domain-trained artifact, kept out of git;
            # rebuild deterministically with potion/train.py (see potion/PROTOCOL.md)
            from pathlib import Path

            model = str(Path(__file__).resolve().parents[2] / "potion" / "model")
        if model.startswith("ollama:"):
            self._kind = "ollama"
            self._model = model.split(":", 1)[1]
            import urllib.request  # stdlib — no client dependency

            self._url = "http://localhost:11434/v1/embeddings"
        else:
            self._kind = "static"
            from model2vec import StaticModel  # lazy: gate-math tests need no model

            self._m = StaticModel.from_pretrained(model)

    def _encode_ollama(self, texts: list[str]):
        import json as _json
        import urllib.request

        import numpy as np

        req = urllib.request.Request(
            self._url,
            data=_json.dumps({"model": self._model, "input": texts}).encode(),
            headers={"Content-Type": "application/json"},
        )
        with urllib.request.urlopen(req, timeout=120) as r:
            data = _json.loads(r.read())
        return np.array([d["embedding"] for d in data["data"]], dtype="float32")

    def encode(self, texts: list[str]):
        import numpy as np

        v = self._encode_ollama(texts) if self._kind == "ollama" else self._m.encode(texts)
        n = np.linalg.norm(v, axis=1, keepdims=True)
        n[n == 0] = 1.0
        return v / n

    _cache: dict | None = None

    def encode_cached(self, texts: list[str]):
        """Like [`encode`] with a per-instance cache — manifest docs recur across
        queries, so each unique doc embeds once (the honest per-query cost is then
        one query embed + K cache hits)."""
        import numpy as np

        if self._cache is None:
            self._cache = {}
        misses = [t for t in texts if t not in self._cache]
        if misses:
            for t, v in zip(misses, self.encode(misses)):
                self._cache[t] = v
        return np.array([self._cache[t] for t in texts])


@dataclass
class Decision:
    """One query through the tiers: the reranked candidates and the gate's verdict."""

    query: str
    expected: list[str]
    category: str
    top: list[tuple[float, str]]  # (rerank score, cell id), best first
    margin: float

    def answered(self, theta: float) -> bool:
        return self.margin >= theta

    @property
    def top1_correct(self) -> bool:
        return bool(self.top) and self.top[0][1] in self.expected

    def tier1_correct(self, tier1_top: str | None) -> bool:
        return tier1_top in self.expected


@dataclass
class SplitStats:
    n: int = 0
    answered: int = 0
    answered_correct: int = 0

    @property
    def answer_rate(self) -> float:
        return self.answered / self.n if self.n else 0.0

    @property
    def precision_on_answered(self) -> float:
        return self.answered_correct / self.answered if self.answered else 1.0


@dataclass
class TierReport:
    embed_model: str
    theta: float
    decisions: list[Decision] = field(default_factory=list)
    tier1_top: dict[int, str | None] = field(default_factory=dict)  # decision idx -> id

    def split(self, category: str, theta: float | None = None) -> SplitStats:
        t = self.theta if theta is None else theta
        s = SplitStats()
        for d in self.decisions:
            if d.category != category:
                continue
            s.n += 1
            if d.answered(t):
                s.answered += 1
                s.answered_correct += int(d.top1_correct)
        return s

    def p_at_1(self, category: str, tier: int) -> float:
        """Ungated top-1 precision per tier (tier 1 = lexical, tier 2 = reranked)."""
        hits = n = 0
        for i, d in enumerate(self.decisions):
            if d.category != category:
                continue
            n += 1
            if tier == 2:
                hits += int(d.top1_correct)
            else:
                hits += int(self.tier1_top.get(i) in d.expected)
        return hits / n if n else 0.0

    def categories(self) -> list[str]:
        seen: list[str] = []
        for d in self.decisions:
            if d.category not in seen:
                seen.append(d.category)
        return seen

    def as_dict(self) -> dict:
        return {
            "embed_model": self.embed_model,
            "theta": self.theta,
            "splits": {
                c: {
                    "n": self.split(c).n,
                    "tier1_p1": self.p_at_1(c, 1),
                    "tier2_p1": self.p_at_1(c, 2),
                    "answer_rate": self.split(c).answer_rate,
                    "precision_on_answered": self.split(c).precision_on_answered,
                }
                for c in self.categories()
            },
        }


def expected_ids(row: dict) -> list[str]:
    e = row["expected"]
    return e if isinstance(e, list) else [e]


def run_tiers(
    dataset: str = "retrieval",
    library_dir: str | None = None,
    embed_model: str = DEFAULT_EMBED_MODEL,
    theta: float | None = None,
    k: int = TIER1_K,
    alpha: float = BLEND_ALPHA,
) -> TierReport:
    """Run every dataset query through tier 1 → the blended tier-2 rerank → the gate.
    `theta` defaults to the model's calibrated operating point."""
    lib = open_library(library_dir)
    emb = Embedder(embed_model)
    if theta is None:
        theta = OPERATING_POINTS.get(embed_model, OPERATING_MARGIN)
    report = TierReport(embed_model=emb.name, theta=theta)

    for i, row in enumerate(load_jsonl(dataset)):
        hits = lib.host.search_scored(row["query"], k)
        ids = [m["id"] for _, m in hits]
        report.tier1_top[i] = ids[0] if ids else None
        if not ids:
            report.decisions.append(
                Decision(row["query"], expected_ids(row), row["category"], [], 0.0)
            )
            continue
        t1 = [s for s, _ in hits]
        docs = emb.encode_cached([_doc(m) for _, m in hits])
        q = emb.encode([row["query"]])[0]
        scored = sorted(
            (
                (alpha * t1[j] + (1.0 - alpha) * float(docs[j] @ q), ids[j])
                for j in range(len(ids))
            ),
            reverse=True,
        )
        margin = scored[0][0] - scored[1][0] if len(scored) > 1 else scored[0][0]
        report.decisions.append(
            Decision(row["query"], expected_ids(row), row["category"], scored, margin)
        )
    return report


def calibrate(report: TierReport, floor: float = 0.75) -> dict:
    """Sweep θ and pick the operating point: the smallest margin whose
    **adversarial precision-on-answered** clears `floor` — i.e. the cheapest gate
    under which a confidently-answered adversarial query is usually *right*, with
    everything shakier escalating to the next rung. Returns the full curve (the
    deliverable) plus the chosen point."""
    grid = [round(x * 0.01, 2) for x in range(0, 41)]
    curve = []
    chosen = None
    for t in grid:
        point = {"theta": t}
        for c in report.categories():
            s = report.split(c, t)
            point[c] = {
                "answer_rate": round(s.answer_rate, 3),
                "precision_on_answered": round(s.precision_on_answered, 3),
            }
        curve.append(point)
        adv = point.get("adversarial", {})
        if chosen is None and adv.get("precision_on_answered", 0.0) >= floor:
            chosen = t
    return {
        "floor": floor,
        "chosen_theta": chosen,
        "curve": curve,
        "note": (
            "chosen_theta = smallest margin where adversarial precision-on-answered "
            ">= floor; queries below the margin escalate to the next rung"
        ),
    }
