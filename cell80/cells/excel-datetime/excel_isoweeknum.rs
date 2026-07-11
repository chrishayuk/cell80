//! Excel ISOWEEKNUM(date): ISO 8601 week number of the year -- weeks start Monday and week 1 is the week containing the year's first Thursday (equivalently, the week containing January 4th); late-December dates that actually fall in the FOLLOWING year's week 1, and early-January dates that actually fall in the PRECEDING year's week 52/53, are resolved by re-deriving the boundary year's own January 1st weekday (day_of_week's own Zeller's-congruence formula, cell80/cells/calendrical-checksum/day_of_week.rs, inlined and reused a second time here for the boundary year) rather than composing with WEEKNUM (excel_weeknum.rs's own non-ISO convention: week 1 there just holds January 1st, and the week-start weekday is caller-selected via return_type 1/2, rather than always Monday) or day_of_year (an ordinal position within a single year, no year-boundary week-folding at all).
//! tags: excel, isoweeknum, iso-week, iso8601, week-number, week-of-year, monday-start, first-thursday, year-boundary, long-year, 53-week-year, calendar-week, gregorian, datetime
//! entry: ExcelIsoweeknum::run
//! limits: escalates (halt 0xFF06, out_of_domain) if month is outside 1-12, or if year < 2 -- the earliest-January boundary case (raw week 0) needs the PRIOR year's own January 1st weekday, and that lookup itself steps back one further year inside Zeller's month<3 adjustment, so year must clear that double step; day_of_week's own plain floor is year >= 1
fn is_leap_year(year: u16) -> u16 {
    let by4 = year % 4u16 == 0u16;
    let by100 = year % 100u16 == 0u16;
    let by400 = year % 400u16 == 0u16;
    (by4 && (!by100 || by400)) as u16
}

// day_of_year's own leap-year check and month-length table
// (cell80/cells/calendrical-checksum/day_of_year.rs), inlined.
fn ordinal_day(year: u16, month: u16, day: u16) -> u16 {
    let is_leap = is_leap_year(year);
    let mut ordinal = day;
    let mut m = 1u16;
    while m < month {
        let base = match m {
            1u16 => 31u16, 2u16 => 28u16, 3u16 => 31u16, 4u16 => 30u16,
            5u16 => 31u16, 6u16 => 30u16, 7u16 => 31u16, 8u16 => 31u16,
            9u16 => 30u16, 10u16 => 31u16, 11u16 => 30u16, 12u16 => 31u16,
            _ => 0u16,
        };
        let mlen = if m == 2u16 && is_leap != 0u16 { 29u16 } else { base };
        ordinal = ordinal + mlen;
        m = m + 1u16;
    }
    ordinal
}

// day_of_week's own Zeller congruence (cell80/cells/calendrical-checksum/day_of_week.rs),
// inlined -- each cell compiles standalone against the shared kernel prelude only, so
// cross-cell logic is duplicated rather than called. Rotated from its own fixed
// 0=Saturday..6=Friday code onto ISO weekday numbering (Monday=1 ... Sunday=7).
fn iso_weekday(year: u16, month: u16, day: u16) -> u16 {
    let mut zm = month;
    let mut zy = year;
    if zm < 3u16 { zm = zm + 12u16; zy = zy - 1u16; }
    let zk = zy % 100u16;
    let zj = zy / 100u16;
    let zterm = (13u16 * (zm + 1u16)) / 5u16;
    let z = (day + zterm + zk + zk / 4u16 + zj / 4u16 + 5u16 * zj) % 7u16;
    ((z + 5u16) % 7u16) + 1u16
}

// Does `year` have 53 ISO weeks (a "long year")? True iff January 1st is an ISO Thursday
// (4), or `year` is a leap year and January 1st is an ISO Wednesday (3) -- the standard
// ISO 8601 long-year test, reusing iso_weekday/is_leap_year above rather than a separate
// p(y)-style residue formula.
fn long_year_weeks(year: u16) -> u16 {
    let jan1_wd = iso_weekday(year, 1u16, 1u16);
    if jan1_wd == 4u16 {
        53u16
    } else if jan1_wd == 3u16 && is_leap_year(year) != 0u16 {
        53u16
    } else {
        52u16
    }
}

struct ExcelIsoweeknum {
    year: u16,
    month: u16,
    day: u16,
    week: u16,
}
impl ExcelIsoweeknum {
    fn run(&mut self) -> u16 {
        if self.month < 1u16 || self.month > 12u16 {
            halt(0xFF06u16);
        }
        if self.year < 2u16 {
            halt(0xFF06u16);
        }

        let ordinal = ordinal_day(self.year, self.month, self.day);
        let wd = iso_weekday(self.year, self.month, self.day);

        // Standard ISO week formula, raw = floor((ordinal - wd + 10) / 7), reordered so the
        // subtraction never goes negative while unsigned: ordinal >= 1 and wd <= 7, so
        // (ordinal + 10) always exceeds wd.
        let raw = (ordinal + 10u16 - wd) / 7u16;

        let week = if raw == 0u16 {
            // Falls in the last week (52 or 53) of the PRECEDING year.
            long_year_weeks(self.year - 1u16)
        } else if raw == 53u16 {
            // Only a genuine week 53 if this year is actually a long year; otherwise it
            // folds forward into week 1 of the FOLLOWING year.
            if long_year_weeks(self.year) == 53u16 { 53u16 } else { 1u16 }
        } else {
            raw
        };

        self.week = week;
        1u16
    }
}
