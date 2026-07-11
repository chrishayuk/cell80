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

#[test]
fn luhn_check_u32_matches_hand_computed_cases() {
    // luhn_check_u32 splits a full card number into hi (upper digits) and lo (fixed
    // low 9 digits), so the caller must pre-split the decimal number: e.g. 4111111111111111
    // (16 digits) -> hi=4111111 (top 7 digits), lo=111111111 (bottom 9 digits).
    use cell80::{StateCell, DEFAULT_CYCLES};

    fn step(hi: u64, lo: u64) -> u16 {
        let mut cell = StateCell::bind(
            &crate::common::cell_src("luhn_check_u32"),
            "LuhnCheckU32",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("hi", hi).unwrap();
        cell.set("lo", lo).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.halt, cell80::Halt::Returned, "hi={hi} lo={lo}");
        cell.get("valid").unwrap() as u16
    }

    // 1. Visa test number 4111111111111111 (16 digits, known-valid Luhn number).
    assert_eq!(step(4_111_111, 111_111_111), 1);

    // 2. Same number with the last digit flipped 1 -> 2: must fail.
    assert_eq!(step(4_111_111, 111_111_112), 0);

    // 3. Trivial all-zero edge case: sum of digits is 0, 0 % 10 == 0 -> valid.
    assert_eq!(step(0, 0), 1);

    // 4. Visa 13-digit test number 4222222222222 (known-valid Luhn number).
    //    hi = 4222 (top 4 digits), lo = 222222222 (bottom 9 digits).
    assert_eq!(step(4222, 222_222_222), 1);

    // 5. Same number with the last digit flipped 2 -> 1: must fail.
    assert_eq!(step(4222, 222_222_221), 0);
}

#[test]
fn luhn_check_digit_u32_matches_hand_computed_cases() {
    // luhn_check_digit_u32 is the generate-side counterpart to luhn_check_u32, sharing
    // its exact hi/lo split (lo = low 9 decimal digits, hi = everything above). Cases
    // are hand-computed by walking the full (unsplit) decimal number's digits from the
    // right, doubling every digit at an even position (0-indexed from the right) and
    // reducing any result over 9 by subtracting 9 -- the same rule luhn_check_digit uses
    // at u16 width, just continued across the hi/lo boundary.
    let cases: &[(u32, u32, u16)] = &[
        (0, 0, 0),         // trivial: no digits, check digit 0
        (0, 1792, 1),      // hi=0; matches narrow luhn_check_digit(1792) = 1
        (7, 992739871, 3), // classic Visa test number 7992739871 split across
        // the hi/lo boundary (hi holds just the leading "7"); known-good check digit is 3
        (1, 5, 8), // hi=1, lo=5 -- lo has 8 leading (high-order) zero
        // digits before its single "5"; full number is 1000000005 (digits 5,0,0,0,0,0,0,0,0,1
        // from the right), doubling at even position sums to 2, check digit (10-2)%10=8 --
        // exercises that hi's fixed parity (double at odd local position) doesn't depend on
        // how many of lo's digits were actually nonzero
        (999_999_999, 999_999_999, 8), // max-width edge: 18 nines; doubling a 9 always
                                       // yields 9 (2*9=18, 18-9=9) so parity is irrelevant here, sum = 18*9 = 162, check
                                       // digit (10 - 162 % 10) % 10 = 8
    ];

    for (hi, lo, expected) in cases {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("luhn_check_digit_u32"),
            "LuhnCheckDigitU32",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("hi", *hi as u64).unwrap();
        cell.set("lo", *lo as u64).unwrap();
        cell.run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run: {e}"));
        let got = cell.get("digit").unwrap();
        assert_eq!(
            got, *expected as u64,
            "luhn_check_digit_u32(hi={hi}, lo={lo}) = {got}, expected {expected}"
        );
    }
}

