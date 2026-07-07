//! Host-oracle tests for the calendrical-checksum pack (`cell80/cells/calendrical-checksum/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::run_cell;

#[test]
fn first_wave_calendrical_checksum_cells_match_defined_behaviour() {
    let cases: &[(&str, &[u16], u16)] = &[
        ("is_leap_year", &[2000], 1), // divisible by 400
        ("is_leap_year", &[1900], 0), // divisible by 100, not 400
        ("is_leap_year", &[2024], 1), // divisible by 4, not 100
        ("is_leap_year", &[2023], 0),
        ("days_in_month", &[2, 1], 29), // Feb, leap
        ("days_in_month", &[2, 0], 28), // Feb, non-leap
        ("days_in_month", &[4, 0], 30),
        ("days_in_month", &[12, 0], 31),
        ("days_in_month", &[13, 0], 0),    // invalid month
        ("day_of_week", &[2000, 1, 1], 0), // Saturday
        ("day_of_week", &[2024, 1, 1], 2), // Monday
        ("luhn_check", &[1230], 1),        // valid Luhn number
        ("luhn_check", &[1231], 0),        // one digit off — invalid
        ("luhn_check", &[0], 1),           // trivial single-zero-digit edge case
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
