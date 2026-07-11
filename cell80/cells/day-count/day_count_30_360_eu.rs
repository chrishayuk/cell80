//! European (ISDA) 30/360 day count from (y1,m1,d1) to (y2,m2,d2): day-of-month 31 on EITHER date is clamped to 30 independently of the other date (Excel's DAYS360 with method=TRUE) -- unlike day_count_30_360_us, D2's clamp never depends on whether D1 was 30 or 31, so there is no conditional chain at all, just days = (Y2-Y1)*360 + (M2-M1)*30 + (D2-D1), returned as a (magnitude, sign) pair since an end date before the start date is a valid negative span, not an error.
//! tags: day-count, daycount, 30-360, thirty-360, european, eu, isda, dcc, day-count-convention, calendar, date, date-span, year-fraction, bond, coupon, accrual, days360, sign-magnitude, wide, u32
//! entry: DayCount30360Eu::run
//! limits: escalates (halt 0xFF06, out_of_domain) if either day is 0 or >31, or either month is 0 or >12; the magnitude is exact across the full u16 year/month/day domain (worst case |span| = 65535*360+11*30+30, well inside u32::MAX) so no overflow escalation is reachable -- proven, not just unobserved, the same standing carmichael_lambda's doc comment claims for its own u32 headroom
fn eu360_ordinal(y: u16, m: u16, d: u16) -> u32 {
    let dd = if d == 31u16 { 30u16 } else { d };
    (y as u32) * 360u32 + (m as u32) * 30u32 + (dd as u32)
}

struct DayCount30360Eu { y1: u16, m1: u16, d1: u16, y2: u16, m2: u16, d2: u16, days_mag: u32, days_neg: u16 }
impl DayCount30360Eu {
    fn run(&mut self) -> u16 {
        if self.d1 == 0u16 || self.d1 > 31u16 || self.d2 == 0u16 || self.d2 > 31u16 { halt(0xFF06u16); }
        if self.m1 == 0u16 || self.m1 > 12u16 || self.m2 == 0u16 || self.m2 > 12u16 { halt(0xFF06u16); }
        let s1 = eu360_ordinal(self.y1, self.m1, self.d1);
        let s2 = eu360_ordinal(self.y2, self.m2, self.d2);
        if s2 >= s1 {
            self.days_mag = s2 - s1;
            self.days_neg = 0u16;
        } else {
            self.days_mag = s1 - s2;
            self.days_neg = 1u16;
        }
        1u16
    }
}
