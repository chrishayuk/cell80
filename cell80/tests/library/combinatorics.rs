//! Host-oracle tests for the combinatorics pack (`cell80/cells/combinatorics/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::{cell_src, run_cell};
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

#[test]
fn recursive_sequences_wave8_cells_match_defined_behaviour() {
    // Wave 8 (docs/math-server-map.md's recursive_sequences category): lucas_u_v folds
    // pell_number and pell_lucas_number in as its p=2,q=1 case (both are U/V of the same
    // recurrence structure), and tribonacci_number is its own genuinely-distinct 3-term
    // recurrence. Every expected value below was cross-checked against an independent
    // Python reference implementation before being transcribed here.

    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // lucas_u_v(p=2, q=1): U is the Pell numbers 0,1,2,5,12,29,...; V is the companion
    // Pell (Pell-Lucas) numbers 2,2,6,14,34,...
    let (_, cell) = step("lucas_u_v", "LucasUV", &[("p", 2), ("q", 1), ("n", 0)]);
    assert_eq!((cell.get("u"), cell.get("v")), (Some(0), Some(2)));
    let (_, cell) = step("lucas_u_v", "LucasUV", &[("p", 2), ("q", 1), ("n", 1)]);
    assert_eq!((cell.get("u"), cell.get("v")), (Some(1), Some(2)));
    let (_, cell) = step("lucas_u_v", "LucasUV", &[("p", 2), ("q", 1), ("n", 5)]);
    assert_eq!((cell.get("u"), cell.get("v")), (Some(29), Some(82)));
    let (_, cell) = step("lucas_u_v", "LucasUV", &[("p", 2), ("q", 1), ("n", 9)]);
    assert_eq!((cell.get("u"), cell.get("v")), (Some(985), Some(2786)));
    // lucas_u_v(p=1, q=1): U is Fibonacci 0,1,1,2,3,5,...; V is the classic Lucas numbers
    // 2,1,3,4,7,11,...
    let (_, cell) = step("lucas_u_v", "LucasUV", &[("p", 1), ("q", 1), ("n", 6)]);
    assert_eq!((cell.get("u"), cell.get("v")), (Some(8), Some(18)));
    // The overflow boundary: U(2,1,26) and V(2,1,26) both exceed u32::MAX; n=25 is the
    // last value that fits.
    let (_, cell) = step("lucas_u_v", "LucasUV", &[("p", 2), ("q", 1), ("n", 25)]);
    assert_eq!(
        (cell.get("u"), cell.get("v")),
        (Some(1_311_738_121), Some(3_710_155_682))
    );
    let (report, _) = step("lucas_u_v", "LucasUV", &[("p", 2), ("q", 1), ("n", 26)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // tribonacci_number: 0,1,1,2,4,7,13,24,44,81,149,...; T(38) is the last value that
    // fits u32, T(39) overflows.
    let (_, cell) = step("tribonacci_number", "TribonacciChecked", &[("n", 0)]);
    assert_eq!(cell.get("result"), Some(0));
    let (_, cell) = step("tribonacci_number", "TribonacciChecked", &[("n", 1)]);
    assert_eq!(cell.get("result"), Some(1));
    let (_, cell) = step("tribonacci_number", "TribonacciChecked", &[("n", 2)]);
    assert_eq!(cell.get("result"), Some(1));
    let (_, cell) = step("tribonacci_number", "TribonacciChecked", &[("n", 9)]);
    assert_eq!(cell.get("result"), Some(81));
    let (_, cell) = step("tribonacci_number", "TribonacciChecked", &[("n", 38)]);
    assert_eq!(cell.get("result"), Some(3_831_006_429));
    let (report, _) = step("tribonacci_number", "TribonacciChecked", &[("n", 39)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn combinatorial_numbers_wave10_cells_match_defined_behaviour() {
    // Wave 10 (docs/math-server-map.md's combinatorial_numbers category). Every
    // expected value below was cross-checked against sympy (stirling_first/second)
    // or the known OEIS sequence (bell_number, is_catalan_number) via an
    // independent Python simulation of each cell's own checked-arithmetic
    // algorithm before being transcribed here.

    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // is_catalan_number: 1, 1, 2, 5, 14, 42, 132, 429, 1430, 4862, 16796, 58786, ...
    assert_eq!(run_cell("is_catalan_number", &[1]), 1);
    assert_eq!(run_cell("is_catalan_number", &[42]), 1);
    assert_eq!(run_cell("is_catalan_number", &[58786]), 1);
    assert_eq!(run_cell("is_catalan_number", &[13]), 0);
    assert_eq!(run_cell("is_catalan_number", &[1000]), 0);
    assert_eq!(run_cell("is_catalan_number", &[4863]), 0);

    // bell_number: 1, 1, 2, 5, 15, 52, 203, 877, 4140, ...; n=14 is the last that
    // fits, n=15 escalates on an intermediate Bell-triangle entry (even though
    // B_15 itself would still fit u32).
    let (_, cell) = step("bell_number", "BellNumber", &[("n", 0)]);
    assert_eq!(cell.get("result"), Some(1));
    let (_, cell) = step("bell_number", "BellNumber", &[("n", 4)]);
    assert_eq!(cell.get("result"), Some(15));
    let (_, cell) = step("bell_number", "BellNumber", &[("n", 9)]);
    assert_eq!(cell.get("result"), Some(21_147));
    let (_, cell) = step("bell_number", "BellNumber", &[("n", 14)]);
    assert_eq!(cell.get("result"), Some(190_899_322));
    let (report, _) = step("bell_number", "BellNumber", &[("n", 15)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // stirling_second: S(n,k), the number of ways to partition n elements into k
    // non-empty subsets.
    let (_, cell) = step("stirling_second", "StirlingSecond", &[("n", 5), ("k", 2)]);
    assert_eq!(cell.get("result"), Some(15));
    let (_, cell) = step("stirling_second", "StirlingSecond", &[("n", 10), ("k", 5)]);
    assert_eq!(cell.get("result"), Some(42_525));
    let (_, cell) = step("stirling_second", "StirlingSecond", &[("n", 0), ("k", 0)]);
    assert_eq!(cell.get("result"), Some(1));
    let (_, cell) = step("stirling_second", "StirlingSecond", &[("n", 3), ("k", 5)]);
    assert_eq!(cell.get("result"), Some(0)); // k > n
    let (report, _) = step("stirling_second", "StirlingSecond", &[("n", 10), ("k", 9)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // stirling_first: unsigned c(n,k), the number of permutations of n elements
    // with exactly k cycles.
    let (_, cell) = step("stirling_first", "StirlingFirst", &[("n", 5), ("k", 2)]);
    assert_eq!(cell.get("result"), Some(50));
    let (_, cell) = step("stirling_first", "StirlingFirst", &[("n", 10), ("k", 5)]);
    assert_eq!(cell.get("result"), Some(269_325));
    let (_, cell) = step("stirling_first", "StirlingFirst", &[("n", 0), ("k", 0)]);
    assert_eq!(cell.get("result"), Some(1));
    let (_, cell) = step("stirling_first", "StirlingFirst", &[("n", 4), ("k", 0)]);
    assert_eq!(cell.get("result"), Some(0));
    let (_, cell) = step("stirling_first", "StirlingFirst", &[("n", 3), ("k", 5)]);
    assert_eq!(cell.get("result"), Some(0)); // k > n
    let (report, _) = step("stirling_first", "StirlingFirst", &[("n", 14), ("k", 1)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // Domain guards.
    let (report, _) = step("bell_number", "BellNumber", &[("n", 20)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
    let (report, _) = step("stirling_first", "StirlingFirst", &[("n", 30), ("k", 24)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}
