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

#[test]
fn day_of_year_matches_hand_computed_cases() {
    // day_of_year(year, month, day) -> ordinal day of year (1-366): cumulative days in all
    // preceding months (per days_in_month's table) plus day, with February's length bumped to
    // 29 whenever is_leap_year(year) would be 1 -- so the leap adjustment only lands for
    // March-onward dates, exactly as day_of_year's spec describes.
    let cases: &[(&str, &[u16], u16)] = &[
        ("day_of_year", &[2024, 1, 1], 1),     // Jan 1 is always day 1
        ("day_of_year", &[2024, 3, 1], 61),    // leap year: 31 (Jan) + 29 (Feb, leap) + 1
        ("day_of_year", &[2023, 3, 1], 60),    // non-leap year: 31 (Jan) + 28 (Feb) + 1
        ("day_of_year", &[2000, 12, 31], 366), // leap year (div by 400), last day of year
        ("day_of_year", &[1900, 12, 31], 365), // century, not div by 400 -> non-leap, last day
        ("day_of_year", &[2024, 2, 29], 60),   // the leap day itself: 31 (Jan) + 29
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
fn is_valid_date_matches_hand_computed_cases() {
    // Checks month bound (1-12), day bound (1..=days_in_month), and that the leap-year
    // rule (div-by-4, except centuries not div-by-400) is actually consulted for February.
    let cases: &[(&str, &[u16], u16)] = &[
        ("is_valid_date", &[2024, 2, 29], 1), // leap year (div by 4, not 100): Feb 29 is real
        ("is_valid_date", &[2023, 2, 29], 0), // non-leap year: Feb only has 28 days
        ("is_valid_date", &[1900, 2, 29], 0), // century not div by 400: non-leap, Feb 28 only
        ("is_valid_date", &[2000, 2, 29], 1), // century div by 400: leap, Feb 29 valid
        ("is_valid_date", &[2024, 4, 31], 0), // April has only 30 days
        ("is_valid_date", &[2024, 13, 1], 0), // month out of 1-12 range
        ("is_valid_date", &[2024, 1, 0], 0),  // day 0 is never valid
        ("is_valid_date", &[2024, 6, 30], 1), // ordinary valid date, 30-day month at its boundary
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
fn luhn_check_digit_generates_digit_that_makes_luhn_check_pass() {
    // luhn_check_digit computes the digit to append to `partial` so the completed
    // number passes luhn_check's mod-10 doubling rule. Cross-check each case against
    // luhn_check itself (partial * 10 + check_digit must validate), plus a couple of
    // independently hand-computed values.
    let cases: &[(u16, u16)] = &[
        (0, 0),    // trivial: no digits to process, check digit 0
        (1, 8),    // "1" -> doubled(1)=2, sum=2, cd=(10-2)%10=8 -> "18"
        (12, 5),   // digits 2,1 -> doubled(2)=4 + 1 = 5, cd=5 -> "125"
        (1792, 1), // digits 2,9,7,1 -> 4+9+5+1=19, cd=(10-9)%10=1 -> "17921"; kept <= 6553 so
        // partial*10 below can't overflow u16 (7992 would have overflowed there)
        (4417, 2), // digits 7,1,4,4 -> 5+1+8+4=18, cd=(10-8)%10=2 -> "44172"
    ];

    for (partial, expected_digit) in cases {
        let got_digit = run_cell("luhn_check_digit", &[*partial]);
        assert_eq!(
            got_digit, *expected_digit,
            "luhn_check_digit({partial}) = {got_digit}, expected {expected_digit}"
        );

        // The completed number (partial's digits followed by the generated check
        // digit) must itself pass luhn_check.
        let completed = partial * 10 + got_digit;
        let passes = run_cell("luhn_check", &[completed]);
        assert_eq!(
            passes, 1,
            "luhn_check({completed}) should pass after appending generated check digit {got_digit}"
        );
    }
}


#[test]
fn is_weekend_matches_hand_computed_cases() {
    // dow codes per day_of_week: 0=Saturday, 1=Sunday, 2=Monday, 3=Tuesday, 4=Wednesday,
    // 5=Thursday, 6=Friday. is_weekend should return 1 only for the two weekend codes
    // (0, 1) and 0 for every weekday code, composing directly with day_of_week's output.
    let cases: &[(u16, u16)] = &[
        (0, 1), // Saturday -> weekend
        (1, 1), // Sunday -> weekend
        (2, 0), // Monday -> not weekend
        (4, 0), // Wednesday -> not weekend
        (6, 0), // Friday -> not weekend
    ];

    let mut failures = Vec::new();
    for (dow, exp) in cases {
        let got = run_cell("is_weekend", &[*dow]);
        if got != *exp {
            failures.push(format!("is_weekend({dow}) = {got}, expected {exp}"));
        }
    }
    assert!(
        failures.is_empty(),
        "cell mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn is_weekday_matches_hand_computed_cases() {
    // day_of_week's convention: 0=Saturday, 1=Sunday, 2=Monday, 3=Tuesday, 4=Wednesday,
    // 5=Thursday, 6=Friday. is_weekday is 1 for codes 2..=6 (Monday-Friday), else 0 -- the
    // direct logical complement of is_weekend.
    let cases: &[(&str, &[u16], u16)] = &[
        ("is_weekday", &[0], 0), // Saturday: weekend
        ("is_weekday", &[1], 0), // Sunday: weekend
        ("is_weekday", &[2], 1), // Monday: lower boundary of weekday range, inclusive
        ("is_weekday", &[4], 1), // Wednesday: mid-range
        ("is_weekday", &[6], 1), // Friday: upper boundary of weekday range, inclusive
        ("is_weekday", &[7], 0), // not a legal day_of_week code; must not spuriously pass
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
