//! Ordinal day-of-year (1-366) for a Gregorian date: sum of days in all preceding months plus day, with a leap-day adjustment for March onward.
//! tags: calendar, date, day-of-year, ordinal, julian-day, gregorian
//! limits: month must be 1-12 and day must be valid for that month (out-of-range inputs are not rejected, just produce a nonsensical total)
fn month_len(month: u16, is_leap: u16) -> u16 {
    let base = match month {
        1u16 => 31u16, 2u16 => 28u16, 3u16 => 31u16, 4u16 => 30u16,
        5u16 => 31u16, 6u16 => 30u16, 7u16 => 31u16, 8u16 => 31u16,
        9u16 => 30u16, 10u16 => 31u16, 11u16 => 30u16, 12u16 => 31u16,
        _ => 0u16,
    };
    if month == 2u16 && is_leap != 0u16 { 29u16 } else { base }
}

fn run(year: u16, month: u16, day: u16) -> u16 {
    let by4 = year % 4u16 == 0u16;
    let by100 = year % 100u16 == 0u16;
    let by400 = year % 400u16 == 0u16;
    let is_leap = (by4 && (!by100 || by400)) as u16;

    let mut total = day;
    let mut m = 1u16;
    while m < month {
        total = total + month_len(m, is_leap);
        m = m + 1u16;
    }
    total
}
