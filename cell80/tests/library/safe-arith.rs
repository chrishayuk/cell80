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

#[test]
fn harmonic_mean2_matches_hand_computed_floor_2ab_over_a_plus_b() {
    // harmonic_mean2(a, b) = floor(2*a*b/(a+b)), the harmonic-mean third leg of the
    // AM-GM-HM triad alongside avg2 (arithmetic) and geomean2 (geometric). Cases hand-
    // computed: a+b == 0 (defined as 0), equal inputs (HM(a,a) = a exactly), an exact
    // ratio, a floor-rounding case, the domain extreme a == b == 65535 (exact), a large
    // asymmetric pair that exercises the q/r decomposition's overflow-avoidance path,
    // another exact ratio, and a zero factor.
    let cases: &[(&str, &[u16], u16)] = &[
        ("harmonic_mean2", &[0, 0], 0),             // a+b == 0 -> defined as 0
        ("harmonic_mean2", &[4, 4], 4),             // HM(a,a) = a exactly
        ("harmonic_mean2", &[4, 12], 6),            // 2*4*12/16 = 96/16 = 6 exactly
        ("harmonic_mean2", &[1, 2], 1),             // 2*1*2/3 = 4/3 = 1.333.., floors to 1
        ("harmonic_mean2", &[65535, 65535], 65535), // domain max, exact
        ("harmonic_mean2", &[65535, 1], 1),         // large asymmetry: 131070/65536 floors to 1
        ("harmonic_mean2", &[100, 300], 150),       // 2*100*300/400 = 60000/400 = 150 exactly
        ("harmonic_mean2", &[0, 5], 0),             // one factor zero -> product zero -> 0
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
fn rms2_matches_hand_computed_floor_sqrt_of_mean_of_squares() {
    // Checks rms2 (cells/safe-arith/rms2.rs): the quadratic mean floor(sqrt((a*a+b*b)/2)),
    // the fourth classical Pythagorean mean alongside avg2 (arithmetic) and geomean2
    // (geometric). a*a and b*b are widened to u32 and combined via add_checked_u32 so an
    // extreme pair escalates instead of silently wrapping, then the checked sum is
    // floor-divided by 2 and reduced with the same branch-free bitwise integer-sqrt loop
    // geomean2/euclid_dist already run inline.
    let cases: &[(&str, &[u16], u16)] = &[
        ("rms2", &[3, 4], 3),         // sum=9+16=25, half=12, floor(sqrt(12))=3
        ("rms2", &[0, 0], 0),         // sum=0, half=0, isqrt(0)=0
        ("rms2", &[10, 10], 10),      // RMS of two equal values is that value: half=100, isqrt=10
        ("rms2", &[1, 7], 5),         // sum=1+49=50, half=25, isqrt(25)=5 exactly
        ("rms2", &[65535, 0], 46340), // half=2147418112; 46340^2<=half<46341^2 -> 46340
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

    // Escalation path: a=b=65535 -> a*a+b*b = 8_589_672_450 > u32::MAX (4_294_967_295),
    // so add_checked_u32 must halt (0xFF05, needs_wider_math) instead of wrapping.
    let mut r = cell80::Runner::compile(&crate::common::cell_src("rms2")).unwrap();
    let report = r.run(None, &[65535, 65535], DEFAULT_CYCLES).unwrap();
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn mul_div_sat_computes_widened_cross_multiply_divide() {
    // floor(a*b/c) via a u32 intermediate product; saturates at 65535 if the true
    // quotient overflows u16, and returns 0 when c == 0 (no divide-by-zero halt).
    let cases: &[(&[u16], u16)] = &[
        // Plain case: 10*20/4 = 200/4 = 50, exact, no truncation.
        (&[10, 20, 4], 50),
        // Floor truncation: 7*3/2 = 21/2 = 10.5 -> floors to 10.
        (&[7, 3, 2], 10),
        // c == 0 guard: returns 0 regardless of a, b.
        (&[100, 100, 0], 0),
        // Saturation: 65535*65535 = 4_294_836_225, /1 is far past u16::MAX -> caps at 65535.
        (&[65535, 65535, 1], 65535),
        // Wide intermediate product that still lands in-range: 1000*1000 = 1_000_000
        // (overflows u16 as an intermediate) / 100 = 10_000, which fits u16 fine -- proof
        // the u32 widening is load-bearing, not just the saturation cap.
        (&[1000, 1000, 100], 10000),
    ];

    let mut failures = Vec::new();
    for (args, exp) in cases {
        let got = run_cell("mul_div_sat", args);
        if got != *exp {
            failures.push(format!("mul_div_sat({args:?}) = {got}, expected {exp}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
