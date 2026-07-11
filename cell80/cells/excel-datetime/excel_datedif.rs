//! Excel DATEDIF(start_date, end_date, unit): the calendar-aware WHOLE-unit difference between two Gregorian dates, where unit is a small integer code standing in for Excel's string argument (0=Y whole years, 1=M whole months, 2=D whole days, 3=MD days ignoring months and years, 4=YM months ignoring years, 5=YD days ignoring years) -- Y/M share one day-of-month comparison to decide whether the last partial period counts, and MD/YD are each derived by stepping start_date forward by the M-unit's or Y-unit's own whole-count (date_add_months/excel_edate's month-stepping-with-end-of-month-clamp technique, inlined) and taking the remaining day gap to end_date, so this is genuinely new calendar-aware differencing logic, not just a day-count span like days_between/excel_days (always one raw elapsed-day number, no unit selector at all) or a year-FRACTION like excel_yearfrac/day_count_* (a single ratio for accrual, never a whole integer count in a caller-chosen calendar unit).
//! tags: excel, datedif, date, calendar, difference, elapsed, age, tenure, duration, unit-code, whole-years, whole-months, whole-days, year, month, day, md, ym, yd, gregorian, leap-year, end-of-month, clamp, escalate, datetime
//! entry: ExcelDatedif::run
//! limits: escalates (halt 0xFF06, out_of_domain) if start month/end month is outside 1-12, if unit is anything other than 0-5, or if end_date's serial day is before start_date's (Excel's own DATEDIF requires end_date >= start_date, otherwise #NUM!); escalates (halt 0xFF05, needs_wider_math) if the M-unit's total whole-month count or the D-unit's total whole-day count exceeds u16::MAX (roughly 5461 years or 179 years apart respectively); does not itself validate that day is a genuine day-of-month beyond the 1-12 month check (garbage in, garbage out, matching days_between/day_count_act_act's own convention) -- MD and YD are the least-confident units here: Excel's own real DATEDIF is documented as buggy right at day-31/Feb-29 month-end boundaries, and this cell resolves that ambiguity by always stepping via the SAME clamp-on-jump rule date_add_months uses rather than a raw day-of-month subtraction, so its MD/YD output can differ from real Excel's in those specific boundary cases (see notes).
fn serial_day(y: u16, m: u16, d: u16) -> u32 {
    let y32 = y as u32;
    let m32 = m as u32;
    let d32 = d as u32;
    let a = (14u32 - m32) / 12u32;
    let yy = y32 + 4800u32 - a;
    let mm = m32 + 12u32 * a - 3u32;
    d32 + (153u32 * mm + 2u32) / 5u32 + 365u32 * yy + yy / 4u32 - yy / 100u32 + yy / 400u32 - 32045u32
}

