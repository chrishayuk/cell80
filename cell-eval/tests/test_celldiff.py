"""Tests for the behavioural snapshot/diff harness (celldiff).

Small real libraries in a tmp dir — the harness compiles and runs actual cells, so the
tests prove the property that matters: an equivalent rewrite compares identical, a
semantic change (the clamp-precedence class of bug) is flagged with the offending
inputs, and signature/membership drift is named.
"""

from __future__ import annotations

import pathlib

from cell_eval.celldiff import battery, compare, load_snapshot, save_snapshot, snapshot

MAX_MUT = """//! Maximum of two values.
//! tags: math, max
fn run(a: u16, b: u16) -> u16 { let mut m = a; if b > a { m = b; } m }
"""
MAX_EXPR = """//! Maximum of two values.
//! tags: math, max
fn run(a: u16, b: u16) -> u16 { if b > a { b } else { a } }
"""
MAX_BROKEN = """//! Maximum of two values.
//! tags: math, max
fn run(a: u16, b: u16) -> u16 { if b >= a { b } else { a } }
"""
CLAMP = """//! Clamp a value to the inclusive range [lo, hi].
//! tags: math, clamp
fn run(x: u16, lo: u16, hi: u16) -> u16 { if x > hi { hi } else if x < lo { lo } else { x } }
"""
# The bug class the harness exists for: identical on well-formed ranges, diverges
# only when lo > hi (which bound wins) — invisible without edge inputs.
CLAMP_LO_WINS = """//! Clamp a value to the inclusive range [lo, hi].
//! tags: math, clamp
fn run(x: u16, lo: u16, hi: u16) -> u16 { if x < lo { lo } else if x > hi { hi } else { x } }
"""
COUNTER_STATE = """//! Add step to a running total (typed state).
//! tags: state, counter
//! entry: Acc::run
struct Acc { total: u16, step: u16 }
impl Acc {
    fn run(&mut self) -> u16 { self.total = self.total + self.step; self.total }
}
"""


def _lib(tmp_path: pathlib.Path, name: str, cells: dict[str, str]) -> str:
    d = tmp_path / name
    d.mkdir()
    for fname, src in cells.items():
        (d / fname).write_text(src)
    return str(d)


def test_battery_shapes():
    assert battery(0) == [[]]
    assert all(len(a) == 1 for a in battery(1))
    assert all(len(a) == 2 for a in battery(2))
    assert all(len(a) == 4 for a in battery(4))
    assert [0, 0] in battery(2)  # degenerate corners present
    assert [65535] in battery(1)


def test_equivalent_rewrite_is_identical(tmp_path):
    before = snapshot(_lib(tmp_path, "a", {"max.rs": MAX_MUT, "clamp.rs": CLAMP}))
    after = snapshot(_lib(tmp_path, "b", {"max.rs": MAX_EXPR, "clamp.rs": CLAMP}))
    rep = compare(before, after)
    assert rep.identical, rep.render()
    assert rep.cells == 2
    assert "OK" in rep.render()


def test_semantic_change_is_flagged_with_inputs(tmp_path):
    # `>` vs `>=` only differs on ties — exactly the subtlety a reviewer misses.
    before = snapshot(_lib(tmp_path, "a", {"max.rs": MAX_MUT}))
    after = snapshot(_lib(tmp_path, "b", {"max.rs": MAX_BROKEN}))
    rep = compare(before, after)
    # equal inputs give the same *value* for max, so > vs >= is actually identical
    # behaviour here — the honest assertion is that the harness agrees.
    assert rep.identical

    # A real semantic change: clamp bound precedence on inverted ranges.
    before = snapshot(_lib(tmp_path, "c", {"clamp.rs": CLAMP}))
    after = snapshot(_lib(tmp_path, "d", {"clamp.rs": CLAMP_LO_WINS}))
    rep = compare(before, after)
    assert not rep.identical
    assert rep.divergent and rep.divergent[0]["cell"] == "clamp"
    assert rep.divergent[0]["inputs"]  # the offending battery rows are named
    assert "clamp" in rep.render()


def test_membership_and_signature_drift(tmp_path):
    before = snapshot(_lib(tmp_path, "a", {"max.rs": MAX_MUT}))
    after = snapshot(
        _lib(
            tmp_path,
            "b",
            {"max.rs": MAX_MUT.replace("(a: u16, b: u16)", "(a: u16, b: u16, pad: u16)")},
        )
    )
    rep = compare(before, after)
    assert not rep.identical
    assert rep.signature_changed == ["max"]

    gone = compare(before, {})
    assert gone.missing == ["max"]
    appeared = compare({}, before)
    assert appeared.added == ["max"]


def test_state_cells_snapshot_by_field_and_round_trip(tmp_path):
    lib = _lib(tmp_path, "a", {"acc.rs": COUNTER_STATE})
    snap = snapshot(lib)
    rows = snap["acc"]["outputs"]
    assert rows  # driven by field name
    # every recorded row carries the post-run state read-back
    assert all(len(v) == 3 and isinstance(v[2], dict) for v in rows.values())

    p = tmp_path / "snap.json"
    save_snapshot(snap, p)
    assert compare(load_snapshot(p), snap).identical