#[test]
fn isbn10_check_matches_hand_computed_cases() {
    // Local helpers: bind the state cell, feed body (9-digit prefix packed as u32) and
    // check (the 10th character's value, 0-9 or 10 for 'X'), run, and read back `valid`.
    fn isbn10_check(body: u32, check: u16) -> u16 {
        let mut c = cell80::StateCell::bind(
            &crate::common::cell_src("isbn10_check"),
            "Isbn10Check",
            None,
        )
        .unwrap();
        c.set("body", body as u64).unwrap();
        c.set("check", check as u64).unwrap();
        c.run(cell80::DEFAULT_CYCLES).unwrap();
        c.get("valid").unwrap() as u16
    }

    let cases: &[(u32, u16, u16, &str)] = &[
        // "0306406152" -- a well-known valid ISBN-10 (Wikipedia's canonical example).
        // Weighted sum 0*10+3*9+0*8+6*7+4*6+0*5+6*4+1*3+5*2+2*1 = 132 = 11*12.
        (30640615, 2, 1, "0306406152 is valid"),
        // Same body, wrong check digit: 130+3=133, 133 % 11 = 1 (not 0) -> invalid.
        (30640615, 3, 0, "0306406153 is invalid (bad check digit)"),
        // "0136091814" -- another valid ISBN-10.
        // 0*10+1*9+3*8+6*7+0*6+9*5+1*4+8*3+1*2+4*1 = 154 = 11*14.
        (13609181, 4, 1, "0136091814 is valid"),
        // "097522980X" -- valid ISBN-10 whose check digit is X (packed as 10).
        // 0*10+9*9+7*8+5*7+2*6+2*5+9*4+8*3+0*2+10*1 = 264 = 11*24.
        (97522980, 10, 1, "097522980X is valid"),
        // check out of the 0..=10 domain (e.g. a mis-encoded 'X') is invalid regardless
        // of what the weighted sum would otherwise be.
        (30640615, 11, 0, "check > 10 is always invalid"),
    ];

    let mut failures = Vec::new();
    for (body, check, expected, label) in cases {
        let got = isbn10_check(*body, *check);
        if got != *expected {
            failures.push(format!(
                "{label}: isbn10_check(body={body}, check={check}) = {got}, expected {expected}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "cell mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn isbn10_check_digit_matches_hand_computed_cases() {
    // Verifies isbn10_check_digit (the generate-side counterpart to isbn10_check) against
    // 5 independently hand-computed expectations: standard weighted-mod-11 cases, the
    // all-zero edge case, and a case landing on the 'X' (10) check character.
    fn digit_for(body: u64) -> u64 {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("isbn10_check_digit"),
            "Isbn10CheckDigit",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("body", body).unwrap();
        cell.run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run: {e}"));
        cell.get("digit").unwrap()
    }

    // body=013110362 ("The C Programming Language"): sum = 0*10+1*9+3*8+1*7+1*6+0*5+3*4+6*3+2*2
    // = 0+9+24+7+6+0+12+18+4 = 80; 80 mod 11 = 3; check = (11-3) mod 11 = 8.
    assert_eq!(digit_for(13110362), 8);

    // body=026203384 ("Introduction to Algorithms"): sum = 0*10+2*9+6*8+2*7+0*6+3*5+3*4+8*3+4*2
    // = 0+18+48+14+0+15+12+24+8 = 139; 139 mod 11 = 7; check = (11-7) mod 11 = 4.
    assert_eq!(digit_for(26203384), 4);

    // body=000000006: only the units digit (weight 2) is nonzero: sum = 6*2 = 12;
    // 12 mod 11 = 1; check = (11-1) mod 11 = 10 -- the 'X' check character.
    assert_eq!(digit_for(6), 10);

    // body=000000000: sum = 0; check = (11-0) mod 11 = 0 -- all-zero edge case.
    assert_eq!(digit_for(0), 0);

    // body=020153082: sum = 0*10+2*9+0*8+1*7+5*6+3*5+0*4+8*3+2*2
    // = 0+18+0+7+30+15+0+24+4 = 98; 98 mod 11 = 10; check = (11-10) mod 11 = 1.
    assert_eq!(digit_for(20153082), 1);
}

#[test]
fn ean13_check_matches_hand_computed_cases() {
    // ean13_check validates a full 13-digit EAN-13/UPC-A/ISBN-13 barcode: the standard
    // mod-10 checksum with weights 1,3 alternating across all 13 digits (equivalently
    // 1,3,1,... left to right since 13 is odd). The 13 digits are split hi/lo across two
    // u32 state fields (7 digits in hi, 6 in lo) since no single u32 field can hold all
    // 13 decimal digits.
    fn step(hi: u64, lo: u64) -> u16 {
        let src = crate::common::cell_src("ean13_check");
        let mut cell = cell80::StateCell::bind(&src, "Ean13Check", None)
            .unwrap_or_else(|e| panic!("bind ean13_check: {e}"));
        cell.set("hi", hi).unwrap();
        cell.set("lo", lo).unwrap();
        cell.run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run ean13_check: {e}"))
            .result
    }

    let cases: &[(&str, u64, u64, u16)] = &[
        // "5901234123457" -- canonical known-valid EAN-13 example (7+6 split).
        // Hand sum: weights 1,3,1,3,1,3,1,3,1,3,1,3,1 over 5,9,0,1,2,3,4,1,2,3,4,5,7 = 90 -> %10==0.
        ("5901234123457 (valid EAN-13)", 5_901_234, 123_457, 1),
        // Same number with the check digit corrupted (7 -> 8): total becomes 91, not a
        // multiple of 10.
        (
            "5901234123458 (corrupted check digit)",
            5_901_234,
            123_458,
            0,
        ),
        // "0036000291452" -- valid UPC-A (036000291452) written as EAN-13 with a leading 0.
        // Hand sum: 0+0+3+18+0+0+0+6+9+3+4+15+2 = 60 -> %10==0.
        ("0036000291452 (valid UPC-A as EAN-13)", 36_000, 291_452, 1),
        // "9780306406157" -- canonical known-valid ISBN-13 example.
        // Hand sum: 9+21+8+0+3+0+6+12+0+18+1+15+7 = 100 -> %10==0.
        ("9780306406157 (valid ISBN-13)", 9_780_306, 406_157, 1),
        // Same ISBN-13 with the check digit corrupted (7 -> 8): total becomes 101, not a
        // multiple of 10.
        (
            "9780306406158 (corrupted check digit)",
            9_780_306,
            406_158,
            0,
        ),
        // Trivial all-zero edge case: sum is 0, a multiple of 10 -- valid.
        ("0000000000000 (trivial all-zero)", 0, 0, 1),
    ];

    let mut failures = Vec::new();
    for (label, hi, lo, expected) in cases {
        let got = step(*hi, *lo);
        if got != *expected {
            failures.push(format!(
                "{label}: hi={hi} lo={lo} => {got}, expected {expected}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "cell mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn ean13_check_digit_matches_hand_computed_cases() {
    // ean13_check_digit is a state cell: hi/lo are the first 12 digits split into two
    // 6-digit decimal halves (hi the more significant half), digit is the computed
    // 13th EAN-13 check digit. Each expected value was hand-derived from the EAN-13
    // rule (odd left-to-right positions weight 1, even positions weight 3, then
    // (10 - sum mod 10) mod 10), including two of Wikipedia's own canonical worked
    // examples, before running anything.
    fn ean13_digit(hi: u32, lo: u32) -> u16 {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("ean13_check_digit"),
            "Ean13CheckDigit",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("hi", hi as u64).unwrap();
        cell.set("lo", lo as u64).unwrap();
        cell.run(cell80::DEFAULT_CYCLES).unwrap().result
    }

    let cases: &[(u32, u32, u16)] = &[
        // Wikipedia's canonical Nutella-jar EAN-13: 4006381333931 -> first 12 digits
        // 400638133393 check to 1.
        (400638, 133393, 1),
        // All-zero 12-digit number: every weighted digit is 0 -> (10-0)%10 = 0.
        (0, 0, 0),
        // 123456789012: odd-position sum (1-indexed from the left) 1+3+5+7+9+1=26,
        // even-position sum (2+4+6+8+0+2)*3=66, total 92, check=(10-92%10)%10=8.
        (123456, 789012, 8),
        // Wikipedia's other canonical EAN-13 worked example: 5901234123457 -> first
        // 12 digits 590123412345 check to 7.
        (590123, 412345, 7),
        // Leading zeros in hi (only the last digit, at position 12, is nonzero):
        // position 12 is even (weight 3), so check = (10 - 5*3%10) % 10 = 5.
        (0, 5, 5),
    ];

    let mut failures = Vec::new();
    for (hi, lo, expected) in cases {
        let got = ean13_digit(*hi, *lo);
        if got != *expected {
            failures.push(format!(
                "ean13_check_digit(hi={hi}, lo={lo}) = {got}, expected {expected}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "cell mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn days_between_matches_hand_computed_cases() {
    // Self-contained: uses fully-qualified paths (cell80::StateCell/DEFAULT_CYCLES,
    // crate::common::cell_src) rather than relying on this file's existing
    // `use crate::common::run_cell;` import, since days_between is a state cell.
    fn step(fields: &[(&str, u64)]) -> (cell80::Report, cell80::StateCell) {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("days_between"),
            "DaysBetween",
            None,
        )
        .unwrap_or_else(|e| panic!("bind days_between: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(cell80::DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // Adjacent days -> 1.
    let (report, cell) = step(&[
        ("y1", 2024),
        ("m1", 1),
        ("d1", 1),
        ("y2", 2024),
        ("m2", 1),
        ("d2", 2),
    ]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("days"), Some(1));

    // Full leap year span: 2024 is a leap year (div 4, not div 100), so Jan1 2024 -> Jan1 2025
    // covers all 366 days of 2024.
    let (report, cell) = step(&[
        ("y1", 2024),
        ("m1", 1),
        ("d1", 1),
        ("y2", 2025),
        ("m2", 1),
        ("d2", 1),
    ]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("days"), Some(366));

    // Same date -> 0.
    let (report, cell) = step(&[
        ("y1", 2023),
        ("m1", 6),
        ("d1", 15),
        ("y2", 2023),
        ("m2", 6),
        ("d2", 15),
    ]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("days"), Some(0));

    // Order independence: Jan1 2023 -> Jun15 2023 is 31+28+31+30+31+14 = 165 days in either
    // direction (the cell always returns the absolute difference).
    let (report, cell) = step(&[
        ("y1", 2023),
        ("m1", 1),
        ("d1", 1),
        ("y2", 2023),
        ("m2", 6),
        ("d2", 15),
    ]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("days"), Some(165));
    let (report, cell) = step(&[
        ("y1", 2023),
        ("m1", 6),
        ("d1", 15),
        ("y2", 2023),
        ("m2", 1),
        ("d2", 1),
    ]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("days"), Some(165));

    // Leap-day crossing: 2020 is a leap year, so Feb28 -> Mar1 spans Feb29 too -> 2 days.
    let (report, cell) = step(&[
        ("y1", 2020),
        ("m1", 2),
        ("d1", 28),
        ("y2", 2020),
        ("m2", 3),
        ("d2", 1),
    ]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("days"), Some(2));

    // Escalation: a multi-century span overflows u16::MAX days (~179 years) -> halt 0xFF05
    // (needs_wider_math), per the cell's documented limits.
    let (report, _) = step(&[
        ("y1", 0),
        ("m1", 1),
        ("d1", 1),
        ("y2", 65535),
        ("m2", 12),
        ("d2", 31),
    ]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}
