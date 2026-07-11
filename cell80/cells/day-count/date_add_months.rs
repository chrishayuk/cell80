//! Steps a Gregorian (year, month, day) forward or backward by a whole number of months (direction 0=forward/add, 1=backward/subtract, magnitude months: u16), clamping the result day to the target month's real length in one direct jump (e.g. Jan 31 stepped forward 1 month lands on Feb 28/29, never Mar 3, and Jan 31 stepped forward 2 months lands squarely on Mar 31 with no intermediate Feb clamp) -- the shared foundation the whole Excel COUP* family (COUPNCD, COUPPCD, COUPDAYBS, COUPDAYS, COUPDAYSNC, COUPNUM) walks backward over in (12/frequency)-month increments, distinct from days_between (spans whole days, not months) and is_valid_date (checks a given date, doesn't derive a new one).
//! tags: calendar, date, month, add-months, edate, step, shift, end-of-month, clamp, gregorian, coupon, day-count, prerequisite
//! entry: DateAddMonths::run
//! limits: escalates (halt 0xFF06, out_of_domain) if month is outside 1-12, or if stepping backward would take the date before year 0; escalates (halt 0xFF05, needs_wider_math) if stepping forward would take the year past 65535
struct DateAddMonths {
    year: u16,
    month: u16,
    day: u16,
    months: u16,
    direction: u16,
    new_year: u16,
    new_month: u16,
    new_day: u16,
}
impl DateAddMonths {
    fn run(&mut self) -> u16 {
        if self.month < 1u16 || self.month > 12u16 {
            halt(0xFF06u16);
        }

        // Zero-based absolute month index counted from epoch year 0: idx = year*12 + (month-1).
        // Widened to u32 since year (up to 65535) * 12 already exceeds u16.
        let idx = (self.year as u32) * 12u32 + (self.month as u32 - 1u32);
        let step = self.months as u32;

        let mut new_idx = 0u32;
        if self.direction == 0u16 {
            new_idx = idx + step;
        } else {
            if step > idx {
                halt(0xFF06u16);
            }
            new_idx = idx - step;
        }

        let new_year32 = new_idx / 12u32;
        if new_year32 > 65535u32 {
            halt(0xFF05u16);
        }
        let new_month0 = new_idx % 12u32;
        let new_year = new_year32 as u16;
        let new_month = (new_month0 + 1u32) as u16;

        // is_leap_year's own formula (cell80/cells/calendrical-checksum/is_leap_year.rs),
        // inlined -- each cell compiles standalone against the shared kernel prelude only,
        // so cross-cell logic is duplicated rather than called.
        let by4 = new_year % 4u16 == 0u16;
        let by100 = new_year % 100u16 == 0u16;
        let by400 = new_year % 400u16 == 0u16;
        let is_leap = (by4 && (!by100 || by400)) as u16;

        // days_in_month's own table (cell80/cells/calendrical-checksum/days_in_month.rs),
        // inlined for the same reason.
        let base = match new_month {
            1u16 => 31u16, 2u16 => 28u16, 3u16 => 31u16, 4u16 => 30u16,
            5u16 => 31u16, 6u16 => 30u16, 7u16 => 31u16, 8u16 => 31u16,
            9u16 => 30u16, 10u16 => 31u16, 11u16 => 30u16, 12u16 => 31u16,
            _ => 0u16,
        };
        let max_day = if new_month == 2u16 && is_leap != 0u16 { 29u16 } else { base };

        let clamped_day = if self.day > max_day { max_day } else { self.day };

        self.new_year = new_year;
        self.new_month = new_month;
        self.new_day = clamped_day;
        1u16
    }
}
