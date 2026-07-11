//! Returns 1 if (year, month, day) is a genuinely valid Gregorian date -- month in 1-12 and day within that month's actual leap-year-aware length -- else 0; distinct from range_check's single static bound.
//! tags: calendar, date, validation, gregorian, leap-year, predicate
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

    let month_ok = month >= 1u16 && month <= 12u16;
    let len = if month_ok { month_len(month, is_leap) } else { 0u16 };
    (month_ok && day >= 1u16 && day <= len) as u16
}
