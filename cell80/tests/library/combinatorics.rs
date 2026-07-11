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

#[test]
fn is_fibonacci_number_membership() {
    // Fibonacci sequence: F(0)=0, F(1)=1, F(2)=1, F(3)=2, F(4)=3, F(5)=5, F(6)=8,
    // F(7)=13, F(8)=21, F(9)=34, F(10)=55, ..., F(24)=46368, F(25)=75025 (>u16::MAX).
    // is_fibonacci_number is the inverse-membership test sibling of fibonacci_checked_u32,
    // mirroring the catalan_number / is_catalan_number pair already in this pack.

    // 0 is F(0): a Fibonacci number.
    assert_eq!(run_cell("is_fibonacci_number", &[0]), 1);
    // 1 is F(1) = F(2): a Fibonacci number (the one duplicate in the sequence).
    assert_eq!(run_cell("is_fibonacci_number", &[1]), 1);
    // 4 sits strictly between F(4)=3 and F(5)=5: not a Fibonacci number.
    assert_eq!(run_cell("is_fibonacci_number", &[4]), 0);
    // 55 is F(10): a Fibonacci number.
    assert_eq!(run_cell("is_fibonacci_number", &[55]), 1);
    // 46368 is F(24), the largest Fibonacci number that fits in u16 (F(25)=75025 overflows).
    assert_eq!(run_cell("is_fibonacci_number", &[46368]), 1);
    // 46369, one more than F(24), is not a Fibonacci number.
    assert_eq!(run_cell("is_fibonacci_number", &[46369]), 0);
}

#[test]
fn fubini_number_matches_oeis_a000670_and_escalates_on_overflow() {
    // fubini_number: a(0)=1, a(n) = sum_{k=1}^{n} C(n,k)*a(n-k) -- OEIS A000670,
    // the ordered-partition ("ordered Bell") counterpart to bell_number's unordered
    // count. Values cross-checked against the published A000670 sequence.
    fn step(n: u64) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src("fubini_number"), "FubiniNumber", None)
            .unwrap_or_else(|e| panic!("bind fubini_number: {e}"));
        cell.set("n", n).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    let (_, cell) = step(0);
    assert_eq!(cell.get("result"), Some(1)); // a(0) = 1 by convention
    let (_, cell) = step(1);
    assert_eq!(cell.get("result"), Some(1));
    let (_, cell) = step(3);
    assert_eq!(cell.get("result"), Some(13)); // 3*3 + 3*1 + 1*1
    let (_, cell) = step(5);
    assert_eq!(cell.get("result"), Some(541));
    let (_, cell) = step(11);
    assert_eq!(cell.get("result"), Some(1_622_632_573)); // last value that fits u32

    // a(12) = 28_091_567_595 overflows u32::MAX -- must escalate, never silently wrap.
    let (report, _) = step(12);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // n = 20 is out of the array bound (n must be < 20).
    let (report, _) = step(20);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}

#[test]
fn verify_rencontres_number() {
    // rencontres_number: D(n,k) = C(n,k) * D(n-k), the count of permutations of n
    // elements with exactly k fixed points. Derangement numbers used below:
    // D(0)=1, D(1)=0, D(2)=1, D(3)=2, D(4)=9, D(5)=44.
    fn step(fields: &[(&str, u64)]) -> u64 {
        let mut cell = StateCell::bind(&cell_src("rencontres_number"), "RencontresNumber", None)
            .unwrap_or_else(|e| panic!("bind rencontres_number: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run rencontres_number: {e}"));
        cell.get("result").unwrap_or_else(|| panic!("no result"))
    }

    // k=0 reproduces derangement_count(4) = 9 exactly, the generalization's base case.
    assert_eq!(step(&[("n", 4), ("k", 0)]), 9);
    // D(4,1) = C(4,1)*D(3) = 4*2 = 8.
    assert_eq!(step(&[("n", 4), ("k", 1)]), 8);
    // D(4,2) = C(4,2)*D(2) = 6*1 = 6.
    assert_eq!(step(&[("n", 4), ("k", 2)]), 6);
    // D(4,4) = C(4,4)*D(0) = 1*1 = 1: every element fixed, the identity permutation.
    assert_eq!(step(&[("n", 4), ("k", 4)]), 1); // C(4,4)*D(0) = 1*1
                                                // D(5,2) = C(5,2)*D(3) = 10*2 = 20.
    assert_eq!(step(&[("n", 5), ("k", 2)]), 20);
    // k > n is out of domain: returns 0, not an escalation.
    assert_eq!(step(&[("n", 3), ("k", 5)]), 0);
    // D(0,0) = C(0,0)*D(0) = 1*1 = 1: the empty permutation, vacuously 0 fixed points.
    assert_eq!(step(&[("n", 0), ("k", 0)]), 1);
}

