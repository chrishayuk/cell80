//! Excel YEARFRAC(start_date, end_date, [basis]): a thin basis-dispatch wrapper that inlines whichever of this pack's own day-count techniques the basis code (0=30/360 US, 1=actual/actual, 2=actual/360, 3=actual/365, 4=30/360 European) selects and returns the final year fraction in one call -- unlike the day_count_* cells themselves (each one fixed to a single convention, returning only a raw day count, a num/den pair, or a signed 30/360 magnitude, leaving the basis choice and the final divide to the caller) and unlike excel_accrintm/excel_disc/excel_pricedisc/etc. (which all take an ALREADY-COMPUTED year fraction as an input field -- this is the cell that would produce the value they consume), YEARFRAC both dispatches on basis and divides in a single call, and always returns a non-negative fraction regardless of which date argument the caller passes first (both dates are chronologically ordered internally before any basis formula runs), matching Excel's own YEARFRAC behavior rather than DAYS360's signed, order-preserving convention.
//! tags: excel, yearfrac, year-fraction, day-count, daycount, basis, basis-dispatch, basis-argument, 30-360, thirty-360, us, united-states, european, eu, isda, actual-actual, act-act, actual-360, act-360, actual-365, act-365, calendar, date, date-span, accrual, bond, coupon, f32, wide, u32
//! entry: ExcelYearfrac::run
//! limits: escalates (halt 0xFF06, out_of_domain) if either day is 0 or >31, either month is 0 or >12, or basis is anything other than 0-4; escalates (halt 0xFF05, needs_wider_math) for basis 1/2/3 (actual/actual, actual/360, actual/365) if the real calendar-day span exceeds u16::MAX (~179 years, the same ceiling days_between/day_count_act_360/day_count_act_365 use) -- basis 0/4's 30/360 pseudo-day arithmetic has no such ceiling, exact across the full u16 year/month/day domain the same way day_count_30_360_eu's own limits comment argues; does not validate that (y,m,d) is a genuine calendar date beyond the day/month range check (garbage in, garbage out, matching days_between/day_count_act_act's own convention); the actual/actual leap-day scan loop is sized for realistic accrual spans (a handful of years), not multi-century inputs, mirroring day_count_act_act's own limit.

// Excel signature: YEARFRAC(start_date, end_date, [basis]). start_date and end_date are
// required; basis is optional and Excel defaults it to 0 (US 30/360) when omitted --
// callers of this cell pass basis explicitly. Note: a standalone day_count_30_360_us
// cell was authored for this library and then backed out at the admission gate as a
// probe-bank false positive (fingerprint-agreed with days_between at 1.00 despite a
// confirmed real algorithmic divergence -- docs/excel-financial-map.md's "Backed out"
// note), so no such file exists to read; its US 30/360 day-adjustment formula survives
// inlined in excel_coupdaybs.rs's own basis-0 branch, and is reproduced from that
// precedent here rather than from a no-longer-present standalone source.
fn serial_day(y: u16, m: u16, d: u16) -> u32 {
    let y32 = y as u32;
    let m32 = m as u32;
    let d32 = d as u32;
    let a = (14u32 - m32) / 12u32;
    let yy = y32 + 4800u32 - a;
    let mm = m32 + 12u32 * a - 3u32;
    d32 + (153u32 * mm + 2u32) / 5u32 + 365u32 * yy + yy / 4u32 - yy / 100u32 + yy / 400u32 - 32045u32
}

