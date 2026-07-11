"""The fused example-equipped retrieval path (WS-F/F2) — sidecar integrity, the
fallback contract, coverage, and the F2 gate itself.

The plain-search CI canary (`test_retrieval.py`, direct P@1 ≥ 0.8) is untouched
by all of this: `CellHost::search` is byte-identical and fusion is a separate
entry point. These tests own the fused path only.
"""

import json
import pathlib

from cell_eval.datasets import DATASETS_DIR, load_jsonl
from cell_eval.library import open_library
from cell_eval.retrieval_examples import load_sidecar, run_retrieval_examples

_REPORT = None


def _report():
    global _REPORT
    if _REPORT is None:
        _REPORT = run_retrieval_examples()
    return _REPORT


def test_sidecar_rows_key_real_cases_and_are_well_formed():
    cases = {str(c.get("id", c["query"])) for c in load_jsonl("retrieval")}
    sidecar = load_sidecar()
    assert sidecar, "sidecar is empty — run `cell-eval gen-examples`"
    unknown = set(sidecar) - cases
    assert not unknown, f"sidecar rows keyed to no retrieval.jsonl case: {sorted(unknown)[:5]}"
    for row in sidecar.values():
        assert row["form"] in ("in", "fields")
        assert 1 <= len(row["examples"]) <= 3
        for ex in row["examples"]:
            if row["form"] == "in":
                assert isinstance(ex["in"], list) and isinstance(ex["out"], int)
            else:
                assert isinstance(ex["fields"], dict)
                assert "out" in ex or "expect" in ex


def test_fused_with_no_examples_is_plain_search():
    lib = open_library()
    for q in ("greatest common divisor", "clamp a value to a range", "manhattan distance"):
        assert lib.search(q, 5, examples=[]) == lib.search(q, 5)
        assert lib.search(q, 5, examples=None) == lib.search(q, 5)


def test_equipped_coverage_is_not_cherry_picked():
    # The gate numbers are only meaningful if nearly every row is equipped — a gate
    # reached by skipping hard rows would be cherry-picking with extra steps. 0.95 is
    # the registered growth-gate floor (baselines/README.md, round-4 re-registration);
    # measured 0.985 at 653 cells. Example-equipped admission (each admitted cell
    # ships probe examples) is what keeps this true as the library grows.
    rep = _report()
    assert rep.coverage("paraphrase") >= 0.95, rep.coverage("paraphrase")
    assert rep.coverage() >= 0.95, rep.coverage()


def test_fusion_never_ranks_expected_worse_than_plain():
    # The fused contract: expected reproduces its own examples by construction and
    # ties keep text order, so fused rank <= plain rank on every equipped case.
    rep = _report()
    regs = rep.regressions()
    assert not regs, [(c.case_id, c.plain_rank, c.fused_rank) for c in regs[:5]]


def test_probe_equipped_gates_govern_library_growth():
    # The registered growth gate (round-4 re-registration, 2026-07-11 — see
    # cell-eval/baselines/README.md): fused retrieval over oracle-equipped cases
    # is the BLOCKING gate for library growth; text-only retrieval is a watched
    # fallback (its fixed-cohort ratchet lives in the ledger, its direct floor in
    # test_retrieval.py). Floors are the registered thresholds, not the
    # measurements — measured at 653 cells (checkpoint 21/22): paraphrase 0.859+,
    # adversarial 0.89+, deployed overall 0.90+.
    # The residue is the co_match ambiguity class (behaviourally indistinguishable
    # candidates); if a floor trips, diagnose with
    # `cell-eval retrieval --examples retrieval-examples`, re-price with analysis
    # committed to baselines/ — never regenerate examples to pass.
    rep = _report()
    para = rep.fused("paraphrase").precision_at_1
    adv = rep.fused("adversarial").precision_at_1
    overall = rep.deployed().precision_at_1
    assert para >= 0.82, f"equipped paraphrase P@1 {para:.4f} < 0.82 (growth gate)"
    assert adv >= 0.82, f"equipped adversarial P@1 {adv:.4f} < 0.82 (growth gate)"
    assert overall >= 0.88, f"deployed overall P@1 {overall:.4f} < 0.88 (growth gate)"
