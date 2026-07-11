//! Actual/365 Fixed day-count year fraction between two Gregorian dates: actual calendar days elapsed (days_between's Rata-Die serial-day technique, reused here) divided by a FIXED 365-day year every time -- never 366, even across a leap year, which is what makes this Actual/365 Fixed and distinct from Actual/Actual (prorates against each date's own real 365- or 366-day year) and from Actual/360 (fixed 360-day divisor instead); equivalent to Excel's YEARFRAC(start_date, end_date, 3).
//! tags: finance, day-count, daycount, basis, actual-365, actual-365-fixed, act-365, year-fraction, yearfrac, calendar, bond, coupon, f32, wide, prerequisite
//! entry: DayCountAct365::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the day span exceeds u16::MAX (roughly 179 years apart, the same ceiling days_between uses)
fn serial_day(y: u16, m: u16, d: u16) -> u32 {
    let y32 = y as u32;
    let m32 = m as u32;
    let d32 = d as u32;
    let a = (14u32 - m32) / 12u32;
    let yy = y32 + 4800u32 - a;
    let mm = m32 + 12u32 * a - 3u32;
    d32 + (153u32 * mm + 2u32) / 5u32 + 365u32 * yy + yy / 4u32 - yy / 100u32 + yy / 400u32 - 32045u32
}

struct DayCountAct365 {
    y1: u16,
    m1: u16,
    d1: u16,
    y2: u16,
    m2: u16,
    d2: u16,
    days: u16,
    year_fraction: f32,
}
impl DayCountAct365 {
    fn run(&mut self) -> u16 {
        let s1 = serial_day(self.y1, self.m1, self.d1);
        let s2 = serial_day(self.y2, self.m2, self.d2);
        let diff = if s1 >= s2 { s1 - s2 } else { s2 - s1 };
        if diff > 65535u32 { halt(0xFF05u16); }
        let days_u16 = diff as u16;
        self.days = days_u16;
        let days_f = int_to_f32(diff);
        self.year_fraction = days_f / 365.0f32;
        1u16
    }
}
