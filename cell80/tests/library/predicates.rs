//! Host-oracle tests for the predicates pack (`cell80/cells/predicates/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::{cell_src, run_cell};
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn wave4_width_precision_predicates_slice() {
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

    // is_lt_u32 / is_gt_u32 / is_le_u32 / is_ge_u32: wide predicates, exercised past the
    // u16 ceiling (money totals in cents routinely exceed 65535).
    assert_eq!(
        step("is_lt_u32", "IsLtWide", &[("a", 100_000), ("b", 100_001)]).0,
        1
    );
    assert_eq!(
        step("is_lt_u32", "IsLtWide", &[("a", 100_001), ("b", 100_000)]).0,
        0
    );
    assert_eq!(
        step("is_lt_u32", "IsLtWide", &[("a", 100_000), ("b", 100_000)]).0,
        0
    );
    assert_eq!(
        step("is_gt_u32", "IsGtWide", &[("a", 100_001), ("b", 100_000)]).0,
        1
    );
    assert_eq!(
        step("is_gt_u32", "IsGtWide", &[("a", 100_000), ("b", 100_000)]).0,
        0
    );
    assert_eq!(
        step("is_le_u32", "IsLeWide", &[("a", 100_000), ("b", 100_000)]).0,
        1
    );
    assert_eq!(
        step("is_le_u32", "IsLeWide", &[("a", 100_001), ("b", 100_000)]).0,
        0
    );
    assert_eq!(
        step("is_ge_u32", "IsGeWide", &[("a", 100_000), ("b", 100_000)]).0,
        1
    );
    assert_eq!(
        step("is_ge_u32", "IsGeWide", &[("a", 100_000), ("b", 100_001)]).0,
        0
    );

    // Wave 4, slice 1: width/precision gap-fill, redirected from the dead PlanFix
    // role/op/slot-validator branch to the two concrete gaps PlanFix's own findings
    // named — a missing wide-comparison family (is_lt/is_gt/is_le/is_ge only existed at
    // u16; answer_eq_u32 was the only wide predicate) and a floor sibling for
    // frac_of_whole (which only has the exact-or-escalate variant; models routinely
    // write "90% of 23"-style reasoning that doesn't divide evenly).
}

