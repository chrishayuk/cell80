//! Host-oracle tests for the safe-arith pack (`cell80/cells/safe-arith/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::{cell_src, run_cell};
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn wide_state_cells_carry_exact_u32_results() {
    // The wide siblings of the result-overflow value cells: the u32 output field holds
    // the exact value the u16 return can't (`square(300)`, `weighted_sum` past 65535).
    let mut sq = StateCell::bind(&cell_src("square_wide"), "Sq", None).unwrap();
    sq.set("n", 300).unwrap();
    sq.run(DEFAULT_CYCLES).unwrap();
    assert_eq!(sq.get("sq"), Some(90_000));
    sq.set("n", 65535).unwrap(); // the domain extreme: 65535² needs all 32 bits
    sq.run(DEFAULT_CYCLES).unwrap();
    assert_eq!(sq.get("sq"), Some(65_535u64 * 65_535));

    let mut ws = StateCell::bind(&cell_src("weighted_sum_wide"), "Ws", None).unwrap();
    for (f, v) in [("a", 30_000u64), ("b", 20_000), ("c", 10_000)] {
        ws.set(f, v).unwrap();
    }
    assert_eq!(ws.run(DEFAULT_CYCLES).unwrap().result, 65535); // saturated scalar
    assert_eq!(ws.get("sum"), Some(100_000)); // a + 2b + 3c, exact
}

#[test]
fn first_wave_safe_arith_cells_match_defined_behaviour() {
    let cases: &[(&str, &[u16], u16)] = &[
        ("add_sat", &[100, 50], 150),
        ("add_sat", &[65535, 10], 65535),
        ("add_sat", &[60000, 6000], 65535),
        ("sub_sat", &[100, 30], 70),
        ("sub_sat", &[30, 100], 0),
        ("mul_sat", &[12, 12], 144),
        ("mul_sat", &[0, 9999], 0),
        ("mul_sat", &[1000, 1000], 65535),
        ("safe_div", &[17, 5], 3),
        ("safe_div", &[9, 0], 0),
        ("safe_mod", &[17, 5], 2),
        ("safe_mod", &[9, 0], 0),
        ("ceil_div", &[17, 5], 4),
        ("ceil_div", &[10, 5], 2),
        ("ceil_div", &[0, 5], 0),
        ("ceil_div", &[9, 0], 0),
        ("ceil_div", &[65535, 2], 32768),
        ("avg2", &[10, 20], 15),
        ("avg2", &[65534, 65534], 65534),
        ("square", &[12], 144),
        ("square", &[255], 65025),
        ("square", &[256], 65535),
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
fn round_div_rounds_to_nearest_quotient_with_ties_up() {
    // round_div(a, b): nearest-integer division, ties rounding UP (matching
    // round_to_multiple's tie convention), 0 when b == 0. Implemented as
    // q = a/b, r = a%b, round up iff r >= b-r -- never a+b/2, so it can't
    // overflow even at the u16 domain extreme.
    let cases: &[(u16, u16, u16)] = &[
        (17, 5, 3),        // 17/5 = 3.4  -> rounds down to 3
        (18, 5, 4),        // 18/5 = 3.6  -> rounds up to 4
        (6, 4, 2),         // 6/4 = 1.5   exact tie -> ties round UP to 2
        (5, 0, 0),         // divide by zero guarded -> 0
        (0, 7, 0),         // 0/7 = 0
        (65535, 2, 32768), // 65535/2 = 32767.5 tie -> rounds up to 32768, no overflow
    ];
    for (a, b, exp) in cases {
        let got = run_cell("round_div", &[*a, *b]);
        assert_eq!(got, *exp, "round_div({a}, {b}) = {got}, expected {exp}");
    }
}

#[test]
fn geomean2_matches_hand_computed_floor_sqrt_of_product() {
    // geomean2(a, b) = floor(sqrt(a*b)), the geometric-mean sibling avg2 (arithmetic
    // mean) has no counterpart for. Cases hand-computed: two perfect squares (36, 60
    // rounds down non-exactly), a zero factor, and the domain extreme where a == b ==
    // 65535 so a*b is the largest u32 product a u16 pair can produce and its root is
    // exactly representable back in u16.
    let cases: &[(&str, &[u16], u16)] = &[
        ("geomean2", &[0, 0], 0),             // sqrt(0) = 0
        ("geomean2", &[4, 9], 6),             // sqrt(36) = 6 exactly
        ("geomean2", &[3, 5], 3),             // sqrt(15) = 3.872..., floors to 3
        ("geomean2", &[6, 10], 7),            // sqrt(60) = 7.746..., floors to 7
        ("geomean2", &[65535, 65535], 65535), // sqrt(65535^2) = 65535 exactly
        ("geomean2", &[100, 0], 0),           // one factor zero -> product zero -> 0
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
