//! Excel WEEKNUM(date, return_type): non-ISO week number of the year, where week 1 is the week containing January 1 and every later week is one more 7-day block counted from whichever weekday the week starts on -- return_type 1 starts weeks on Sunday, return_type 2 starts weeks on Monday (WEEKDAY's own two numbering conventions, inlined here rather than called), distinct from an ISO week number (WEEKNUM's return_type 21, not covered here) where week 1 is instead the week holding the year's first Thursday and weeks always start Monday.
//! tags: excel, weeknum, week-number, week-of-year, calendar-week, return-type, week-start-convention, sunday-start, monday-start, non-iso, day-of-year, gregorian, datetime
//! entry: ExcelWeeknum::run
//! limits: only return_type 1 (Sunday-start) and return_type 2 (Monday-start) are supported, matching the brief; escalates (halt 0xFF06, out_of_domain) for any other return_type, for month outside 1-12, or for year 0 (the internal Jan-1 weekday calculation steps to year-1, which underflows at year 0, same restriction as day_of_week)
struct ExcelWeeknum {
    year: u16,
    month: u16,
    day: u16,
    return_type: u16,
    week: u16,
}
impl ExcelWeeknum {
    fn run(&mut self) -> u16 {
        if self.month < 1u16 || self.month > 12u16 {
            halt(0xFF06u16);
        }
        if self.return_type != 1u16 && self.return_type != 2u16 {
            halt(0xFF06u16);
        }
        if self.year < 1u16 {
            halt(0xFF06u16);
        }

        // day_of_year's own leap-year check and month-length table
        // (cell80/cells/calendrical-checksum/day_of_year.rs), inlined.
        let by4 = self.year % 4u16 == 0u16;
        let by100 = self.year % 100u16 == 0u16;
        let by400 = self.year % 400u16 == 0u16;
        let is_leap = (by4 && (!by100 || by400)) as u16;

        let mut doy = self.day;
        let mut m = 1u16;
        while m < self.month {
            let base = match m {
                1u16 => 31u16, 2u16 => 28u16, 3u16 => 31u16, 4u16 => 30u16,
                5u16 => 31u16, 6u16 => 30u16, 7u16 => 31u16, 8u16 => 31u16,
                9u16 => 30u16, 10u16 => 31u16, 11u16 => 30u16, 12u16 => 31u16,
                _ => 0u16,
            };
            let mlen = if m == 2u16 && is_leap != 0u16 { 29u16 } else { base };
            doy = doy + mlen;
            m = m + 1u16;
        }

        // day_of_week's own Zeller congruence (cell80/cells/calendrical-checksum/day_of_week.rs),
        // inlined and specialized for month=1, day=1 (January 1st of this year): with month
        // fixed at 1, Zeller's month-adjustment always pushes it to m=13 of the PRIOR year,
        // so its term=(13*(m+1))/5 collapses to the fixed constant (13*14)/5 = 36.
        let y = self.year - 1u16;
        let k = y % 100u16;
        let j = y / 100u16;
        let z = (37u16 + k + k / 4u16 + j / 4u16 + 5u16 * j) % 7u16;

        // Map Zeller's 0=Saturday..6=Friday code onto Excel's WEEKDAY numbering for the
        // requested return_type: type 1 is Sunday=1..Saturday=7, type 2 is Monday=1..Sunday=7.
        let jan1_wd = if self.return_type == 1u16 {
            if z == 0u16 { 7u16 } else { z }
        } else {
            ((z + 5u16) % 7u16) + 1u16
        };

        // Week 1 holds Jan 1; every further complete 7-day block (counted from the weekday
        // the return_type says a week starts on) advances the week number by one.
        let week = (doy + jan1_wd - 2u16) / 7u16 + 1u16;
        self.week = week;
        1u16
    }
}
