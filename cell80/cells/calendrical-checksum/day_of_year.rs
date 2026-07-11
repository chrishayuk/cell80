//! Ordinal day-of-year (1-366) for a Gregorian date: sum of days in all preceding months plus day, with a leap-day adjustment for March onward.
//! tags: calendar, date, day-of-year, ordinal, julian-day, gregorian
//! limits: month must be 1-12 and day must be valid for that month (out-of-range inputs are not rejected, just produce a nonsensical total)
fn run(year: u16, month: u16, day: u16) -> u16 {
    let by4 = year % 4u16 == 0u16;
    let by100 = year % 100u16 == 0u16;
    let by400 = year % 400u16 == 0u16;
    let is_leap = (by4 && (!by100 || by400)) as u16;

    // Cumulative non-leap days before each month, closed form. Month 0
    // contributes nothing (the old summation loop never ran) and months past
    // 12 contribute a full year (their month lengths were 0) — the same
    // out-of-range totals as before, at O(1) instead of O(month) steps.
    let before = match month {
        0u16 => 0u16,
        1u16 => 0u16,
        2u16 => 31u16,
        3u16 => 59u16,
        4u16 => 90u16,
        5u16 => 120u16,
        6u16 => 151u16,
        7u16 => 181u16,
        8u16 => 212u16,
        9u16 => 243u16,
        10u16 => 273u16,
        11u16 => 304u16,
        12u16 => 334u16,
        _ => 365u16,
    };
    let leap_add = (month > 2u16 && is_leap != 0u16) as u16;
    day + before + leap_add
}
