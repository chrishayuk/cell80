"""The margin gate, offline: pure gate math over synthetic decisions (no embedding
model, no library) — answered/escalate accounting, per-split stats, and the
calibration sweep's operating-point rule."""

from cell_eval.tiers import Decision, TierReport, calibrate


def _d(cat, margin, top1_ok):
    top = [(0.9, "right" if top1_ok else "wrong"), (0.9 - margin, "other")]
    return Decision(query="q", expected=["right"], category=cat, top=top, margin=margin)


def _report(theta=0.1):
    r = TierReport(embed_model="synthetic", theta=theta)
    # direct: confident and right
    r.decisions += [_d("direct", 0.3, True), _d("direct", 0.25, True)]
    # paraphrase: one confident-right, one shaky-wrong (must escalate at θ=0.1)
    r.decisions += [_d("paraphrase", 0.2, True), _d("paraphrase", 0.02, False)]
    # adversarial: confident-wrong at low margin, escalates once θ clears it
    r.decisions += [_d("adversarial", 0.05, False), _d("adversarial", 0.01, False)]
    for i in range(len(r.decisions)):
        r.tier1_top[i] = r.decisions[i].top[0][1]
    return r


def test_gate_answers_confident_and_escalates_shaky():
    r = _report(theta=0.1)
    assert r.split("direct").answer_rate == 1.0
    assert r.split("direct").precision_on_answered == 1.0
    p = r.split("paraphrase")
    assert p.answered == 1 and p.answered_correct == 1  # the shaky-wrong escalated
    a = r.split("adversarial")
    assert a.answered == 0  # everything below the margin escalates


def test_calibration_picks_the_smallest_safe_theta():
    r = _report()
    cal = calibrate(r, floor=0.75)
    # Adversarial answers are all wrong below 0.06 margin; the smallest θ that clears
    # the floor is the first one where no adversarial query is answered (vacuous 1.0).
    assert cal["chosen_theta"] is not None
    t = cal["chosen_theta"]
    assert r.split("adversarial", t).precision_on_answered >= 0.75
    # And every smaller θ on the grid fails the floor.
    smaller = [p for p in cal["curve"] if p["theta"] < t]
    assert all(p["adversarial"]["precision_on_answered"] < 0.75 for p in smaller)


def test_report_dict_carries_all_splits_and_tiers():
    r = _report(theta=0.1)
    d = r.as_dict()
    assert set(d["splits"]) == {"direct", "paraphrase", "adversarial"}
    assert d["splits"]["direct"]["tier1_p1"] == 1.0
    assert 0.0 <= d["splits"]["adversarial"]["tier2_p1"] <= 1.0


class _FakeStatic:
    """Deterministic stand-in for a model2vec StaticModel: char-bucket vectors."""

    def encode(self, texts):
        import numpy as np

        out = []
        for t in texts:
            v = np.zeros(16, dtype="float32")
            for b in t.lower().encode():
                v[b % 16] += 1.0
            out.append(v)
        return np.array(out)


def test_embedder_static_and_cell_potion_paths(monkeypatch):
    import model2vec

    from cell_eval.tiers import Embedder

    seen = []
    monkeypatch.setattr(
        model2vec.StaticModel,
        "from_pretrained",
        classmethod(lambda cls, model: seen.append(model) or _FakeStatic()),
    )
    e = Embedder("some/hf-model")
    v = e.encode(["min of two", "max of two"])
    assert v.shape[0] == 2
    # L2-normalised: dot with self is 1.
    assert abs(float(v[0] @ v[0]) - 1.0) < 1e-6
    # encode_cached: repeated docs hit the cache (same vectors back).
    c = e.encode_cached(["min of two", "min of two"])
    assert (c[0] == c[1]).all()
    # "cell-potion" resolves to the local trained-artifact path before loading.
    Embedder("cell-potion")
    assert seen[-1].endswith("potion/model"), seen


def test_embedder_ollama_backend_offline(monkeypatch):
    import io
    import json as _json
    import urllib.request

    from cell_eval.tiers import Embedder

    def fake_urlopen(req, timeout=0):
        body = _json.loads(req.data)
        data = {"data": [{"embedding": [float(len(t)), 1.0, 0.0]} for t in body["input"]]}
        return io.BytesIO(_json.dumps(data).encode())

    monkeypatch.setattr(urllib.request, "urlopen", fake_urlopen)
    e = Embedder("ollama:fake-embed")
    v = e.encode(["ab", "abcd"])
    assert v.shape == (2, 3)
    assert abs(float(v[1] @ v[1]) - 1.0) < 1e-6


def test_run_tiers_offline_over_tiny_library(tiny_setup, monkeypatch):
    from cell_eval import tiers as t

    monkeypatch.setattr(
        t, "Embedder", lambda model=None: type("E", (), {
            "name": "fake",
            "encode": staticmethod(_FakeStatic().encode),
            "encode_cached": staticmethod(_FakeStatic().encode),
        })()
    )
    library, dataset = tiny_setup
    # Append a query nothing matches: the empty-hits branch must record an
    # escalating decision instead of crashing.
    import json as _json
    import pathlib

    rows = pathlib.Path(dataset).read_text()
    ds = pathlib.Path(dataset).parent / "tiny-with-miss.jsonl"
    ds.write_text(
        rows + _json.dumps({"id": "miss-1", "query": "zzz qqq xyzzy", "expected": "pick_lo", "category": "adversarial"}) + "\n"
    )
    rep = t.run_tiers(dataset=str(ds), library_dir=library, embed_model="fake", theta=0.05)
    assert len(rep.decisions) == 9
    empty = [d for d in rep.decisions if not d.top]
    assert len(empty) == 1 and not empty[0].answered(rep.theta)
    # The split accounting runs over real decisions (tier1_correct included).
    for cat in ("direct", "paraphrase", "adversarial"):
        s = rep.split(cat)
        assert s.n > 0
    assert rep.as_dict()["splits"]["direct"]["n"] >= 4
