//! Number of days in a month (1-12; 0 for an invalid month), given a leap-year flag for February.
//! tags: calendar, date, days-in-month, month, gregorian
//! limits: month must be 1-12 (returns 0 otherwise); is_leap is a 0/1 flag, not a year (compose with is_leap_year)
fn run(month: u16, is_leap: u16) -> u16 {
    let base = match month {
        1u16 => 31u16, 2u16 => 28u16, 3u16 => 31u16, 4u16 => 30u16,
        5u16 => 31u16, 6u16 => 30u16, 7u16 => 31u16, 8u16 => 31u16,
        9u16 => 30u16, 10u16 => 31u16, 11u16 => 30u16, 12u16 => 31u16,
        _ => 0u16,
    };
    if month == 2u16 && is_leap != 0u16 { 29u16 } else { base }
}
