//! Excel DAYS360(start_date, end_date, [method]): 30/360-convention day count between two Gregorian dates dispatched by a method flag (0/FALSE = US NASD, 1/TRUE = European), inlining day_count_30_360_us's own conditional chain for method 0 (start date's day 31 clamps to 30 unconditionally, but the end date's day 31 only clamps to 30 when the start date was ALSO adjusted to 30, otherwise stays exactly as-is -- the same chain excel_coupdaybs.rs's basis-0 branch already runs) and day_count_30_360_eu's unconditional independent per-date clamp for method 1 (day 31 on EITHER date becomes 30 regardless of the other date), days = (Y2-Y1)*360 + (M2-M1)*30 + (D2-D1) either way, returned as a (magnitude, sign) pair since an end date before the start date is a valid negative span, not an error -- distinct from day_count_30_360_us/eu themselves (fixed single-basis primitives with no method argument at all) and from excel_days (whole-day Rata-Die span, no 30/360 month-length adjustment whatsoever).
//! tags: excel, days360, day-count, daycount, 30-360, thirty-360, us, nasd, european, eu, isda, method, method-flag, dispatch, basis, calendar, date, date-span, year-fraction, bond, coupon, accrual, sign-magnitude, wide, u32
//! entry: ExcelDays360::run
//! limits: escalates (halt 0xFF06, out_of_domain) if either day is 0 or >31, either month is 0 or >12, or method is anything other than 0 or 1; the magnitude is exact across the full u16 year/month/day domain (worst case |span| = 65535*360+12*30+31, well inside u32::MAX) so no overflow escalation is reachable -- the same proof day_count_30_360_eu's own doc comment makes for its identical ordinal construction
struct ExcelDays360 {
    y1: u16,
    m1: u16,
    d1: u16,
    y2: u16,
    m2: u16,
    d2: u16,
    method: u16,
    days_mag: u32,
    days_neg: u16,
}
impl ExcelDays360 {
    fn run(&mut self) -> u16 {
        if self.d1 == 0u16 || self.d1 > 31u16 || self.d2 == 0u16 || self.d2 > 31u16 {
            halt(0xFF06u16);
        }
        if self.m1 == 0u16 || self.m1 > 12u16 || self.m2 == 0u16 || self.m2 > 12u16 {
            halt(0xFF06u16);
        }
        if self.method > 1u16 {
            halt(0xFF06u16);
        }

        let mut dd1 = self.d1;
        let mut dd2 = self.d2;
        if self.method == 0u16 {
            // US/NASD (day_count_30_360_us's own conditional chain, inlined -- each cell
            // compiles standalone against the shared kernel prelude only, so cross-cell
            // logic is duplicated rather than called): the start date's day 31 clamps to 30
            // unconditionally, but the end date's day 31 only clamps to 30 when the start
            // date was ALSO 30 (either originally, or just adjusted on the line above) --
            // otherwise the end date's day 31 is left exactly as-is, the same conditional
            // chain excel_coupdaybs.rs's basis-0 branch already runs.
            if dd1 == 31u16 { dd1 = 30u16; }
            if dd2 == 31u16 && dd1 == 30u16 { dd2 = 30u16; }
        } else {
            // European/ISDA (day_count_30_360_eu's own formula, inlined): day 31 on EITHER
            // date clamps to 30 independently of the other date, no conditional chain at all.
            if dd1 == 31u16 { dd1 = 30u16; }
            if dd2 == 31u16 { dd2 = 30u16; }
        }

        let s1 = (self.y1 as u32) * 360u32 + (self.m1 as u32) * 30u32 + (dd1 as u32);
        let s2 = (self.y2 as u32) * 360u32 + (self.m2 as u32) * 30u32 + (dd2 as u32);
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
