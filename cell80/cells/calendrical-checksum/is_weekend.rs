//! Returns 1 if dow (a day-of-week code as produced by day_of_week: 0=Saturday, 1=Sunday, 2=Monday...6=Friday) is Saturday or Sunday, else 0.
//! tags: calendar, date, weekday, weekend, predicate, day-of-week
fn run(dow: u16) -> u16 {
    (dow == 0u16 || dow == 1u16) as u16
}
