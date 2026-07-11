//! Host-oracle tests for the money-bps pack (`cell80/cells/money-bps/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::cell_src;
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn money_bps_state_cells_match_defined_behaviour() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }

    // The GSM8K math-campaign money/basis-points pack (Phase 2.3, M1 pack 2/5) — basis
    // points, never float percentages, per the campaign spec.

    let (_, _, cell) = step("bps_of", "BpsOf", &[("value", 1000), ("bps", 500)]);
    assert_eq!(cell.get("result"), Some(50)); // 5% of 1000

    let (_, _, cell) = step(
        "increase_by_bps",
        "IncreaseByBps",
        &[("value", 1000), ("bps", 500)],
    );
    assert_eq!(cell.get("result"), Some(1050));

    let (_, _, cell) = step(
        "decrease_by_bps",
        "DecreaseByBps",
        &[("value", 1000), ("bps", 500)],
    );
    assert_eq!(cell.get("result"), Some(950));

    // The reverse-percent pair recovers the original value exactly.
    let (_, _, cell) = step(
        "original_before_bps_increase",
        "OriginalBeforeIncrease",
        &[("final_value", 1050), ("bps", 500)],
    );
    assert_eq!(cell.get("original"), Some(1000));
    let (_, _, cell) = step(
        "original_before_bps_decrease",
        "OriginalBeforeDecrease",
        &[("final_value", 950), ("bps", 500)],
    );
    assert_eq!(cell.get("original"), Some(1000));
    // bps == 10000 (100% discount) escalates rather than dividing by zero.
    let (_, report, _) = step(
        "original_before_bps_decrease",
        "OriginalBeforeDecrease",
        &[("final_value", 950), ("bps", 10000)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    let (_, _, cell) = step(
        "cents_mul_qty",
        "CentsMulQty",
        &[("unit_cents", 150), ("qty", 3)],
    );
    assert_eq!(cell.get("total"), Some(450));
}

#[test]
fn money_bps_wave2_cells_match_defined_behaviour() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }

    // GSM8K math-campaign money/bps pack, second slice: the missing inverse — recovering
    // the *rate* from a before/after pair, complementing increase_by_bps/decrease_by_bps
    // (rate -> final value) and original_before_bps_increase/decrease (rate + final ->
    // original). Checked docs/cell-index.md first: cents_div_qty/change_due-style
    // candidates were already considered and rejected as duplicates of
    // div_floor_u32/sub_checked_u32 (docs/library-growth.md, M1 pack 2/5) — not repeated.

    let (_, _, cell) = step(
        "bps_increase_between",
        "BpsIncreaseBetween",
        &[("before", 200), ("after", 250)],
    );
    assert_eq!(cell.get("bps"), Some(2500)); // 25% increase
    let (_, report, _) = step(
        "bps_increase_between",
        "BpsIncreaseBetween",
        &[("before", 250), ("after", 200)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06)); // after < before

    let (_, _, cell) = step(
        "bps_decrease_between",
        "BpsDecreaseBetween",
        &[("before", 200), ("after", 150)],
    );
    assert_eq!(cell.get("bps"), Some(2500)); // 25% decrease
    let (_, report, _) = step(
        "bps_decrease_between",
        "BpsDecreaseBetween",
        &[("before", 150), ("after", 200)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06)); // after > before
}

