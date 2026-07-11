"""The example sidecar generator (`cell-eval gen-examples`) over the tiny synthetic
library — every generation path: value form, expect form for status-flag state twins,
the skip reasons, co_match honesty, and byte-identical determinism. Plus the fused
eval runner + its render over the same setup, closing the loop the F2 measurement
runs at scale."""

import json

from cell_eval.examples_gen import generate, write_sidecar
from cell_eval.report import render_retrieval_examples
from cell_eval.retrieval_examples import run_retrieval_examples


def _rows_by_id(rows):
    return {r["id"]: r for r in rows}


def test_generate_covers_value_state_and_skip_paths(tiny_setup):
    library, dataset = tiny_setup
    rows, stats = generate(dataset=dataset, library_dir=library)
    by_id = _rows_by_id(rows)

    # Value twins: behaviour separates pick_lo from pick_hi, so co_match is empty
    # and the examples are positional.
    lo = by_id["lo-1"]
    assert lo["form"] == "in" and lo["co_match"] == []
    for ex in lo["examples"]:
        args, out = ex["in"], ex["out"]
        assert out == min(args), (args, out)
    # Both of pick_lo's cases equip the same way (same expected cell).
    assert by_id["lo-2"]["form"] == "in"

    # Status-flag state twins: both return 1, so the return alone leaves f_sub
    # co-matching — the generator must escalate to the expect form and separate.
    add = by_id["add-1"]
    assert add["form"] == "fields"
    assert add["co_match"] == []
    assert any("expect" in ex for ex in add["examples"]), add
    ex = add["examples"][0]
    assert ex["out"] == 1
    assert ex["expect"]["out"] == (ex["fields"]["a"] + ex["fields"]["b"]) % 65536
    # `out` is an output field: never part of the authored inputs.
    assert "out" not in ex["fields"]

    # Skip accounting: f32 cell, always-halting cell, and a missing expected id.
    assert stats.unequipped["non-scalar-or-arity"] == 1
    assert stats.unequipped["no-clean-runs"] == 1
    assert stats.unequipped["expected-not-in-library"] == 1
    assert stats.equipped == len(rows) == 5
    assert stats.with_expect >= 1


def test_write_sidecar_is_deterministic(tiny_setup, tmp_path):
    library, dataset = tiny_setup
    out = tmp_path / "sidecar.jsonl"
    path, stats = write_sidecar(dataset=dataset, out=out, library_dir=library)
    first = path.read_text()
    assert first.startswith("#"), "stats header comment expected"
    assert f"equipped={stats.equipped}" in first
    # Comment lines are dataset-loader-invisible; rows parse as JSON.
    rows = [json.loads(l) for l in first.splitlines() if l and not l.startswith("#")]
    assert len(rows) == stats.equipped
    # Re-running must be byte-identical (the diff-clean regression contract).
    write_sidecar(dataset=dataset, out=out, library_dir=library)
    assert path.read_text() == first


def test_fused_eval_runner_and_render_over_the_sidecar(tiny_setup, tmp_path):
    library, dataset = tiny_setup
    sidecar = tmp_path / "sidecar.jsonl"
    write_sidecar(dataset=dataset, out=sidecar, library_dir=library)

    rep = run_retrieval_examples(
        dataset=dataset, examples=str(sidecar), library_dir=library
    )
    assert len(rep.cases) == 8
    # 5 of 8 equipped; coverage is per category too.
    assert 0 < rep.coverage() < 1
    assert rep.coverage("paraphrase") == 1.0
    # The fused contract: no equipped case ranks worse than plain search.
    assert rep.regressions() == []
    # Equipped cases hit rank 1: the twins are separated by behaviour.
    assert rep.fused("direct").precision_at_1 == 1.0
    assert rep.deployed().n == 8
    d = rep.as_dict()
    assert d["eval"] == "retrieval-examples"
    assert d["overall"]["coverage"] == round(rep.coverage(), 4)
    assert set(d["by_category"]) == {"direct", "paraphrase"}

    text = render_retrieval_examples(rep)
    assert "coverage:" in text and "OVERALL plain" in text and "fused" in text
    assert "REGRESSIONS" not in text, text
