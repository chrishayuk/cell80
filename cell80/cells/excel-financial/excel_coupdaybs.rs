//! Excel COUPDAYBS(settlement, maturity, frequency, [basis]): days from the previous coupon date (PCD) to settlement, found by stepping maturity backward one (12/frequency)-month period at a time -- the same chained date_add_months-clamping loop excel_coupncd.rs runs, inlined again since cells can't call each other -- until landing on or before settlement, then a basis-dispatched day count from that PCD to settlement (0/4 = 30/360 US/European, distinguished by the US convention's day-31 cross-adjustment vs European's unconditional min(day,30); 1/2/3 = actual/actual, actual/360, actual/365, all three giving the identical raw calendar-day numerator here since COUPDAYBS returns a day count, not a year fraction, so the three actual bases have nothing left to differ by) -- distinct from COUPDAYS (total days spanning the WHOLE coupon period, PCD to NCD) and COUPDAYSNC (days from settlement forward to the NEXT coupon, not backward from PCD), and from COUPPCD (returns the PCD date itself, not a day count from it).
//! tags: excel, coupdaybs, coupon, coupon-date, day-count, bond, settlement, maturity, frequency, basis, 30-360, actual, pcd, date-step, schedule
//! entry: ExcelCoupdaybs::run
//! limits: escalates (halt 0xFF06, out_of_domain) if frequency isn't 1, 2, or 4; if either date's month is outside 1-12; if basis isn't 0-4; if maturity is not strictly after settlement; if stepping backward would underflow past year 0; or if a computed 30/360 total somehow lands settlement before PCD (should be unreachable given PCD <= settlement by construction). Escalates (halt 0xFF05, needs_wider_math) if the bounded loop (2000 periods, ample for any realistic bond term at any supported frequency) is exhausted without settlement being crossed, or if the resulting day count would exceed u16::MAX (unreachable for any single real coupon period).

// Excel signature: COUPDAYBS(settlement, maturity, frequency, [basis]). settlement,
// maturity, and frequency are required (frequency must be 1 = annual, 2 = semiannual, or
// 4 = quarterly -- anything else is Excel's #NUM!). basis is optional and Excel defaults
// it to 0 (30/360 US) when omitted -- callers of this cell pass 0 explicitly for that
// case. Return value is a plain day count (an integer number of days), never a cash
// amount or rate, so none of Excel's outflow-negative sign convention or annuity `type`
// flag applies here.
fn serial_day(y: u16, m: u16, d: u16) -> u32 {
    let y32 = y as u32;
    let m32 = m as u32;
    let d32 = d as u32;
    let a = (14u32 - m32) / 12u32;
    let yy = y32 + 4800u32 - a;
    let mm = m32 + 12u32 * a - 3u32;
    d32 + (153u32 * mm + 2u32) / 5u32 + 365u32 * yy + yy / 4u32 - yy / 100u32 + yy / 400u32 - 32045u32
}

struct ExcelCoupdaybs {
    sy: u16,
    sm: u16,
    sd: u16,
    my: u16,
    mm: u16,
    md: u16,
    frequency: u16,
    basis: u16,
    days_bs: u16,
}
impl ExcelCoupdaybs {
    fn run(&mut self) -> u16 {
        let freq_ok = self.frequency == 1u16 || self.frequency == 2u16 || self.frequency == 4u16;
        if !freq_ok { halt(0xFF06u16); }
        if self.sm < 1u16 || self.sm > 12u16 || self.mm < 1u16 || self.mm > 12u16 {
            halt(0xFF06u16);
        }
        if self.basis > 4u16 { halt(0xFF06u16); }

        let settlement_serial = serial_day(self.sy, self.sm, self.sd);
        let maturity_serial = serial_day(self.my, self.mm, self.md);
        if maturity_serial <= settlement_serial { halt(0xFF06u16); }

        let step_months = 12u16 / self.frequency;

        // Chain backward from maturity one (12/frequency)-month period at a time, the
        // same date_add_months-clamping loop excel_coupncd.rs runs (inlined again, since
        // cells can't call each other) -- but stopping the moment a candidate lands ON OR
        // BEFORE settlement (COUPPCD's own stop rule), unlike COUPNCD which stops one step
        // short (the last candidate still strictly AFTER settlement).
        let mut cur_y = self.my;
        let mut cur_m = self.mm;
        let mut cur_d = self.md;

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
            cur_y = new_year;
            cur_m = new_month;
            cur_d = clamped_day;
            if next_serial <= settlement_serial {
                i = max_iter;
                found = 1u16;
            } else {
                i = i + 1u16;
            }
        }
        if found == 0u16 { halt(0xFF05u16); }

        // cur_y/cur_m/cur_d now holds PCD (on or before settlement). Basis-dispatched day
        // count from PCD to settlement: 0/4 use a 30/360 day count (US adjusts the end day
        // only when the start day was also month-end; European clamps each side to 30
        // independently), 1/2/3 (actual/actual, actual/360, actual/365) all use the same
        // real calendar-day count -- the three actual bases only differ in the denominator
        // used to build a year fraction elsewhere, never in this numerator.
        if self.basis == 0u16 || self.basis == 4u16 {
            let mut d1 = cur_d;
            let mut d2 = self.sd;
            if self.basis == 4u16 {
                if d1 > 30u16 { d1 = 30u16; }
                if d2 > 30u16 { d2 = 30u16; }
            } else {
                if d1 == 31u16 { d1 = 30u16; }
                if d2 == 31u16 && d1 == 30u16 { d2 = 30u16; }
            }
            let total_settle = (self.sy as u32) * 360u32 + (self.sm as u32) * 30u32 + d2 as u32;
            let total_pcd = (cur_y as u32) * 360u32 + (cur_m as u32) * 30u32 + d1 as u32;
            if total_settle < total_pcd { halt(0xFF06u16); }
            let diff = total_settle - total_pcd;
            if diff > 65535u32 { halt(0xFF05u16); }
            self.days_bs = diff as u16;
            return 1u16;
        }

        let pcd_serial = serial_day(cur_y, cur_m, cur_d);
        if pcd_serial > settlement_serial { halt(0xFF06u16); }
        let diff = settlement_serial - pcd_serial;
        if diff > 65535u32 { halt(0xFF05u16); }
        self.days_bs = diff as u16;
        1u16
    }
}
