//! Actual/360 day-count year fraction: actual calendar days between two Gregorian dates (days_between's own Rata-Die serial-day technique, reused here) divided by a fixed 360-day year -- the plainest of the five day-count conventions, with no month-length adjustment unlike 30/360 US/European and no actual/365 or actual/actual denominator swap.
//! tags: day-count, daycount, basis, actual-360, act-360, act360, year-fraction, accrual, bond, coupon, money-market, calendar, date, days-between, f32, wide, u32
//! entry: DayCountAct360::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the day difference exceeds u16::MAX (same ~179-year threshold as days_between); no invalid-date validation (that's is_valid_date's job elsewhere)
fn serial_day(y: u16, m: u16, d: u16) -> u32 {
    let y32 = y as u32;
    let m32 = m as u32;
    let d32 = d as u32;
    let a = (14u32 - m32) / 12u32;
    let yy = y32 + 4800u32 - a;
    let mm = m32 + 12u32 * a - 3u32;
    d32 + (153u32 * mm + 2u32) / 5u32 + 365u32 * yy + yy / 4u32 - yy / 100u32 + yy / 400u32 - 32045u32
}

struct DayCountAct360 {
    y1: u16,
    m1: u16,
    d1: u16,
    y2: u16,
    m2: u16,
    d2: u16,
    year_fraction: f32,
}
impl DayCountAct360 {
    fn run(&mut self) -> u16 {
        let s1 = serial_day(self.y1, self.m1, self.d1);
        let s2 = serial_day(self.y2, self.m2, self.d2);
        let diff = if s1 >= s2 { s1 - s2 } else { s2 - s1 };
        if diff > 65535u32 { halt(0xFF05u16); }
        let days_f = int_to_f32(diff);
        let result = days_f / 360.0f32;
        self.year_fraction = result;
        1u16
    }
}
