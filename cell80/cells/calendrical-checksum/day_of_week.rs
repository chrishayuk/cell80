//! Day of week for a Gregorian date via Zeller's congruence: 0=Saturday, 1=Sunday, 2=Monday, ... 6=Friday.
//! tags: calendar, date, weekday, zeller, day-of-week, gregorian
//! limits: year must be >= 1 (Jan/Feb dates underflow the internal year-1 adjustment at year 0)
fn run(year: u16, month: u16, day: u16) -> u16 {
    let mut m = month;
    let mut y = year;
    if m < 3u16 { m = m + 12u16; y = y - 1u16; }
    let k = y % 100u16;
    let j = y / 100u16;
    let term = (13u16 * (m + 1u16)) / 5u16;
    (day + term + k + k / 4u16 + j / 4u16 + 5u16 * j) % 7u16
}