// Checked against bps_increase_between/bps_decrease_between (each requires a fixed
// direction and halts on the other); bps_change_between accepts either direction and
// reports the rate as a sign-magnitude pair instead.
#[test]
fn bps_change_between_matches_defined_behaviour() {
    fn step(fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src("bps_change_between"), "BpsChangeBetween", None)
            .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // Rose: before=200, after=250 -> +25% = 2500 bps, neg=0 (same magnitude
    // bps_increase_between would report for this pair).
    let (_, cell) = step(&[("before", 200), ("after", 250)]);
    assert_eq!(cell.get("bps_mag"), Some(2500));
    assert_eq!(cell.get("bps_neg"), Some(0));

    // Fell: before=200, after=150 -> -25% = 2500 bps, neg=1 (same magnitude
    // bps_decrease_between would report for this pair) -- one cell handles both.
    let (_, cell) = step(&[("before", 200), ("after", 150)]);
    assert_eq!(cell.get("bps_mag"), Some(2500));
    assert_eq!(cell.get("bps_neg"), Some(1));

    // No change: mag=0, and neg is forced to 0 rather than left ambiguous.
    let (_, cell) = step(&[("before", 1000), ("after", 1000)]);
    assert_eq!(cell.get("bps_mag"), Some(0));
    assert_eq!(cell.get("bps_neg"), Some(0));

    // before == 0 is out of domain (a base of zero has no defined rate), regardless
    // of direction.
    let (report, _) = step(&[("before", 0), ("after", 100)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // A large enough diff overflows the *10000 scale step and escalates rather than
    // silently wrapping.
    let (report, _) = step(&[("before", 1), ("after", 500000)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}


#[test]
fn compound_increase_by_bps_matches_defined_behaviour() {
    fn step(fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src("compound_increase_by_bps"), "CompoundIncreaseByBps", None)
            .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // 10% growth compounded for 3 periods: 1000 -> 1100 -> 1210 -> 1331 (like increase_by_bps
    // applied three times in a row, but looped internally rather than called three times).
    let (_, cell) = step(&[("value", 1000), ("bps", 1000), ("periods", 3)]);
    assert_eq!(cell.get("result"), Some(1331));

    // periods = 0 is a no-op: the while-loop body never runs, value passes through unchanged.
    let (_, cell) = step(&[("value", 500), ("bps", 250), ("periods", 0)]);
    assert_eq!(cell.get("result"), Some(500));

    // bps = 10000 (100% growth) doubles every period: 1 -> 2 -> 4 -> 8 -> 16.
    let (_, cell) = step(&[("value", 1), ("bps", 10000), ("periods", 4)]);
    assert_eq!(cell.get("result"), Some(16));

    // Multiply overflow: value * bps itself exceeds u32::MAX on the first iteration.
    let (report, _) = step(&[("value", 4_000_000_000), ("bps", 5000), ("periods", 1)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // Add-only overflow: value=4294967290, bps=1 keeps the multiply (product=4294967290)
    // safely inside u32, but value+delta=4295396786 overflows u32::MAX -- proves the checked
    // add, not just the checked multiply, is load-bearing every iteration.
    let (report, _) = step(&[("value", 4_294_967_290), ("bps", 1), ("periods", 1)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn compound_decrease_by_bps_matches_hand_computed_expectations() {
    use crate::common::cell_src;
    use cell80::{StateCell, DEFAULT_CYCLES};

    // Checked against decrease_by_bps (single application) and compound_increase_by_bps
    // (the opposite-direction sibling): compounds the same bps discount rate over
    // `periods` iterations, escalating if any step's discount would exceed the running value.
    fn step(value: u64, bps: u64, periods: u64) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src("compound_decrease_by_bps"), "CompoundDecreaseByBps", None)
            .unwrap_or_else(|e| panic!("bind compound_decrease_by_bps: {e}"));
        cell.set("value", value).unwrap();
        cell.set("bps", bps).unwrap();
        cell.set("periods", periods).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // value=1000, bps=1000 (10%), periods=3: 1000 -100=900; 900 -90=810; 810 -81=729.
    let (_, cell) = step(1000, 1000, 3);
    assert_eq!(cell.get("result"), Some(729));

    // periods=0 is the identity: result == value unchanged.
    let (_, cell) = step(12345, 250, 0);
    assert_eq!(cell.get("result"), Some(12345));

    // periods=1 matches a single decrease_by_bps application: 2000 - (2000*750/10000=150) = 1850.
    let (_, cell) = step(2000, 750, 1);
    assert_eq!(cell.get("result"), Some(1850));

    // bps > 10000 (would decrease past zero): value=1, bps=20000 ->
    // product=20000, delta=floor(20000/10000)=2, 2 > 1 -> halt 0xFF05.
    let (report, _) = step(1, 20000, 1);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // Multiply overflow inside a step: value=900_000_000, bps=10000 ->
    // product = 9_000_000_000_000, far past u32::MAX -> mul_checked_u32 halts 0xFF05.
    let (report, _) = step(900_000_000, 10000, 1);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // Two periods at 25%: 800 -200=600; 600 -150=450.
    let (_, cell) = step(800, 2500, 2);
    assert_eq!(cell.get("result"), Some(450));
}
