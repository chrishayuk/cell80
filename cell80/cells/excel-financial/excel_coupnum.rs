//! Number of coupons payable between settlement and maturity (Excel COUPNUM(settlement, maturity, frequency, [basis])): counts coupon dates by stepping maturity backward one (12/frequency)-month period at a time -- via date_add_months' end-of-month-clamping logic, inlined and chained onto each previously-stepped date in turn, since cells can't call each other, the same loop excel_coupncd.rs runs -- until a step lands on or before settlement, returning how many steps (including maturity itself) stayed strictly after settlement; distinct from COUPNCD/COUPPCD (return the bracketing coupon DATE itself, not how many there are) and from COUPDAYS/COUPDAYBS (return a day count within one coupon period, not a period tally), and from the day-count-fraction prerequisite cells (day_count_30_360_us/eu, day_count_act_act, day_count_act_360, day_count_act_365 -- those turn a fixed date pair into a year-fraction, never touched here since COUPNUM's tally is pure calendar month-stepping, independent of any day-count convention); the optional 4th Excel argument (basis, default 0) is accepted by real Excel but never changes this count, so it is intentionally not a field here.
//! tags: excel, coupnum, coupon, coupon-count, number-of-coupons, bond, schedule, frequency, date-step, edate, settlement, maturity, day-count-and-coupons
//! entry: ExcelCoupnum::run
//! limits: escalates (halt 0xFF06, out_of_domain) if frequency isn't 1, 2, or 4; if either date's month is outside 1-12; if maturity is not strictly after settlement; or if stepping backward would underflow past year 0 (mirrors date_add_months' own guard, effectively unreachable for real bond dates). Escalates (halt 0xFF05, needs_wider_math) if the bounded loop (2000 periods, ample for any realistic bond term at any supported frequency) is exhausted without settlement being crossed.
fn serial_day(y: u16, m: u16, d: u16) -> u32 {
    let y32 = y as u32;
    let m32 = m as u32;
    let d32 = d as u32;
    let a = (14u32 - m32) / 12u32;
    let yy = y32 + 4800u32 - a;
    let mm = m32 + 12u32 * a - 3u32;
    d32 + (153u32 * mm + 2u32) / 5u32 + 365u32 * yy + yy / 4u32 - yy / 100u32 + yy / 400u32 - 32045u32
}

struct ExcelCoupnum {
    sy: u16,
    sm: u16,
    sd: u16,
    my: u16,
    mm: u16,
    md: u16,
    frequency: u16,
    num_coupons: u16,
}
impl ExcelCoupnum {
    fn run(&mut self) -> u16 {
        let freq_ok = self.frequency == 1u16 || self.frequency == 2u16 || self.frequency == 4u16;
        if !freq_ok { halt(0xFF06u16); }
        if self.sm < 1u16 || self.sm > 12u16 || self.mm < 1u16 || self.mm > 12u16 {
            halt(0xFF06u16);
        }

        let settlement_serial = serial_day(self.sy, self.sm, self.sd);
        let maturity_serial = serial_day(self.my, self.mm, self.md);
        if maturity_serial <= settlement_serial { halt(0xFF06u16); }

        let step_months = 12u16 / self.frequency;

        // Chain backward from maturity one (12/frequency)-month period at a time, the
        // same date_add_months-clamping loop excel_coupncd.rs runs (inlined again, since
        // cells can't call each other). Maturity itself always counts as the first
        // payable coupon (count starts at 1); each further step that still lands
        // strictly after settlement adds one more.
        let mut cur_y = self.my;
        let mut cur_m = self.mm;
        let mut cur_d = self.md;
        let mut count = 1u16;

        let mut i = 0u16;
        let max_iter = 2000u16;
        let mut found = 0u16;
        while i < max_iter {
            let idx = (cur_y as u32) * 12u32 + (cur_m as u32 - 1u32);
            let step = step_months as u32;
            if step > idx { halt(0xFF06u16); }
            let new_idx = idx - step;
            let new_year32 = new_idx / 12u32;
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
            let clamped_day = if cur_d > max_day { max_day } else { cur_d };

            let next_serial = serial_day(new_year, new_month, clamped_day);
            if next_serial <= settlement_serial {
                i = max_iter;
                found = 1u16;
            } else {
                cur_y = new_year;
                cur_m = new_month;
                cur_d = clamped_day;
                count = count + 1u16;
                i = i + 1u16;
            }
        }
        if found == 0u16 { halt(0xFF05u16); }

        self.num_coupons = count;
        1u16
    }
}
