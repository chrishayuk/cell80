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