struct ExcelYearfrac {
    y_start: u16,
    m_start: u16,
    d_start: u16,
    y_end: u16,
    m_end: u16,
    d_end: u16,
    basis: u16,
    year_frac: f32,
}
impl ExcelYearfrac {
    fn run(&mut self) -> u16 {
        if self.d_start == 0u16 || self.d_start > 31u16 || self.d_end == 0u16 || self.d_end > 31u16 {
            halt(0xFF06u16);
        }
        if self.m_start == 0u16 || self.m_start > 12u16 || self.m_end == 0u16 || self.m_end > 12u16 {
            halt(0xFF06u16);
        }
        if self.basis > 4u16 {
            halt(0xFF06u16);
        }

        let s1 = serial_day(self.y_start, self.m_start, self.d_start);
        let s2 = serial_day(self.y_end, self.m_end, self.d_end);

        // Chronologically order the pair (earlier date first) so YEARFRAC always returns
        // a non-negative fraction no matter which argument the caller passes first --
        // Excel's own YEARFRAC behavior, and this pack's day_count_act_act/act_360/act_365
        // convention (magnitude-only, order-independent), unlike day_count_30_360_eu
        // (which preserves DAYS360's signed order, a different Excel function entirely).
        let earlier = s1 <= s2;
        let ay1 = if earlier { self.y_start } else { self.y_end };
        let am1 = if earlier { self.m_start } else { self.m_end };
        let ad1 = if earlier { self.d_start } else { self.d_end };
        let ay2 = if earlier { self.y_end } else { self.y_start };
        let am2 = if earlier { self.m_end } else { self.m_start };
        let ad2 = if earlier { self.d_end } else { self.d_start };
        let s_lo = if earlier { s1 } else { s2 };
        let s_hi = if earlier { s2 } else { s1 };

        if self.basis == 0u16 || self.basis == 4u16 {
            let mut da = ad1;
            let mut db = ad2;
            if self.basis == 4u16 {
                // European 30/360 (day_count_30_360_eu's own technique): day 31 on EITHER
                // date clamps to 30 independently of the other date, no conditional chain.
                if da == 31u16 { da = 30u16; }
                if db == 31u16 { db = 30u16; }
            } else {
                // US 30/360 (this project's day_count_30_360_us convention, preserved
                // inlined in excel_coupdaybs.rs's basis-0 branch): the end day only
                // clamps to 30 when the start day was ALSO 30 (either originally, or
                // just clamped down from 31 above).
                if da == 31u16 { da = 30u16; }
                if db == 31u16 && da == 30u16 { db = 30u16; }
            }
            let total_lo = (ay1 as u32) * 360u32 + (am1 as u32) * 30u32 + (da as u32);
            let total_hi = (ay2 as u32) * 360u32 + (am2 as u32) * 30u32 + (db as u32);
            // Non-negative by construction: chronological ordering above guarantees the
            // 30/360 pseudo-ordinal is monotonic non-decreasing with true calendar date
            // (day_count_30_360_eu's own limits comment makes the identical claim), so no
            // underflow is reachable here.
            let days30 = total_hi - total_lo;
            let days_f = int_to_f32(days30);
            self.year_frac = days_f / 360.0f32;
            return 1u16;
        }

        // Actual-day bases (1/2/3) all share the same real calendar-day numerator
        // (days_between's Rata-Die technique, already computed above as s_hi - s_lo);
        // only the denominator differs between them.
        let diff = s_hi - s_lo;
        if diff > 65535u32 {
            halt(0xFF05u16);
        }

        if self.basis == 1u16 {
            // Actual/Actual (day_count_act_act's own technique): denominator is 366 if
            // the span includes a real Feb 29, 365 otherwise -- scan every calendar year
            // the span touches.
            let mut includes_feb29 = 0u16;
            let mut y = ay1;
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
                if y == ay2 {
                    done = 1u16;
                } else {
                    y = y + 1u16;
                }
            }
            let denom = if includes_feb29 == 1u16 { 366.0f32 } else { 365.0f32 };
            let days_f = int_to_f32(diff);
            self.year_frac = days_f / denom;
            return 1u16;
        }

        let days_f = int_to_f32(diff);
        if self.basis == 2u16 {
            self.year_frac = days_f / 360.0f32;
        } else {
            self.year_frac = days_f / 365.0f32;
        }
        1u16
    }
}
