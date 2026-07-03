"""The repair eval, offline: (a) every dataset row is genuinely rejected by the
compiler today (a row that compiles has retired); (b) the one-shot loop end-to-end
against a scripted 'model' — a known-good fix must count as repaired, a bad fix must
not; (c) semantic verification catches a compiling-but-wrong repair."""

import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from _agentfake import FakeClient, _Msg  # noqa: E402

from cell_eval.datasets import load_jsonl  # noqa: E402
from cell_eval.repair import (  # noqa: E402
    extract_source,
    run_repair,
    try_compile,
)
from cell_eval.agent import AgentConfig  # noqa: E402

DATASET = "repair"


def test_every_row_is_rejected_with_prose_not_debug_dumps():
    rows = load_jsonl(DATASET)
    assert len(rows) >= 20, "the repair dataset should cover ~10 classes twice"
    classes = {r["klass"] for r in rows}
    assert len(classes) >= 10, f"want >=10 diagnostic classes, got {sorted(classes)}"
    for r in rows:
        err = try_compile(r["src"])
        assert err is not None, f"{r['id']}: row compiles — retire or update it"
        # The Phase-1.2 DoD holds on every diagnostic the dataset exercises.
        for marker in ("attrs:", "span:", "Expr {", "Lit {"):
            assert marker not in err, f"{r['id']}: syn Debug dump in diagnostic: {err}"


def test_known_good_fix_counts_and_bad_fix_does_not(tmp_path):
    # Two rows: the try-operator row repaired correctly, the float row "repaired" into
    # something that compiles but computes the wrong thing.
    rows = [
        {
            "id": "ok.try",
            "klass": "try_operator",
            "src": "fn run(a: u16, b: u16) -> u16 { a.checked_div(b)? }",
            "intent": "a/b, 0 when b is 0",
            "examples": [[[17, 5], 3], [[9, 0], 0]],
        },
        {
            "id": "bad.float",
            "klass": "float_literal",
            "src": "fn run(x: u16) -> u16 { x * 0.5 }",
            "intent": "half of x",
            "examples": [[[10], 5]],
        },
    ]
    ds = tmp_path / "repair_two.jsonl"
    ds.write_text("\n".join(json.dumps(r) for r in rows))

    good_fix = (
        "```rust\nfn run(a: u16, b: u16) -> u16 { let mut r = 0u16; "
        "if b != 0u16 { r = a / b; } r }\n```"
    )
    wrong_fix = "```rust\nfn run(x: u16) -> u16 { x * 2u16 }\n```"  # compiles, wrong
    client = FakeClient([_Msg(content=good_fix), _Msg(content=wrong_fix)])
    report = run_repair(
        str(ds), client=client, cfg=AgentConfig(model="fake", base_url="offline")
    )

    by_id = {r.id: r for r in report.results}
    assert by_id["ok.try"].compiled and by_id["ok.try"].correct
    assert by_id["bad.float"].compiled and not by_id["bad.float"].correct
    assert report.overall.n == 2 and report.overall.correct == 1
    assert report.by_class["try_operator"].repair_at_1 == 1.0
    assert report.by_class["float_literal"].repair_at_1 == 0.0


def test_extract_source_forms():
    assert extract_source("```rust\nfn run() -> u16 { 1u16 }\n```") is not None
    assert extract_source("fn run() -> u16 { 1u16 }") is not None  # bare source
    assert extract_source("I cannot fix this.") is None
    # The last block wins when the model narrates with multiple blocks.
    two = "first\n```rust\nfn a() -> u16 { 1u16 }\n```\nthen\n```rust\nfn b() -> u16 { 2u16 }\n```"
    assert "fn b" in extract_source(two)
