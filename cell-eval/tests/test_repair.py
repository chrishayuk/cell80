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
    # range_pattern graduated from a repair class to a supported dialect feature (rows
    # removed from the dataset), so this floor tracks the current 9-class/18-row reality,
    # not a fixed "~10 classes twice" — a class retiring is a win, not a regression.
    assert len(rows) >= 18
    classes = {r["klass"] for r in rows}
    assert len(classes) >= 9, f"want >=9 diagnostic classes, got {sorted(classes)}"
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


def test_run_repair_covers_every_per_row_arm(tmp_path):
    """The four non-happy arms in one scripted pass: a retired row (compiles
    unrepaired, no model call), an endpoint error, a reply with no code, a repair
    the compiler still rejects — plus a good fix so the loop ends on success."""
    broken = "fn run(a: u16, b: u16) -> u16 { let x = if a > b { a }; x }"
    rows = [
        # Already compiles → retires before any client call.
        {
            "id": "retired",
            "klass": "none",
            "src": "fn run(a: u16, b: u16) -> u16 { a + b }",
            "intent": "sum",
            "examples": [[[3, 4], 7]],
        },
        {"id": "endpoint", "klass": "if_no_else", "src": broken, "intent": "max", "examples": [[[3, 4], 4]]},
        {"id": "nocode", "klass": "if_no_else", "src": broken, "intent": "max", "examples": [[[3, 4], 4]]},
        {"id": "stillbad", "klass": "if_no_else", "src": broken, "intent": "max", "examples": [[[3, 4], 4]]},
        {"id": "good", "klass": "if_no_else", "src": broken, "intent": "max", "examples": [[[3, 4], 4]]},
    ]
    ds = tmp_path / "repair-arms.jsonl"
    ds.write_text("\n".join(json.dumps(r) for r in rows) + "\n")

    script = [
        RuntimeError("connection refused"),  # endpoint
        _Msg(content="I would fix it like this, in prose only."),  # nocode
        _Msg(content="```rust\nfn run(a: u16, b: u16) -> u16 { still broken }\n```"),
        _Msg(content="```rust\nfn run(a: u16, b: u16) -> u16 { if a > b { a } else { b } }\n```"),
    ]
    rep = run_repair(dataset=str(ds), model="fake", client=FakeClient(script))
    notes = {r.id: r.note for r in rep.results}
    assert "skipped" in notes["retired"]
    assert "endpoint error" in notes["endpoint"]
    assert "no code block" in notes["nocode"]
    assert "still rejected" in notes["stillbad"]
    good = next(r for r in rep.results if r.id == "good")
    assert good.compiled and good.correct
    d = rep.as_dict()
    assert d["overall"]["n"] == 5 and 0 < d["overall"]["repair_at_1"] < 1