#[test]
fn first_wave_predicates_cells_match_defined_behaviour() {
    let cases: &[(&str, &[u16], u16)] = &[
        ("eq", &[5, 5], 1),
        ("eq", &[5, 6], 0),
        ("neq", &[5, 6], 1),
        ("neq", &[5, 5], 0),
        ("is_lt", &[3, 5], 1),
        ("is_lt", &[5, 5], 0),
        ("is_le", &[5, 5], 1),
        ("is_le", &[6, 5], 0),
        ("is_gt", &[6, 5], 1),
        ("is_gt", &[5, 5], 0),
        ("is_ge", &[5, 5], 1),
        ("is_ge", &[4, 5], 0),
        ("is_zero", &[0], 1),
        ("is_zero", &[3], 0),
        ("nonzero", &[3], 1),
        ("nonzero", &[0], 0),
        ("is_even", &[4], 1),
        ("is_even", &[0], 1),
        ("is_even", &[7], 0),
        ("is_odd", &[7], 1),
        ("is_odd", &[4], 0),
    ];

    let mut failures = Vec::new();
    for (id, args, exp) in cases {
        let got = run_cell(id, args);
        if got != *exp {
            failures.push(format!("{id}({args:?}) = {got}, expected {exp}"));
        }
    }
    assert!(
        failures.is_empty(),
        "cell mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn neq_u32_wide_not_equal_predicate() {
    // neq_u32: wide sibling of neq, exercised past the u16 ceiling (money totals in
    // cents routinely exceed 65535) — completes the six-operator wide comparison
    // family alongside is_lt_u32/is_gt_u32/is_le_u32/is_ge_u32.
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> u16 {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // Equal, past the u16 ceiling -> 0.
    assert_eq!(
        step("neq_u32", "NeqWide", &[("a", 100_000), ("b", 100_000)]),
        0
    );
    // Off by one at wide width -> 1.
    assert_eq!(
        step("neq_u32", "NeqWide", &[("a", 100_000), ("b", 100_001)]),
        1
    );
    // Order-independent: reversed operands, still not-equal -> 1.
    assert_eq!(
        step("neq_u32", "NeqWide", &[("a", 100_001), ("b", 100_000)]),
        1
    );
    // Both zero -> equal -> 0.
    assert_eq!(step("neq_u32", "NeqWide", &[("a", 0), ("b", 0)]), 0);
    // One zero, one nonzero -> 1.
    assert_eq!(step("neq_u32", "NeqWide", &[("a", 0), ("b", 1)]), 1);
    // Large equal values that would collide under mod-65536 confusion if this
    // were mistakenly wired to u16 width -> confirms genuine u32 comparison -> 0.
    assert_eq!(
        step(
            "neq_u32",
            "NeqWide",
            &[("a", 4_000_000_000), ("b", 4_000_000_000)]
        ),
        0
    );
}

// is_zero_u32 (IsZeroWide): wide-width zero check, exercised past the u16 ceiling —
// money-cents totals and other wide balances routinely exceed 65535, and is_zero
// (which is u16-only) can't safely check those without truncation risk first.
#[test]
fn is_zero_u32_wide_zero_predicate() {
    fn step(x: u64) -> u16 {
        let mut cell = StateCell::bind(&cell_src("is_zero_u32"), "IsZeroWide", None)
            .unwrap_or_else(|e| panic!("bind is_zero_u32: {e}"));
        cell.set("x", x).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    assert_eq!(step(0), 1, "0 is zero");
    assert_eq!(step(1), 0, "1 is not zero");
    assert_eq!(step(65_536), 0, "65536 (just past u16 ceiling) is not zero");
    assert_eq!(step(4_294_967_295), 0, "u32::MAX is not zero");
    assert_eq!(step(100_000), 0, "a money-cents-scale balance is not zero");
}

#[test]
fn nonzero_u32_wide_predicate() {
    // nonzero_u32 / NonzeroWide: wide sibling of nonzero, exercised past the u16 ceiling
    // (money totals in cents routinely exceed 65535).
    fn step(fields: &[(&str, u64)]) -> u16 {
        let mut cell = StateCell::bind(&cell_src("nonzero_u32"), "NonzeroWide", None)
            .unwrap_or_else(|e| panic!("bind nonzero_u32: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // x = 0 -> 0 (the zero case)
    assert_eq!(step(&[("x", 0)]).clone(), 0);
    // x = 1 -> 1 (smallest nonzero value)
    assert_eq!(step(&[("x", 1)]), 1);
    // x = 65_535 -> 1 (u16 max, still nonzero — sanity check at the old narrow ceiling)
    assert_eq!(step(&[("x", 65_535)]), 1);
    // x = 65_536 -> 1 (exceeds u16 range entirely; only representable because the field is u32)
    assert_eq!(step(&[("x", 65_536)]), 1);
    // x = 4_294_967_295 -> 1 (u32 max, upper bound of the wide field)
    assert_eq!(step(&[("x", 4_294_967_295)]), 1);
}

#[test]
fn is_lt_i16_matches_hand_computed_expectations() {
    // is_lt_i16(a, b): 1 if a < b under true signed ordering, else 0 -- the signed
    // sibling of is_lt/is_lt_u32. Args/results are read as their two's-complement u16
    // bit pattern (-5 <-> 65531), matching signed-deltas' own convention.

    // Both positive: 3 < 5 -> 1.
    assert_eq!(run_cell("is_lt_i16", &[3, 5]), 1);

    // Both positive, false case: 5 < 3 -> 0.
    assert_eq!(run_cell("is_lt_i16", &[5, 3]), 0);

    // Mixed sign, the case a naive u16 bit-pattern compare gets backwards:
    // -1 (bits 65535) < 0 -> true signed ordering says 1, unsigned bit-compare would say 0.
    assert_eq!(run_cell("is_lt_i16", &[65535, 0]), 1);

    // Both negative: -5 (65531) < -3 (65533) -> 1 (true signed ordering).
    assert_eq!(run_cell("is_lt_i16", &[65531, 65533]), 1);

    // Equal values: 7 < 7 -> 0.
    assert_eq!(run_cell("is_lt_i16", &[7, 7]), 0);

    // Extremes: i16::MIN (32768) < i16::MAX (32767) -> 1.
    assert_eq!(run_cell("is_lt_i16", &[32768, 32767]), 1);
}

#[test]
fn is_gt_i16_signed_ordering_matches_hand_computed_expectations() {
    // is_gt_i16: (a > b) as u16 under TRUE signed ordering -- the signed sibling of is_gt (u16)
    // and is_gt_u32, and the direct complement of is_lt_i16. Exercises the case where a naive
    // unsigned bit-comparison would give the wrong answer (mixed-sign inputs), plus the ties
    // and extremes any comparison predicate needs.
    fn i16_bits(v: i16) -> u16 {
        v as u16
    }

    let cases: &[(i16, i16, u16)] = &[
        (5, 3, 1),               // both positive, a > b -> true
        (3, 5, 0),               // both positive, a < b -> false
        (-5, -3, 0),             // both negative: -5 > -3 is false
        (1, -1, 1), // mixed sign: 1 > -1 -> true (unsigned bit-compare would wrongly say 1 > 65535 is false)
        (42, 42, 0), // equal values -> false
        (i16::MAX, i16::MIN, 1), // extremes: 32767 > -32768 -> true
    ];
    for &(a, b, expected) in cases {
        assert_eq!(
            run_cell("is_gt_i16", &[i16_bits(a), i16_bits(b)]),
            expected,
            "is_gt_i16({a}, {b}) should be {expected}"
        );
    }
}

#[test]
fn is_le_i16_matches_hand_computed_expectations() {
    // is_le_i16(a, b): non-strict signed <= sibling of is_lt_i16/is_gt_i16/is_ge_i16, and
    // the signed counterpart of is_le, which bit-reinterprets negative values as large
    // positives and so orders them wrong. Negative arguments are passed/read as their
    // two's-complement u16 bit pattern, matching this file's other signed-i16 cases.

    // a=5, b=3: 5 <= 3 is false -> 0.
    assert_eq!(run_cell("is_le_i16", &[5, 3]), 0);
    // a=3, b=5: 3 <= 5 is true -> 1.
    assert_eq!(run_cell("is_le_i16", &[3, 5]), 1);
    // a=5, b=5: equal, <= is inclusive -> 1.
    assert_eq!(run_cell("is_le_i16", &[5, 5]), 1);
    // a=-5 (65531), b=-3 (65533): -5 <= -3 is true -> 1.
    assert_eq!(run_cell("is_le_i16", &[65531, 65533]), 1);
    // a=-1 (65535), b=0: -1 <= 0 is true -> 1. Unsigned bit-pattern compare would say
    // 65535 <= 0 is false -- this is exactly the case that proves signed ordering is used.
    assert_eq!(run_cell("is_le_i16", &[65535, 0]), 1);
    // a=1, b=-1 (65535): 1 <= -1 is false -> 0.
    assert_eq!(run_cell("is_le_i16", &[1, 65535]), 0);
    // a=i16::MIN (32768 bits, -32768), b=i16::MAX (32767 bits, 32767): -32768 <= 32767 -> 1.
    assert_eq!(run_cell("is_le_i16", &[32768, 32767]), 1);
}

#[test]
fn is_ge_i16_hand_computed_cases() {
    // Compiles and runs the is_ge_i16 free function via the host oracle, checking
    // (a >= b) as u16 under true signed ordering for i16 inputs.
    use cell80::{Runner, DEFAULT_CYCLES};

    fn run(a: i16, b: i16) -> u16 {
        let mut r =
            Runner::compile(&cell_src("is_ge_i16")).unwrap_or_else(|e| panic!("compile: {e}"));
        r.run(None, &[a as u16, b as u16], DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run: {e}"))
            .result
    }

    // 5 >= 3 -> true (positive, strictly greater) -> 1
    assert_eq!(run(5, 3), 1, "5 >= 3");
    // 3 >= 5 -> false (positive, strictly less) -> 0
    assert_eq!(run(3, 5), 0, "3 >= 5");
    // -1 >= -1 -> true (equal negatives) -> 1
    assert_eq!(run(-1, -1), 1, "-1 >= -1");
    // -5 >= -3 -> false: -5 is the smaller (more negative) value under true signed
    // ordering, even though its raw bit pattern (0xFFFB) is numerically larger than
    // -3's (0xFFFD) -- this is exactly the bug is_ge (unsigned) would get wrong.
    assert_eq!(run(-5, -3), 0, "-5 >= -3");
    // i16::MAX >= i16::MIN -> true (largest vs smallest representable values)
    assert_eq!(run(32767, -32768), 1, "i16::MAX >= i16::MIN");
}

// is_even_u32 (IsEvenWide): wide-width parity check, exercised past the u16 ceiling —
// money-cents totals and other wide balances routinely exceed 65535, and is_even
// (which is u16-only) can't safely check those without truncation risk first.
#[test]
fn is_even_u32_wide_parity_predicate() {
    fn step(x: u64) -> u16 {
        let mut cell = StateCell::bind(&cell_src("is_even_u32"), "IsEvenWide", None)
            .unwrap_or_else(|e| panic!("bind is_even_u32: {e}"));
        cell.set("x", x).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // x = 0 -> 1 (zero is even)
    assert_eq!(step(0), 1, "0 is even");
    // x = 1 -> 0 (smallest odd value)
    assert_eq!(step(1), 0, "1 is odd");
    // x = 65_536 -> 1 (just past the u16 ceiling, still even; only representable because the field is u32)
    assert_eq!(step(65_536), 1, "65536 is even");
    // x = 65_537 -> 0 (just past the u16 ceiling, odd)
    assert_eq!(step(65_537), 0, "65537 is odd");
    // x = 4_294_967_294 -> 1 (u32::MAX - 1, even, upper bound of the wide field minus one)
    assert_eq!(step(4_294_967_294), 1, "u32::MAX - 1 is even");
    // x = 4_294_967_295 -> 0 (u32::MAX is odd)
    assert_eq!(step(4_294_967_295), 0, "u32::MAX is odd");
}

// is_odd_u32 (IsOddWide): wide-width odd/parity check, exercised past the u16 ceiling —
// money-cents totals and other wide balances routinely exceed 65535, and is_odd (which
// is u16-only) can't safely check those without truncation risk first. Direct complement
// of is_zero_u32/nonzero_u32's pairing pattern, here completing is_even_u32's pair.
#[test]
fn is_odd_u32_wide_odd_predicate() {
    fn step(x: u64) -> u16 {
        let mut cell = StateCell::bind(&cell_src("is_odd_u32"), "IsOddWide", None)
            .unwrap_or_else(|e| panic!("bind is_odd_u32: {e}"));
        cell.set("x", x).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // 0 is even -> 0
    assert_eq!(step(0), 0, "0 is even");
    // 1 is odd -> 1
    assert_eq!(step(1), 1, "1 is odd");
    // 65_536 is just past u16 ceiling and even (65536 / 2 = 32768 exactly) -> 0
    assert_eq!(step(65_536), 0, "65536 is even");
    // 100_001 is a money-cents-scale odd balance -> 1
    assert_eq!(step(100_001), 1, "100001 is odd");
    // u32::MAX = 4_294_967_295 is odd -> 1
    assert_eq!(step(4_294_967_295), 1, "u32::MAX is odd");
    // u32::MAX - 1 = 4_294_967_294 is even -> 0
    assert_eq!(step(4_294_967_294), 0, "u32::MAX - 1 is even");
}


#[test]
fn is_positive_i16_hand_computed_cases() {
    // is_positive_i16(x): 1 if x > 0 under true signed i16 ordering, else 0 -- tests
    // order against the implicit zero that is_gt_i16/is_ge_i16/is_lt_i16/is_le_i16 (all
    // two-argument) never exercise alone, and is distinct from sign_i16 (returns -1/0/1,
    // not the 0/1 predicate convention). Negative arguments are passed as their
    // two's-complement u16 bit pattern, matching this file's other signed-i16 cases.
    fn i16_bits(v: i16) -> u16 {
        v as u16
    }

    let cases: &[(i16, u16)] = &[
        (5, 1),      // clearly positive -> 1
        (0, 0),      // zero is NOT strictly positive -> 0
        (-1, 0),     // -1 bit pattern is 0xFFFF (65535), a large u16 -- must NOT be
                     // misread as positive -> 0 (proves true signed ordering is used)
        (-32768, 0), // i16::MIN, very negative -> 0
        (32767, 1),  // i16::MAX, very positive -> 1
        (1, 1),      // smallest positive value -> 1
    ];

    for &(x, expected) in cases {
        assert_eq!(
            run_cell("is_positive_i16", &[i16_bits(x)]),
            expected,
            "is_positive_i16({x}) should be {expected}"
        );
    }
}

#[test]
fn is_negative_i16_matches_hand_computed_cases() {
    // is_negative_i16(x): 1 if x < 0 under true signed ordering, else 0 -- the direct
    // complement of is_positive_i16. Args/results are read as their two's-complement
    // u16 bit pattern (-5 <-> 65531), matching signed-deltas' own convention.
    let cases: &[(u16, u16)] = &[
        (0, 0),      // x = 0, not negative -> 0
        (5, 0),      // x = 5, positive -> 0
        (65535, 1),  // x = -1 (bits 65535), negative -> 1
        (65531, 1),  // x = -5 (bits 65531), negative -> 1
        (32768, 1),  // x = i16::MIN (-32768), negative -> 1
        (32767, 0),  // x = i16::MAX (32767), positive -> 0
    ];

    let mut failures = Vec::new();
    for (x, exp) in cases {
        let got = run_cell("is_negative_i16", &[*x]);
        if got != *exp {
            failures.push(format!("is_negative_i16({x}) = {got}, expected {exp}"));
        }
    }
    assert!(failures.is_empty(), "cell mismatches:\n{}", failures.join("\n"));
}

#[test]
fn is_nonneg_i16_matches_hand_computed_expectations() {
    // is_nonneg_i16(x): (x >= 0) as u16 under true signed ordering -- the non-strict
    // complement of is_positive_i16 (which is strict, x > 0), mirroring the
    // is_gt_i16/is_ge_i16 strict/non-strict pairing already in this pack. Negative
    // arguments are passed as their two's-complement u16 bit pattern, matching this
    // file's other signed-i16 cases. Distinct from verifier-ranker's smag_is_nonneg,
    // which tests a (magnitude, sign) pair, not a raw i16.
    fn i16_bits(v: i16) -> u16 {
        v as u16
    }

    let cases: &[(i16, u16)] = &[
        (0, 1),        // zero counts as nonnegative -> 1
        (5, 1),        // ordinary positive -> 1
        (-1, 0),       // smallest-magnitude negative -> 0
        (i16::MAX, 1), // 32767 -> 1
        (i16::MIN, 0), // -32768 -> 0
    ];

    for &(x, expected) in cases {
        assert_eq!(
            run_cell("is_nonneg_i16", &[i16_bits(x)]),
            expected,
            "is_nonneg_i16({x}) should be {expected}"
        );
    }
}

#[test]
fn is_nonpos_i16_hand_computed_cases() {
    // is_nonpos_i16(x): (x <= 0) as u16 under true signed ordering -- the non-strict
    // complement of is_positive_i16, completing the sign-vs-zero family alongside
    // is_gt_i16/is_ge_i16/is_lt_i16/is_le_i16. Negative arguments are passed/read as
    // their two's-complement u16 bit pattern, matching this file's other signed-i16 cases.
    fn i16_bits(v: i16) -> u16 {
        v as u16
    }

    let cases: &[(i16, u16)] = &[
        (0, 1),             // zero counts as non-positive -> 1
        (5, 0),              // positive -> 0
        (-5, 1),              // negative -> 1
        (-1, 1),              // negative -> 1
        (i16::MAX, 0),       // 32767 <= 0 -> 0
        (i16::MIN, 1),       // -32768 <= 0 -> 1
    ];
    for &(x, expected) in cases {
        assert_eq!(
            run_cell("is_nonpos_i16", &[i16_bits(x)]),
            expected,
            "is_nonpos_i16({x}) should be {expected}"
        );
    }
}

#[test]
fn same_sign_i16_hand_computed_cases() {
    // Compiles and runs the same_sign_i16 free function via the host oracle, checking
    // (i16_neg(a) == i16_neg(b)) as u16 -- i.e. whether a and b share a sign bucket,
    // with zero counted nonnegative per the spec.
    use cell80::{Runner, DEFAULT_CYCLES};

    fn run(a: i16, b: i16) -> u16 {
        let mut r = Runner::compile(&cell_src("same_sign_i16"))
            .unwrap_or_else(|e| panic!("compile: {e}"));
        r.run(None, &[a as u16, b as u16], DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run: {e}"))
            .result
    }

    // 5, 3: both nonnegative -> same bucket -> 1
    assert_eq!(run(5, 3), 1, "5,3 both nonneg");
    // -5, -3: both negative -> same bucket -> 1
    assert_eq!(run(-5, -3), 1, "-5,-3 both neg");
    // 5, -3: opposite buckets -> 0
    assert_eq!(run(5, -3), 0, "5,-3 opposite");
    // 0, -1: zero counts nonnegative, -1 is negative -> opposite buckets -> 0
    assert_eq!(run(0, -1), 0, "0,-1 opposite (zero is nonneg)");
    // 0, 0: both nonnegative (zero counted nonnegative) -> same bucket -> 1
    assert_eq!(run(0, 0), 1, "0,0 both nonneg");
    // i16::MIN, i16::MAX: opposite buckets -> 0
    assert_eq!(run(-32768, 32767), 0, "MIN,MAX opposite");
}

#[test]
fn diff_sign_i16_hand_computed_cases() {
    // diff_sign_i16(a, b): 1 if a and b fall in different sign buckets (one >=0, the
    // other <0), else 0 -- the direct complement of same_sign_i16. Passes i16 args as
    // their u16 bit pattern to the host oracle, matching this file's other signed-i16
    // cases (e.g. is_ge_i16_hand_computed_cases).
    use cell80::{Runner, DEFAULT_CYCLES};

    fn run(a: i16, b: i16) -> u16 {
        let mut r = Runner::compile(&cell_src("diff_sign_i16"))
            .unwrap_or_else(|e| panic!("compile: {e}"));
        r.run(None, &[a as u16, b as u16], DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run: {e}"))
            .result
    }

    // 5, 3: both >= 0 -> same bucket -> 0
    assert_eq!(run(5, 3), 0, "5,3 same bucket");
    // -5, 3: a < 0, b >= 0 -> different buckets -> 1
    assert_eq!(run(-5, 3), 1, "-5,3 different buckets");
    // 5, -3: a >= 0, b < 0 -> different buckets -> 1
    assert_eq!(run(5, -3), 1, "5,-3 different buckets");
    // -5, -3: both < 0 -> same bucket -> 0
    assert_eq!(run(-5, -3), 0, "-5,-3 same bucket");
    // 0, -1: 0 counts as >= 0, -1 < 0 -> different buckets -> 1
    assert_eq!(run(0, -1), 1, "0,-1 different buckets (0 is nonneg)");
    // 0, 0: both >= 0 -> same bucket -> 0
    assert_eq!(run(0, 0), 0, "0,0 same bucket");
    // i16::MIN, i16::MAX: -32768 < 0, 32767 >= 0 -> different buckets -> 1
    assert_eq!(run(-32768, 32767), 1, "i16::MIN,i16::MAX different buckets");
}
