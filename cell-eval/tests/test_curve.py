"""The library-scale curve (Phase 2.3). Deterministic parts only — no model required."""

import json

from cell_eval.curve import append_checkpoint, record_checkpoint


def test_record_checkpoint_over_seed_library():
    rec = record_checkpoint(label="test-checkpoint")
    assert rec["label"] == "test-checkpoint"
    assert rec["cell_count"] >= 100  # the real seed library, not a stub
    assert rec["retrieval"]["eval"] == "retrieval"
    assert rec["retrieval"]["overall"]["n"] > 0
    # No model configured in this test environment — gracefully skipped, not faked.
    assert "skipped" in rec["adoption"] or "eval" in rec["adoption"]
    assert "skipped" in rec["composition"] or "eval" in rec["composition"]


def test_record_checkpoint_default_label_uses_cell_count():
    rec = record_checkpoint()
    assert rec["label"] == f"{rec['cell_count']}-cells"


def test_append_checkpoint_creates_and_appends(tmp_path):
    path = tmp_path / "curve.json"
    rec1 = {"label": "a", "cell_count": 10}
    rec2 = {"label": "b", "cell_count": 20}

    out1 = append_checkpoint(rec1, path)
    assert out1 == path
    assert json.loads(path.read_text()) == [rec1]

    append_checkpoint(rec2, path)
    assert json.loads(path.read_text()) == [rec1, rec2]
