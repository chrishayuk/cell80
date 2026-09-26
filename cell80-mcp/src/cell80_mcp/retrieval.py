"""Live tier-2 retrieval: lexical evidence, cached catalog rerank, margin gate.

The embedding dependency is deliberately optional. A server with the ``smart`` extra
and a locally cached model gets hybrid ranking; every other installation falls back to
the calibrated lexical decision and reports why in its confidence metadata. Search
never downloads a model as a side effect.
"""

from __future__ import annotations

import json
import math
import urllib.request
from collections.abc import Callable, Sequence
from pathlib import Path

DEFAULT_MODEL = "minishlab/potion-retrieval-32M"
DEFAULT_ALPHA = 0.75
DEFAULT_TYPE_BONUS = 0.20
DEFAULT_TYPE_RANK_K = 50
DEFAULT_HYBRID_MARGIN = 0.18
DEFAULT_LEXICAL_MARGIN = 0.07


def manifest_document(manifest: dict) -> str:
    """Build the same compact semantic document used by the evaluator."""
    tags = " ".join(manifest.get("tags", []))
    return f"{manifest.get('id', '')} {manifest.get('summary', '')} {tags}"


def _normalise(vector: Sequence[float]) -> list[float]:
    values = [float(value) for value in vector]
    norm = math.sqrt(sum(value * value for value in values))
    return [value / norm for value in values] if norm else values


def _cosine(left: Sequence[float], right: Sequence[float]) -> float:
    return sum(a * b for a, b in zip(left, right))


class EmbeddingBackend:
    """Lazy static-model or local-Ollama embeddings with a uniform interface."""

    def __init__(self, model: str):
        self.name = model
        if model.startswith("ollama:"):
            self._kind = "ollama"
            self._model = model.split(":", 1)[1]
            self._url = "http://localhost:11434/v1/embeddings"
        else:
            self._kind = "static"
            from huggingface_hub import snapshot_download
            from model2vec import StaticModel

            # A live search must not unexpectedly reach the network. Operators can
            # pre-populate the cache by installing the smart extra/model explicitly.
            local_model = (
                model
                if Path(model).exists()
                else snapshot_download(repo_id=model, local_files_only=True)
            )
            self._static = StaticModel.from_pretrained(
                local_model, normalize=True, force_download=False
            )

    def encode(self, texts: list[str]) -> list[list[float]]:
        if self._kind == "static":
            vectors = self._static.encode(texts, use_multiprocessing=False)
            return [_normalise(vector) for vector in vectors]

        request = urllib.request.Request(
            self._url,
            data=json.dumps({"model": self._model, "input": texts}).encode(),
            headers={"Content-Type": "application/json"},
        )
        with urllib.request.urlopen(request, timeout=120) as response:
            payload = json.loads(response.read())
        return [_normalise(item["embedding"]) for item in payload["data"]]


class HybridRetriever:
    """Fuse lexical, cached catalog-semantic, and type-led rank evidence."""

    def __init__(
        self,
        host,
        manifests: Callable[[], list[dict]],
        *,
        model: str = DEFAULT_MODEL,
        alpha: float = DEFAULT_ALPHA,
        type_bonus: float = DEFAULT_TYPE_BONUS,
        type_rank_k: int = DEFAULT_TYPE_RANK_K,
        embedder=None,
    ):
        self.host = host
        self._manifests = manifests
        self.model = getattr(embedder, "name", model)
        self.alpha = alpha
        self.type_bonus = type_bonus
        self.type_rank_k = type_rank_k
        self._embedder = embedder
        self._catalog: list[dict] | None = None
        self._vectors: dict[str, list[float]] | None = None
        self._unavailable: str | None = None

    @property
    def unavailable_reason(self) -> str | None:
        return self._unavailable

    @property
    def catalog_size(self) -> int:
        return len(self._catalog or [])

    def _prepare(self) -> bool:
        if self._unavailable is not None:
            return False
        if self._vectors is not None:
            return True
        if self.model.lower() in {"off", "none", "disabled"}:
            self._unavailable = "hybrid retrieval disabled by CELL_RETRIEVAL_MODEL"
            return False
        try:
            if self._embedder is None:
                self._embedder = EmbeddingBackend(self.model)
            manifests = self._manifests()
            documents = [manifest_document(manifest) for manifest in manifests]
            vectors = self._embedder.encode(documents) if documents else []
            if len(vectors) != len(manifests):
                raise ValueError(
                    f"embedder returned {len(vectors)} vectors for {len(manifests)} manifests"
                )
            self._vectors = {
                manifest["id"]: vector for manifest, vector in zip(manifests, vectors)
            }
            self._catalog = manifests
            return True
        except Exception as error:  # optional dependency/model/server unavailable
            self._unavailable = f"{type(error).__name__}: {error}"
            return False

    def rank(self, query: str, limit: int) -> list[dict] | None:
        """Return hybrid score records, or ``None`` when tier 2 is unavailable."""
        if not self._prepare():
            return None
        # A lexical hit establishes that the request belongs in this library. Once it
        # does, score the whole cached catalog so a paraphrase is not trapped inside a
        # brittle lexical top-K. The type-led reciprocal-rank feature contributes the
        # signature/behaviour cues already learned by CellHost.
        hits = list(self.host.search_scored(query, max(len(self._vectors), 2)))
        if not hits:
            return []
        try:
            query_vector = self._embedder.encode([query])[0]
            if self._vectors and len(query_vector) != len(
                next(iter(self._vectors.values()))
            ):
                raise ValueError("query and manifest embedding dimensions differ")
        except Exception as error:
            self._unavailable = f"{type(error).__name__}: {error}"
            return None
        lexical = {manifest["id"]: float(score) for score, manifest in hits}
        type_rank = {
            manifest["id"]: rank
            for rank, manifest in enumerate(self.host.search(query, self.type_rank_k))
        }
        ranked = []
        for manifest in self._catalog:
            cell_id = manifest["id"]
            semantic = _cosine(self._vectors[manifest["id"]], query_vector)
            lexical_score = lexical.get(cell_id, 0.0)
            signature_score = (
                self.type_bonus / (type_rank[cell_id] + 1)
                if cell_id in type_rank
                else 0.0
            )
            score = (
                self.alpha * lexical_score
                + (1.0 - self.alpha) * semantic
                + signature_score
            )
            ranked.append(
                {
                    "manifest": manifest,
                    "score": score,
                    "lexical_score": lexical_score,
                    "semantic_score": semantic,
                    "signature_score": signature_score,
                }
            )
        ranked.sort(key=lambda item: (-item["score"], item["manifest"]["id"]))
        return ranked[:limit]
