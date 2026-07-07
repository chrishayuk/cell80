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
