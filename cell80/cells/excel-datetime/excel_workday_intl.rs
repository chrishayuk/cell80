//! Excel WORKDAY.INTL(start_date, days, weekend): same signed day-by-day stepping as excel_workday (direction 0=forward, 1=backward, magnitude num_days: u16), except which stepped-to days count as workdays is decided by a caller-chosen 7-bit weekend_mask (bit i set means Zeller weekday code i -- day_of_week's own 0=Saturday..6=Friday numbering -- is a non-working day) instead of excel_workday's hardwired Monday-Friday check, and takes NO holidays argument at all, per the assignment brief, unlike full Excel WORKDAY.INTL -- the running Zeller code is seeded once from the start date and just rotated +1/-1 mod 7 per calendar-day step (excel_workday's own technique, reused verbatim) rather than ever recomputed from scratch, so the year-1 Zeller underflow risk only has to be guarded once, at the seed.
//! tags: excel, workday, workday-intl, date, calendar, weekend, weekend-mask, bitmask, custom-weekend, business-day, working-day, skip-weekend, day-of-week, zeller, step, stepping-loop, gregorian, datetime
//! entry: ExcelWorkdayIntl::run
//! limits: escalates (halt 0xFF06, out_of_domain) if month is outside 1-12, if weekend_mask is >127 (bits above bit 6 of the 7-bit representation are undefined), if year is 0 with month 1 or 2 at the seed date (the Zeller year-1 adjustment would underflow, the same limitation day_of_week.rs/excel_workday.rs document), if stepping backward would take the date before year 0, or if the bounded stepping loop's safety margin (7*num_days+16 calendar-day steps -- widened from excel_workday's 2*num_days+16 because a caller-chosen weekend_mask can leave as few as one working day per 7-day week, e.g. mask 0b1111110, instead of excel_workday's fixed five) is ever exceeded; escalates (halt 0xFF05, needs_wider_math) if stepping forward would take the year past 65535; like every day-by-day loop in this pack, cost scales with the magnitude of num_days, so very large day counts (or a sparse weekend_mask) cost proportionally more steps to execute.
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

struct ExcelWorkdayIntl {
    year: u16,
    month: u16,
    day: u16,
    num_days: u16,
    direction: u16,
    weekend_mask: u16,
    new_year: u16,
    new_month: u16,
    new_day: u16,
}
impl ExcelWorkdayIntl {
    fn run(&mut self) -> u16 {
        if self.month < 1u16 || self.month > 12u16 {
            halt(0xFF06u16);
        }
        if self.weekend_mask > 127u16 {
            halt(0xFF06u16);
        }

        // day_of_week's own Zeller's-congruence formula (cell80/cells/calendrical-checksum/day_of_week.rs),
        // inlined once to seed the running weekday code -- each further calendar-day step below
        // just rotates it by +1/-1 mod 7 instead of recomputing Zeller from scratch (excel_workday's
        // own technique, reused verbatim).
        let mut zm = self.month;
        let mut zy = self.year;
        if zm < 3u16 {
            if zy == 0u16 {
                halt(0xFF06u16);
            }
            zm = zm + 12u16;
            zy = zy - 1u16;
        }
        let k = zy % 100u16;
        let j = zy / 100u16;
        let term = (13u16 * (zm + 1u16)) / 5u16;
        let mut dow = (self.day + term + k + k / 4u16 + j / 4u16 + 5u16 * j) % 7u16;

        let mut y = self.year;
        let mut m = self.month;
        let mut d = self.day;

        let max_steps = (self.num_days as u32) * 7u32 + 16u32;
        let mut steps = 0u32;
        let mut counted = 0u16;

        while counted < self.num_days {
            if steps >= max_steps {
                halt(0xFF06u16);
            }

            if self.direction == 0u16 {
                let dim = days_in_month(y, m);
                if d < dim {
                    d = d + 1u16;
                } else {
                    d = 1u16;
                    if m < 12u16 {
                        m = m + 1u16;
                    } else {
                        if y == 65535u16 {
                            halt(0xFF05u16);
                        }
                        m = 1u16;
                        y = y + 1u16;
                    }
                }
                dow = (dow + 1u16) % 7u16;
            } else {
                if d > 1u16 {
                    d = d - 1u16;
                } else {
                    if m > 1u16 {
                        m = m - 1u16;
                    } else {
                        if y == 0u16 {
                            halt(0xFF06u16);
                        }
                        m = 12u16;
                        y = y - 1u16;
                    }
                    d = days_in_month(y, m);
                }
                dow = (dow + 6u16) % 7u16;
            }

            steps = steps + 1u32;

            let is_weekend_day = (self.weekend_mask >> dow) & 1u16;
            if is_weekend_day == 0u16 {
                counted = counted + 1u16;
            }
        }

        self.new_year = y;
        self.new_month = m;
        self.new_day = d;
        1u16
    }
}
