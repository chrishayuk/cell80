//! Actual/Actual day-count year fraction between two dates: the numerator is the real calendar day count (days_between's own Rata-Die serial-day technique, inlined -- each cell compiles standalone against the shared kernel prelude only, so cross-cell logic is duplicated rather than called), the denominator is 366 if the actual-day span includes a Feb 29 and 365 otherwise -- a single well-defined simplification of the multi-period ISDA actual/actual standard (whose coupon-frequency-weighted subtleties are out of scope for a tiny cell), returned as an unreduced num/den pair so the caller can see which basis applied -- distinct from this pack's forthcoming fixed-denominator actual/360 and actual/365 siblings by deriving its denominator from the span's own leap-day content instead of a constant.
//! tags: calendar, date, day-count, daycount, year-fraction, yearfrac, actual-actual, act-act, basis, convention, bond, coupon, accrual, leap-year, denominator, fraction, prerequisite, wide, u32
//! entry: DayCountActAct::run
//! limits: does not validate y/m/d as genuine calendar dates (garbage in, garbage out -- matches days_between's own convention); the Feb-29 scan loops once per calendar year the span touches, so it's sized for realistic accrual spans (bond coupon periods, at most a handful of years), not multi-century inputs

// days_between's own Rata-Die serial-day technique (cell80/cells/calendrical-checksum/days_between.rs),
// inlined -- each cell compiles standalone against the shared kernel prelude only, so cross-cell
// logic is duplicated rather than called.
fn serial_day(y: u16, m: u16, d: u16) -> u32 {
    let y32 = y as u32;
    let m32 = m as u32;
    let d32 = d as u32;
    let a = (14u32 - m32) / 12u32;
    let yy = y32 + 4800u32 - a;
    let mm = m32 + 12u32 * a - 3u32;
    d32 + (153u32 * mm + 2u32) / 5u32 + 365u32 * yy + yy / 4u32 - yy / 100u32 + yy / 400u32 - 32045u32
}

struct DayCountActAct {
    y1: u16,
    m1: u16,
    d1: u16,
    y2: u16,
    m2: u16,
    d2: u16,
    num: u32,
    den: u32,
}
impl DayCountActAct {
    fn run(&mut self) -> u16 {
        let s1 = serial_day(self.y1, self.m1, self.d1);
        let s2 = serial_day(self.y2, self.m2, self.d2);
        let s_lo = if s1 <= s2 { s1 } else { s2 };
        let s_hi = if s1 <= s2 { s2 } else { s1 };
        let y_lo = if self.y1 <= self.y2 { self.y1 } else { self.y2 };
        let y_hi = if self.y1 <= self.y2 { self.y2 } else { self.y1 };

        // Scan every calendar year the span touches for a Feb 29 that actually falls
        // inside [s_lo, s_hi] -- this is the ISDA-simplification's single leap-day test,
        // not a general "did a leap year exist nearby" check. is_leap_year's own formula
        // (cell80/cells/calendrical-checksum/is_leap_year.rs), inlined the same way
        // date_add_months.rs does in this pack.
        let mut includes_feb29 = 0u16;
        let mut y = y_lo;
        let mut done = 0u16;
        while done == 0u16 {
            let by4 = y % 4u16 == 0u16;
            let by100 = y % 100u16 == 0u16;
            let by400 = y % 400u16 == 0u16;
            let is_leap = (by4 && (!by100 || by400)) as u16;
            if is_leap != 0u16 {
                let feb29 = serial_day(y, 2u16, 29u16);
                if feb29 >= s_lo && feb29 <= s_hi {
                    includes_feb29 = 1u16;
                }
            }
            if y == y_hi {
                done = 1u16;
            } else {
                y = y + 1u16;
            }
        }

        let diff = if s1 >= s2 { s1 - s2 } else { s2 - s1 };
        self.num = diff;
        let denom = if includes_feb29 == 1u16 { 366u32 } else { 365u32 };
        self.den = denom;
        1u16
    }
}
