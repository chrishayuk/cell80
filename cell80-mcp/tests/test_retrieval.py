from cell80_mcp.retrieval import HybridRetriever
from cell80_mcp.library import CellLibrary


class FakeHost:
    def __init__(self, manifests):
        self.manifests = manifests

    def search_scored(self, query, limit):
        return [(0.40, self.manifests[0]), (0.35, self.manifests[1])]

    def search(self, query, limit):
        return self.manifests


class FakeEmbedder:
    name = "fake-semantic"

    def __init__(self):
        self.calls = []

    def encode(self, texts):
        self.calls.append(list(texts))
        vectors = []
        for text in texts:
            vectors.append([0.0, 1.0] if "semantic" in text else [1.0, 0.0])
        return vectors


def test_hybrid_reranks_with_semantics_and_caches_manifest_vectors():
    manifests = [
        {"id": "lexical", "summary": "literal candidate", "tags": []},
        {"id": "semantic", "summary": "semantic candidate", "tags": []},
    ]
    embedder = FakeEmbedder()
    retriever = HybridRetriever(
        FakeHost(manifests), lambda: manifests, embedder=embedder
    )

    first = retriever.rank("semantic request", 2)
    second = retriever.rank("semantic request", 2)

    assert first[0]["manifest"]["id"] == "semantic"
    assert first[0]["semantic_score"] > first[1]["semantic_score"]
    assert second[0]["manifest"]["id"] == "semantic"
    assert [len(call) for call in embedder.calls] == [2, 1, 1]


class BrokenEmbedder:
    name = "missing-model"

    def encode(self, texts):
        raise RuntimeError("model is not installed")


def test_hybrid_failure_is_cached_for_visible_fallback():
    manifests = [{"id": "one", "summary": "one", "tags": []}]
    retriever = HybridRetriever(
        FakeHost(manifests), lambda: manifests, embedder=BrokenEmbedder()
    )

    assert retriever.rank("one", 1) is None
    assert retriever.rank("one", 1) is None
    assert retriever.unavailable_reason == "RuntimeError: model is not installed"


def test_search_decision_reports_lexical_fallback():
    manifest = {"id": "add", "summary": "Add two integers", "tags": ["sum"]}

    class FallbackHost(FakeHost):
        def search_confident(self, query, limit, min_margin):
            return {
                "results": [manifest],
                "selected": manifest,
                "confidence": {
                    "method": "tfidf_margin",
                    "status": "confident",
                    "abstained": False,
                    "threshold": min_margin,
                    "margin": 0.5,
                    "top_score": 0.5,
                    "runner_up_score": None,
                },
            }

    host = FallbackHost([manifest])
    library = CellLibrary.__new__(CellLibrary)
    library.host = host
    library._retriever = HybridRetriever(
        host, lambda: [manifest], embedder=BrokenEmbedder()
    )

    decision = library.search_decision("add two integers", 2)

    assert decision["confidence"]["method"] == "lexical_fallback"
    assert decision["confidence"]["fallback_reason"].startswith("RuntimeError:")
    assert decision["results"][0]["id"] == "add"
