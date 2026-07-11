//! Excel NETWORKDAYS.INTL(start_date, end_date, [weekend]): counts the whole workdays between two dates, inclusive of both endpoints, skipping every day that falls on a caller-chosen weekend pattern instead of NETWORKDAYS's fixed Saturday/Sunday -- the weekend pattern is represented as a 7-bit mask (bit i set means day_of_week's own day-of-week code i, 0=Saturday..6=Friday convention, is a weekend day), a deliberate divergence from Excel's weekend string/number argument encoding since the dialect has no strings; returned sign-magnitude (workdays_mag/workdays_neg) the same way excel_days is, since Excel returns a negative count when start_date falls after end_date -- distinct from excel_days (counts ALL elapsed days, weekends included, no workday filtering at all) and from is_weekend/day_of_week (test or derive a single day's status, never accumulate a count across a date range). No holidays argument: the same deliberate omission NETWORKDAYS itself would carry without its optional list.
//! tags: excel, networkdays, networkdays-intl, workdays, business-days, weekend, weekend-mask, custom-weekend, weekend-pattern, bitmask, date-range, date, datetime, calendar, gregorian, sign-magnitude, signed, rata-die, julian-day, wide, u32, checked, escalate
//! entry: ExcelNetworkdaysIntl::run
//! limits: weekend_mask must fit in 7 bits (bits 0-6, day_of_week's 0=Saturday..6=Friday convention) -- halts (0xFF06, out_of_domain) if any bit above bit 6 is set; escalates (halt 0xFF05, needs_wider_math) if the inclusive day span between the two dates would exceed 65535 days (~179 years, matching excel_days/days_between's own limit); inherits day_of_week's own year>=1 limit since it reuses Zeller's congruence internally to find the starting day-of-week (Jan/Feb dates underflow the internal year-1 adjustment at year 0)
fn serial_day(y: u16, m: u16, d: u16) -> u32 {
    let y32 = y as u32;
    let m32 = m as u32;
    let d32 = d as u32;
    let a = (14u32 - m32) / 12u32;
    let yy = y32 + 4800u32 - a;
    let mm = m32 + 12u32 * a - 3u32;
    d32 + (153u32 * mm + 2u32) / 5u32 + 365u32 * yy + yy / 4u32 - yy / 100u32 + yy / 400u32 - 32045u32
}

fn zeller_dow(year: u16, month: u16, day: u16) -> u16 {
    let mut m = month;
    let mut y = year;
    if m < 3u16 { m = m + 12u16; y = y - 1u16; }
    let k = y % 100u16;
    let j = y / 100u16;
    let term = (13u16 * (m + 1u16)) / 5u16;
    (day + term + k + k / 4u16 + j / 4u16 + 5u16 * j) % 7u16
}

struct ExcelNetworkdaysIntl {
    y1: u16,
    m1: u16,
    d1: u16,
    y2: u16,
    m2: u16,
    d2: u16,
    weekend_mask: u16,
    workdays_mag: u16,
    workdays_neg: u16,
}
impl ExcelNetworkdaysIntl {
    fn run(&mut self) -> u16 {
        if self.weekend_mask > 127u16 {
            halt(0xFF06u16);
        }

        let s1 = serial_day(self.y1, self.m1, self.d1);
        let s2 = serial_day(self.y2, self.m2, self.d2);
        let dow1 = zeller_dow(self.y1, self.m1, self.d1);
        let dow2 = zeller_dow(self.y2, self.m2, self.d2);

        let ordered = s1 <= s2;
        let neg = if ordered { 0u16 } else { 1u16 };
        let start = if ordered { s1 } else { s2 };
        let end = if ordered { s2 } else { s1 };
        let start_dow = if ordered { dow1 } else { dow2 };

        let diff = end - start;
        if diff > 65534u32 {
            halt(0xFF05u16);
        }
        let total_days = (diff + 1u32) as u16;

        // Popcount the low 7 bits of the weekend mask -- how many distinct
        // weekdays out of 7 are marked weekend.
        let mut weekend_count = 0u16;
        let mut b = 0u16;
        while b < 7u16 {
            let bit = (self.weekend_mask >> b) & 1u16;
            weekend_count = weekend_count + bit;
            b = b + 1u16;
        }
        let workdays_per_week = 7u16 - weekend_count;

        // Every full 7-day week contributes exactly workdays_per_week
        // workdays regardless of which day it starts on; only the leftover
        // (< 7 day) remainder needs per-day checking against the mask.
        let full_weeks = total_days / 7u16;
        let remainder = total_days % 7u16;
        let full_week_workdays = full_weeks * workdays_per_week;

        let mut rem_workdays = 0u16;
        let mut i = 0u16;
        let mut cur_dow = start_dow;
        while i < remainder {
            let is_weekend_day = (self.weekend_mask >> cur_dow) & 1u16;
            if is_weekend_day == 0u16 {
                rem_workdays = rem_workdays + 1u16;
            }
            cur_dow = (cur_dow + 1u16) % 7u16;
            i = i + 1u16;
        }

        self.workdays_mag = full_week_workdays + rem_workdays;
        self.workdays_neg = neg;
        1u16
    }
}