#[test]
fn double_factorial_checked_recurrence_skips_every_other_term() {
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

    // double_factorial: 0!! = 1 by convention (empty product).
    let (_, _, cell) = step("double_factorial", "DoubleFactorial", &[("n", 0)]);
    assert_eq!(cell.get("result"), Some(1));

    // 7!! = 7*5*3*1 = 105 (odd chain, distinct from 7! = 5040).
    let (_, _, cell) = step("double_factorial", "DoubleFactorial", &[("n", 7)]);
    assert_eq!(cell.get("result"), Some(105));

    // 10!! = 10*8*6*4*2 = 3840 (even chain).
    let (_, _, cell) = step("double_factorial", "DoubleFactorial", &[("n", 10)]);
    assert_eq!(cell.get("result"), Some(3840));

    // 20!! = 3,715,891,200 -- last even n that still fits u32::MAX (4,294,967,295).
    let (_, _, cell) = step("double_factorial", "DoubleFactorial", &[("n", 20)]);
    assert_eq!(cell.get("result"), Some(3_715_891_200));

    // 21!! = 13,749,310,575 and 22!! = 81,749,606,400 both overflow u32 -- escalate rather
    // than silently wrap, the same shape factorial_checked_u32 uses at n >= 13.
    let (_, report, _) = step("double_factorial", "DoubleFactorial", &[("n", 21)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
    let (_, report, _) = step("double_factorial", "DoubleFactorial", &[("n", 22)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn partition_number_matches_oeis_a000041_and_escalates_on_overflow() {
    // partition_number: p(n), the integer partition function (order doesn't count,
    // parts may repeat) -- OEIS A000041. Distinct from bell_number/fubini_number/
    // stirling_second, which all count SET partitions, not integer partitions.
    // Values cross-checked against the published A000041 sequence (and, for the
    // overflow boundary, independently against the pentagonal number theorem
    // recurrence, not against this cell's own DP).
    fn step(n: u64, cycles: u64) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src("partition_number"), "PartitionNumber", None)
            .unwrap_or_else(|e| panic!("bind partition_number: {e}"));
        cell.set("n", n).unwrap();
        let report = cell.run(cycles).unwrap();
        (report, cell)
    }

    let (_, cell) = step(0, DEFAULT_CYCLES);
    assert_eq!(cell.get("result"), Some(1)); // p(0) = 1, the empty sum, by convention
    let (_, cell) = step(4, DEFAULT_CYCLES);
    assert_eq!(cell.get("result"), Some(5)); // 4, 3+1, 2+2, 2+1+1, 1+1+1+1
    let (_, cell) = step(5, DEFAULT_CYCLES);
    assert_eq!(cell.get("result"), Some(7));
    let (_, cell) = step(10, DEFAULT_CYCLES);
    assert_eq!(cell.get("result"), Some(42));
    let (_, cell) = step(20, DEFAULT_CYCLES);
    assert_eq!(cell.get("result"), Some(627));

    // p(127) = 3_913_864_295, the last value that fits u32::MAX. The O(n^2) DP needs
    // more than DEFAULT_CYCLES at this n (same shape as square_pyramidal_number's
    // documented cycle-budget note) -- budget explicitly, per the cell's own doc note.
    let (report, cell) = step(127, 50_000_000);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(3_913_864_295));

    // p(128) = 4_351_078_600 overflows u32::MAX -- must escalate, never silently wrap.
    let (report, _) = step(128, 50_000_000);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // n = 150 is out of the array bound (n must be < 150).
    let (report, _) = step(150, 50_000_000);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}

#[test]
fn verify_distinct_partitions_matches_oeis_a000009_and_escalates() {
    // distinct_partitions: Q(n), the number of partitions of n into distinct
    // (no repeated) parts -- a 0/1 knapsack pass over the same subset-sum DP array
    // shape partition_number uses (inner loop i runs n down to k, not k up to n, so
    // each part size is folded in at most once). Values cross-checked against OEIS
    // A000009 via an independent Python subset-sum DP.
    fn step(n: u64, cycles: u64) -> (cell80::Report, StateCell) {
        let mut cell =
            StateCell::bind(&cell_src("distinct_partitions"), "DistinctPartitions", None)
                .unwrap_or_else(|e| panic!("bind distinct_partitions: {e}"));
        cell.set("n", n).unwrap();
        let report = cell.run(cycles).unwrap();
        (report, cell)
    }

    let (_, cell) = step(0, DEFAULT_CYCLES);
    assert_eq!(cell.get("result"), Some(1)); // Q(0) = 1, the empty sum by convention

    let (_, cell) = step(4, DEFAULT_CYCLES);
    assert_eq!(cell.get("result"), Some(2)); // {4}, {3,1}

    let (_, cell) = step(7, DEFAULT_CYCLES);
    assert_eq!(cell.get("result"), Some(5)); // {7},{6,1},{5,2},{4,3},{4,2,1}

    let (_, cell) = step(20, DEFAULT_CYCLES);
    assert_eq!(cell.get("result"), Some(64)); // OEIS A000009

    // Reaching the overflow point needs more iterations than DEFAULT_CYCLES affords
    // for this pack's checked-arithmetic-call-per-iteration cost (the same reason
    // square_pyramidal_number's test budgets a larger cycle count explicitly). n=255
    // (just under the array bound) hits its own dp[n] overflow early, at k=54 per an
    // independent Python subset-sum DP -- well inside a 20,000,000-cycle budget.
    let (report, _) = step(255, 20_000_000);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // n = 256 is out of the array bound (n must be < 256); this halt fires before
    // any loop work runs, so it is cheap even under DEFAULT_CYCLES.
    let (report, _) = step(256, DEFAULT_CYCLES);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}

#[test]
fn verify_trinomial_coefficient() {
    // trinomial_coefficient: n!/(k1!*k2!*k3!) with k3=n-k1-k2, the number of ways to
    // split n labeled items into 3 labeled groups, computed as choose(n,k1)*choose(n-k1,k2).
    fn step(fields: &[(&str, u64)]) -> u64 {
        let mut cell = StateCell::bind(&cell_src("trinomial_coefficient"), "TrinomialCoeff", None)
            .unwrap_or_else(|e| panic!("bind trinomial_coefficient: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run trinomial_coefficient: {e}"));
        cell.get("result").unwrap_or_else(|| panic!("no result"))
    }

    // n=5,k1=2,k2=2,k3=1: 5!/(2!2!1!) = 120/4 = 30. choose(5,2)=10, choose(3,2)=3, 10*3=30.
    assert_eq!(step(&[("n", 5), ("k1", 2), ("k2", 2)]), 30);
    // n=6,k1=0,k2=0,k3=6: 6!/(0!0!6!) = 1, the all-in-one-group case.
    assert_eq!(step(&[("n", 6), ("k1", 0), ("k2", 0)]), 1);
    // n=4,k1=1,k2=1,k3=2: 4!/(1!1!2!) = 24/2 = 12. choose(4,1)=4, choose(3,1)=3, 4*3=12.
    assert_eq!(step(&[("n", 4), ("k1", 1), ("k2", 1)]), 12);
    // n=10,k1=3,k2=3,k3=4: 10!/(3!3!4!) = 3628800/864 = 4200. choose(10,3)=120, choose(7,3)=35.
    assert_eq!(step(&[("n", 10), ("k1", 3), ("k2", 3)]), 4200);
    // k1+k2 > n is out of domain: returns 0, not an escalation.
    assert_eq!(step(&[("n", 3), ("k1", 2), ("k2", 2)]), 0);
    // n=0,k1=0,k2=0,k3=0: the empty split, vacuously 1 way.
    assert_eq!(step(&[("n", 0), ("k1", 0), ("k2", 0)]), 1);
}

#[test]
fn choose_with_repetition_hand_computed() {
    // Hand-computed multiset-coefficient cases: C(n+k-1, k), the count of ways to choose
    // k items from n types when repeats are allowed (stars-and-bars).
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // n=3, k=2: C(3+2-1,2) = C(4,2) = 6. {aa,bb,cc,ab,ac,bc}.
    let (_, cell) = step(
        "choose_with_repetition",
        "ChooseWithRepetition",
        &[("n", 3), ("k", 2)],
    );
    assert_eq!(cell.get("result"), Some(6));

    // n=1, k=5: C(5,5) = 1. Only one type -> one way (aaaaa).
    let (_, cell) = step(
        "choose_with_repetition",
        "ChooseWithRepetition",
        &[("n", 1), ("k", 5)],
    );
    assert_eq!(cell.get("result"), Some(1));

    // n=5, k=1: C(5,1) = 5. Choosing 1 item from 5 types -> 5 ways.
    let (_, cell) = step(
        "choose_with_repetition",
        "ChooseWithRepetition",
        &[("n", 5), ("k", 1)],
    );
    assert_eq!(cell.get("result"), Some(5));

    // n=0, k=0: no types, choosing nothing -> 1 way (the empty selection), special-cased
    // since the naive n+k-1 formula would underflow at n=0.
    let (_, cell) = step(
        "choose_with_repetition",
        "ChooseWithRepetition",
        &[("n", 0), ("k", 0)],
    );
    assert_eq!(cell.get("result"), Some(1));

    // n=0, k=3: no types but need 3 items -> 0 ways.
    let (_, cell) = step(
        "choose_with_repetition",
        "ChooseWithRepetition",
        &[("n", 0), ("k", 3)],
    );
    assert_eq!(cell.get("result"), Some(0));

    // n=2, k=3: C(2+3-1,3) = C(4,3) = 4. {aaa,aab,abb,bbb}.
    let (_, cell) = step(
        "choose_with_repetition",
        "ChooseWithRepetition",
        &[("n", 2), ("k", 3)],
    );
    assert_eq!(cell.get("result"), Some(4));
}
