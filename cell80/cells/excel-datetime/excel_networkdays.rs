//! Excel NETWORKDAYS(start_date, end_date): count of whole workdays (Monday-Friday) between the two dates inclusive, no holidays argument (Excel's optional holidays array is a separate, already-known variable-length-array gap, not covered here) -- walks every calendar day from the earlier date to the later one in a bounded day-by-day loop, re-deriving day_of_week's own Zeller's-congruence formula and is_weekday's Monday-Friday classification fresh from scratch at each step (unlike excel_workday's running dow rotated +1/-1 mod 7 per step, since here every visited day must independently confirm its own classification rather than accumulate toward a single target), and reports the count sign-magnitude since a start_date later than end_date reports the same workday count negated (i16 isn't a usable state field) -- distinct from days_between/excel_days (span every calendar day, no weekday filter at all), from is_weekday/day_of_week (classify one already-known date, never walk a range), and from excel_workday (derives a brand-new date N workdays away from a single start_date, rather than counting the workdays already spanned by two given dates).
//! tags: excel, networkdays, workdays, workday-count, business-days, business-day-count, mon-fri, weekday-count, date-range, span, count-between-dates, loop, zeller, day-of-week, calendar, gregorian, datetime, sign-magnitude, wide, u32, checked, escalate
//! entry: ExcelNetworkdays::run
//! limits: no holidays argument (Excel's optional holidays array is a separate, already-known gap, skipped here); escalates (halt 0xFF06, out_of_domain) if either month is outside 1-12, or if the Zeller adjustment ever steps a year below 1 (the same restriction day_of_week.rs/excel_weekday.rs document); escalates (halt 0xFF05, needs_wider_math) if the inclusive day span would exceed u16::MAX -- one day tighter than days_between/excel_days's own plain-diff limit, since NETWORKDAYS counts the span inclusively (diff + 1, not diff); like every day-by-day loop in this pack, cost scales with the size of the date span.
fn serial_day(y: u16, m: u16, d: u16) -> u32 {
    let y32 = y as u32;
    let m32 = m as u32;
    let d32 = d as u32;
    let a = (14u32 - m32) / 12u32;
    let yy = y32 + 4800u32 - a;
    let mm = m32 + 12u32 * a - 3u32;
    d32 + (153u32 * mm + 2u32) / 5u32 + 365u32 * yy + yy / 4u32 - yy / 100u32 + yy / 400u32 - 32045u32
}

fn days_in_month(y: u16, m: u16) -> u16 {
    let by4 = y % 4u16 == 0u16;
    let by100 = y % 100u16 == 0u16;
    let by400 = y % 400u16 == 0u16;
    let leap = by4 && (!by100 || by400);
    let base = match m {
        1u16 => 31u16, 2u16 => 28u16, 3u16 => 31u16, 4u16 => 30u16,
        5u16 => 31u16, 6u16 => 30u16, 7u16 => 31u16, 8u16 => 31u16,
        9u16 => 30u16, 10u16 => 31u16, 11u16 => 30u16, 12u16 => 31u16,
        _ => 0u16,
    };
    if m == 2u16 && leap { 29u16 } else { base }
}

struct ExcelNetworkdays {
    y_start: u16,
    m_start: u16,
    d_start: u16,
    y_end: u16,
    m_end: u16,
    d_end: u16,
    workdays_mag: u16,
    workdays_neg: u16,
}
impl ExcelNetworkdays {
    fn run(&mut self) -> u16 {
        if self.m_start < 1u16 || self.m_start > 12u16 || self.m_end < 1u16 || self.m_end > 12u16 {
            halt(0xFF06u16);
        }

        let s_start = serial_day(self.y_start, self.m_start, self.d_start);
        let s_end = serial_day(self.y_end, self.m_end, self.d_end);

        let neg = if s_start <= s_end { 0u16 } else { 1u16 };
        let diff = if s_end >= s_start { s_end - s_start } else { s_start - s_end };
        if diff > 65534u32 {
            halt(0xFF05u16);
        }

        // Walk forward from whichever date is earlier; the other date's fields were only
        // needed above, to order the pair and size the span.
        let mut cy = if neg == 0u16 { self.y_start } else { self.y_end };
        let mut cm = if neg == 0u16 { self.m_start } else { self.m_end };
        let mut cd = if neg == 0u16 { self.d_start } else { self.d_end };

        let total = (diff as u16) + 1u16;
        let mut count = 0u16;
        let mut i = 0u16;
        while i < total {
            // day_of_week's own Zeller's-congruence formula (cell80/cells/calendrical-checksum/day_of_week.rs),
            // re-derived fresh from the current walking date every iteration (against local copies,
            // so the month/year adjustment below never disturbs cy/cm/cd itself) -- unlike
            // excel_workday's running dow rotated +1/-1 mod 7 per step, this cell independently
            // reclassifies each visited day rather than carrying weekday state between them.
            let mut zm = cm;
            let mut zy = cy;
            if zm < 3u16 {
                if zy == 0u16 {
                    halt(0xFF06u16);
                }
                zy = zy - 1u16;
                zm = zm + 12u16;
            }
            let k = zy % 100u16;
            let j = zy / 100u16;
            let term = (13u16 * (zm + 1u16)) / 5u16;
            let dow = (cd + term + k + k / 4u16 + j / 4u16 + 5u16 * j) % 7u16;

            // is_weekday's own predicate (cell80/cells/calendrical-checksum/is_weekday.rs), inlined:
            // dow 2..6 is Monday..Friday under day_of_week's 0=Saturday..6=Friday code.
            let is_wd = (dow >= 2u16 && dow <= 6u16) as u16;
            if is_wd == 1u16 {
                count = count + 1u16;
            }

            // Step the walking date forward by one calendar day (days_in_month's own table and
            // leap-year check, inlined as a local helper the same way excel_workday.rs does).
            let dim = days_in_month(cy, cm);
            if cd < dim {
                cd = cd + 1u16;
            } else {
                cd = 1u16;
                if cm < 12u16 {
                    cm = cm + 1u16;
                } else {
                    cm = 1u16;
                    cy = cy + 1u16;
                }
            }

            i = i + 1u16;
        }

        self.workdays_mag = count;
        self.workdays_neg = neg;
        1u16
    }
}
