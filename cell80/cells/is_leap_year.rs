//! Returns 1 if year is a Gregorian leap year, else 0: divisible by 4, except centuries not divisible by 400.
//! tags: calendar, date, leap-year, gregorian, year, validation
fn run(year: u16) -> u16 {
    let by4 = year % 4u16 == 0u16;
    let by100 = year % 100u16 == 0u16;
    let by400 = year % 400u16 == 0u16;
    (by4 && (!by100 || by400)) as u16
}
