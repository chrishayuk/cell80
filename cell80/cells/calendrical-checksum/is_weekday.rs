//! Returns 1 if a day-of-week code (day_of_week's convention: 0=Saturday, 1=Sunday, 2=Monday, ... 6=Friday) falls Monday through Friday, else 0 -- the direct logical complement of is_weekend.
//! tags: calendar, date, weekday, weekend, predicate, day-of-week
fn run(dow: u16) -> u16 {
    (dow >= 2u16 && dow <= 6u16) as u16
}
