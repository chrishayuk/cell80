//! Host-oracle tests for the combinatorics pack (`cell80/cells/combinatorics/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::cell_src;
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn math_aime_pack_combinatorics_slice() {
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

    // factorial_checked_u32: 0! = 1! = 1 by convention; escalates at 13! (overflows u32).
    let (_, _, cell) = step("factorial_checked_u32", "FactorialChecked", &[("n", 0)]);
    assert_eq!(cell.get("result"), Some(1));
    let (_, _, cell) = step("factorial_checked_u32", "FactorialChecked", &[("n", 5)]);
    assert_eq!(cell.get("result"), Some(120));
    let (_, _, cell) = step("factorial_checked_u32", "FactorialChecked", &[("n", 12)]);
    assert_eq!(cell.get("result"), Some(479_001_600));
    let (_, report, _) = step("factorial_checked_u32", "FactorialChecked", &[("n", 13)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // choose_u32 (nCr): k > n returns 0 (not an escalation); escalates once the true
    // binomial coefficient itself overflows u32.
    let (_, _, cell) = step("choose_u32", "ChooseWide", &[("n", 5), ("k", 2)]);
    assert_eq!(cell.get("result"), Some(10));
    let (_, _, cell) = step("choose_u32", "ChooseWide", &[("n", 10), ("k", 0)]);
    assert_eq!(cell.get("result"), Some(1));
    let (_, _, cell) = step("choose_u32", "ChooseWide", &[("n", 3), ("k", 5)]);
    assert_eq!(cell.get("result"), Some(0));
    let (_, _, cell) = step("choose_u32", "ChooseWide", &[("n", 30), ("k", 15)]);
    assert_eq!(cell.get("result"), Some(155_117_520));
    // C(67,33) genuinely doesn't fit u32 (~1.4e19); the pre-division intermediate overflows
    // before the division that would (in exact math) bring it back down.
    let (_, report, _) = step("choose_u32", "ChooseWide", &[("n", 67), ("k", 33)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // permute_u32 (nPr): k > n returns 0; escalates once n!/(n-k)! overflows u32.
    let (_, _, cell) = step("permute_u32", "PermuteWide", &[("n", 10), ("k", 3)]);
    assert_eq!(cell.get("result"), Some(720));
    let (_, _, cell) = step("permute_u32", "PermuteWide", &[("n", 3), ("k", 5)]);
    assert_eq!(cell.get("result"), Some(0));
    let (_, _, cell) = step("permute_u32", "PermuteWide", &[("n", 13), ("k", 10)]);
    assert_eq!(cell.get("result"), Some(1_037_836_800));
    let (_, report, _) = step("permute_u32", "PermuteWide", &[("n", 20), ("k", 10)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn geometry_combinatorics_sequences_combinatorics_slice() {
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

    // fibonacci_checked_u32: standard indexing, F(0)=0, F(1)=1, ...; escalates at n=47.
    let (_, _, cell) = step("fibonacci_checked_u32", "FibonacciChecked", &[("n", 0)]);
    assert_eq!(cell.get("result"), Some(0));
    let (_, _, cell) = step("fibonacci_checked_u32", "FibonacciChecked", &[("n", 10)]);
    assert_eq!(cell.get("result"), Some(55));
    let (_, _, cell) = step("fibonacci_checked_u32", "FibonacciChecked", &[("n", 46)]);
    assert_eq!(cell.get("result"), Some(1_836_311_903));
    let (_, report, _) = step("fibonacci_checked_u32", "FibonacciChecked", &[("n", 47)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // catalan_number: 1, 1, 2, 5, 14, 42, 132, ...; verified safe through n=17.
    let (_, _, cell) = step("catalan_number", "CatalanNumber", &[("n", 0)]);
    assert_eq!(cell.get("result"), Some(1));
    let (_, _, cell) = step("catalan_number", "CatalanNumber", &[("n", 6)]);
    assert_eq!(cell.get("result"), Some(132));
    let (_, _, cell) = step("catalan_number", "CatalanNumber", &[("n", 17)]);
    assert_eq!(cell.get("result"), Some(129_644_790));

    // derangement_count: 1, 0, 1, 2, 9, 44, 265, 1854, 14833, ...
    let (_, _, cell) = step("derangement_count", "DerangementCount", &[("n", 0)]);
    assert_eq!(cell.get("result"), Some(1));
    let (_, _, cell) = step("derangement_count", "DerangementCount", &[("n", 1)]);
    assert_eq!(cell.get("result"), Some(0));
    let (_, _, cell) = step("derangement_count", "DerangementCount", &[("n", 8)]);
    assert_eq!(cell.get("result"), Some(14_833));
}
