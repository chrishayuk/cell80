//! Number of days in the coupon period that contains the settlement date (Excel COUPDAYS(settlement, maturity, frequency, [basis]), all four arguments required with basis defaulting to 0/US-30-360 in real Excel when omitted -- here basis must be passed explicitly since cell80 has no optional-field mechanism): for basis 0 (US 30/360), 2 (Actual/360), 4 (European 30/360) returns the fixed 360/frequency, for basis 3 (Actual/365) returns the fixed 365/frequency, and only for basis 1 (Actual/actual) walks backward from maturity in period-months jumps (the DateAddMonths stepping technique, inlined since cells can't call each other) to bracket settlement between its previous coupon date (PCD) and next coupon date (NCD) and returns their real calendar-day gap -- distinct from COUPDAYBS (days from PCD to settlement, only the elapsed half) and COUPDAYSNC (days from settlement to NCD, only the remaining half), and from the day-count-fraction prerequisite cells (day_count_act_act etc., which turn a fixed pair of dates into a year-fraction rather than search for the bracketing coupon dates themselves); no outflow-negative sign convention and no annuity type flag apply to a day count.
//! tags: excel, coupdays, coupon, bond, day-count, basis, 30-360, actual-actual, act-act, pcd, ncd, date-stepping, frequency, f32
//! entry: ExcelCoupdays::run
//! limits: escalates (halt 0xFF06, out_of_domain) if settle_m/mat_m is outside 1-12, frequency is not one of 1/2/4, basis is greater than 4, settlement's serial day is not strictly before maturity's, or the basis-1 backward coupon-date search exceeds 2400 period jumps (a bond term far beyond any realistic schedule); never escalates 0xFF05 (all arithmetic stays comfortably within u32 headroom for realistic bond terms)

// serial_day duplicated inline from calendrical-checksum/days_between.rs's own helper --
// cells compile standalone against the shared kernel prelude only, so cross-cell logic
// is duplicated rather than called (the same convention day-count/date_add_months.rs
// follows for is_leap_year/days_in_month).
fn serial_day(y: u16, m: u16, d: u16) -> u32 {
    let y32 = y as u32;
    let m32 = m as u32;
    let d32 = d as u32;
    let a = (14u32 - m32) / 12u32;
    let yy = y32 + 4800u32 - a;
    let mm = m32 + 12u32 * a - 3u32;
    d32 + (153u32 * mm + 2u32) / 5u32 + 365u32 * yy + yy / 4u32 - yy / 100u32 + yy / 400u32 - 32045u32
}

struct ExcelCoupdays {
    settle_y: u16,
    settle_m: u16,
    settle_d: u16,
    mat_y: u16,
    mat_m: u16,
    mat_d: u16,
    frequency: u16,
    basis: u16,
    days: f32,
}
impl ExcelCoupdays {
    fn run(&mut self) -> u16 {
        if self.settle_m < 1u16 || self.settle_m > 12u16 || self.mat_m < 1u16 || self.mat_m > 12u16 {
            halt(0xFF06u16);
        }
        if self.frequency != 1u16 && self.frequency != 2u16 && self.frequency != 4u16 {
            halt(0xFF06u16);
        }
        if self.basis > 4u16 {
            halt(0xFF06u16);
        }

        let settle_serial = serial_day(self.settle_y, self.settle_m, self.settle_d);
        let mat_serial = serial_day(self.mat_y, self.mat_m, self.mat_d);
        if settle_serial >= mat_serial {
            halt(0xFF06u16);
        }

        let period_months = 12u16 / self.frequency;
        let mut days_result = 0.0f32;

        if self.basis == 1u16 {
            // Actual/actual: bracket settlement between the previous coupon date
            // (PCD) and next coupon date (NCD) by jumping backward from maturity
            // in whole multiples of period_months, re-clamping to the ORIGINAL
            // maturity day-of-month each time (exactly what a fresh date_add_months
            // call with months = k*period_months, direction = backward would do --
            // never chained off an already-clamped day, so a day-31 maturity stays
            // anchored to 31 every jump instead of drifting down after one clamp).
            let mat_idx = (self.mat_y as u32) * 12u32 + (self.mat_m as u32 - 1u32);
            let mut k = 0u16;
            let mut done = 0u16;
            let mut pcd_y = self.mat_y;
            let mut pcd_m = self.mat_m;
            let mut pcd_d = self.mat_d;
            let mut ncd_y = self.mat_y;
            let mut ncd_m = self.mat_m;
            let mut ncd_d = self.mat_d;
            while done == 0u16 {
                let step = (k as u32) * (period_months as u32);
                if step > mat_idx {
                    halt(0xFF06u16);
                }
                let cand_idx = mat_idx - step;
                let cand_year32 = cand_idx / 12u32;
                let cand_month0 = cand_idx % 12u32;
                let cand_year = cand_year32 as u16;
                let cand_month = (cand_month0 + 1u32) as u16;

                let by4 = cand_year % 4u16 == 0u16;
                let by100 = cand_year % 100u16 == 0u16;
                let by400 = cand_year % 400u16 == 0u16;
                let is_leap = (by4 && (!by100 || by400)) as u16;
                let base = match cand_month {
                    1u16 => 31u16, 2u16 => 28u16, 3u16 => 31u16, 4u16 => 30u16,
                    5u16 => 31u16, 6u16 => 30u16, 7u16 => 31u16, 8u16 => 31u16,
                    9u16 => 30u16, 10u16 => 31u16, 11u16 => 30u16, 12u16 => 31u16,
                    _ => 0u16,
                };
                let max_day = if cand_month == 2u16 && is_leap != 0u16 { 29u16 } else { base };
                let clamped_day = if self.mat_d > max_day { max_day } else { self.mat_d };

                let cand_serial = serial_day(cand_year, cand_month, clamped_day);
                if cand_serial <= settle_serial {
                    pcd_y = cand_year;
                    pcd_m = cand_month;
                    pcd_d = clamped_day;
                    done = 1u16;
                } else {
                    ncd_y = cand_year;
                    ncd_m = cand_month;
                    ncd_d = clamped_day;
                    k = k + 1u16;
                    if k > 2400u16 {
                        halt(0xFF06u16);
                    }
                }
            }
            let pcd_serial = serial_day(pcd_y, pcd_m, pcd_d);
            let ncd_serial = serial_day(ncd_y, ncd_m, ncd_d);
            let diff = ncd_serial - pcd_serial;
            days_result = int_to_f32(diff);
        } else if self.basis == 3u16 {
            days_result = 365.0f32 / int_to_f32(self.frequency);
        } else {
            days_result = 360.0f32 / int_to_f32(self.frequency);
        }

        self.days = days_result;
        1u16
    }
}