// date_add_months/excel_edate's own month-stepping-with-clamp technique, inlined directly
// at each of the two call sites below (MD and YD) rather than as a shared helper function:
// a wide (u32) parameter may only appear ALONE at a function-call boundary (Tier 2's "at
// most one u32 param per call, and nothing more" -- docs/library-growth.md), and this
// stepping needs y/m/d alongside the u32 months-forward count, so it can't be a 4-parameter
// free fn. A `u32` *local variable*, entirely inside `run`'s own body, is never gated by
// that call-boundary limit (only actual function calls passing u32 arguments are), so this
// duplicates the ~15-line stepping arithmetic twice instead of sharing one function.
struct ExcelDatedif {
    y_start: u16,
    m_start: u16,
    d_start: u16,
    y_end: u16,
    m_end: u16,
    d_end: u16,
    unit: u16,
    result: u16,
}
impl ExcelDatedif {
    fn run(&mut self) -> u16 {
        if self.m_start < 1u16 || self.m_start > 12u16 || self.m_end < 1u16 || self.m_end > 12u16 {
            halt(0xFF06u16);
        }
        if self.unit > 5u16 {
            halt(0xFF06u16);
        }

        let s1 = serial_day(self.y_start, self.m_start, self.d_start);
        let s2 = serial_day(self.y_end, self.m_end, self.d_end);
        if s1 > s2 {
            halt(0xFF06u16);
        }

        // Y: whole years, decremented by one unless end_date's (month, day) has already
        // reached start_date's (month, day) within the year -- the same "has this year's
        // anniversary happened yet" comparison a birthday/tenure calculation makes.
        let anniversary_reached = (self.m_end > self.m_start)
            || (self.m_end == self.m_start && self.d_end >= self.d_start);
        let y_dec = (!anniversary_reached) as u16;
        let y_whole = (self.y_end - self.y_start) - y_dec;

        // M: whole months across the entire span, decremented by one unless end_date's
        // day-of-month has already reached start_date's day-of-month -- computed as
        // (whole years * 12 + month delta) first, THEN the day-of-month is subtracted last,
        // so the intermediate never goes negative even when m_end < m_start.
        let m_dec = (self.d_end < self.d_start) as u32;
        let y_span = (self.y_end - self.y_start) as u32;
        let m_raw = y_span * 12u32 + (self.m_end as u32);
        let m_whole32 = (m_raw - (self.m_start as u32)) - m_dec;

        let mut result = 0u16;

        if self.unit == 0u16 {
            // Y
            result = y_whole;
        } else if self.unit == 1u16 {
            // M: total whole months, spanning every year in between
            if m_whole32 > 65535u32 {
                halt(0xFF05u16);
            }
            result = m_whole32 as u16;
        } else if self.unit == 2u16 {
            // D: total whole days, ignoring months/years entirely
            let d_total = s2 - s1;
            if d_total > 65535u32 {
                halt(0xFF05u16);
            }
            result = d_total as u16;
        } else if self.unit == 3u16 {
            // MD: days ignoring months and years -- step start_date forward by the
            // M-unit's own whole-month count (always lands on or before end_date, by
            // construction: see the module-level limits comment), then the remaining
            // day gap to end_date is the MD answer. (Inlined stepping -- see the
            // module-level comment on why this can't be a shared 4-param helper fn.)
            let idx = (self.y_start as u32) * 12u32 + (self.m_start as u32 - 1u32) + m_whole32;
            let new_year32 = idx / 12u32;
            if new_year32 > 65535u32 {
                halt(0xFF05u16);
            }
            let new_month0 = idx % 12u32;
            let new_year = new_year32 as u16;
            let new_month = (new_month0 + 1u32) as u16;

            let by4 = new_year % 4u16 == 0u16;
            let by100 = new_year % 100u16 == 0u16;
            let by400 = new_year % 400u16 == 0u16;
            let is_leap = (by4 && (!by100 || by400)) as u16;

            let base = match new_month {
                1u16 => 31u16, 2u16 => 28u16, 3u16 => 31u16, 4u16 => 30u16,
                5u16 => 31u16, 6u16 => 30u16, 7u16 => 31u16, 8u16 => 31u16,
                9u16 => 30u16, 10u16 => 31u16, 11u16 => 30u16, 12u16 => 31u16,
                _ => 0u16,
            };
            let max_day = if new_month == 2u16 && is_leap != 0u16 { 29u16 } else { base };
            let clamped_day = if self.d_start > max_day { max_day } else { self.d_start };

            let stepped = serial_day(new_year, new_month, clamped_day);
            let md32 = s2 - stepped;
            result = md32 as u16;
        } else if self.unit == 4u16 {
            // YM: whole months ignoring years -- M's total minus the whole years already
            // counted, always landing in 0..=11 by construction (Y and M share the same
            // day-of-month decrement logic above).
            let ym32 = m_whole32 - (y_whole as u32) * 12u32;
            result = ym32 as u16;
        } else {
            // unit == 5, YD: days ignoring years -- step start_date forward by the Y-unit's
            // own whole-year count expressed as whole months (Y*12), then the remaining day
            // gap to end_date is the YD answer. (Inlined stepping, same reason as MD above.)
            let years_fwd = (y_whole as u32) * 12u32;
            let idx = (self.y_start as u32) * 12u32 + (self.m_start as u32 - 1u32) + years_fwd;
            let new_year32 = idx / 12u32;
            if new_year32 > 65535u32 {
                halt(0xFF05u16);
            }
            let new_month0 = idx % 12u32;
            let new_year = new_year32 as u16;
            let new_month = (new_month0 + 1u32) as u16;

            let by4 = new_year % 4u16 == 0u16;
            let by100 = new_year % 100u16 == 0u16;
            let by400 = new_year % 400u16 == 0u16;
            let is_leap = (by4 && (!by100 || by400)) as u16;

            let base = match new_month {
                1u16 => 31u16, 2u16 => 28u16, 3u16 => 31u16, 4u16 => 30u16,
                5u16 => 31u16, 6u16 => 30u16, 7u16 => 31u16, 8u16 => 31u16,
                9u16 => 30u16, 10u16 => 31u16, 11u16 => 30u16, 12u16 => 31u16,
                _ => 0u16,
            };
            let max_day = if new_month == 2u16 && is_leap != 0u16 { 29u16 } else { base };
            let clamped_day = if self.d_start > max_day { max_day } else { self.d_start };

            let stepped = serial_day(new_year, new_month, clamped_day);
            let yd32 = s2 - stepped;
            result = yd32 as u16;
        }

        self.result = result;
        1u16
    }
}
