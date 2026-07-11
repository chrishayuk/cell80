//! Excel WORKDAY(start_date, days): the date landing a signed whole number of WEEKDAYS-ONLY before or after start_date (direction 0=forward for positive days, 1=backward for negative days, magnitude days: u16) -- no holidays argument at all (that is WORKDAY.INTL/NETWORKDAYS territory, unbuilt here) -- found by a bounded day-by-day stepping loop that walks the real Gregorian calendar one day at a time (day-of-month rollover and the leap-year check inlined the same way date_add_months/excel_edate do), carrying a running Zeller's-congruence weekday code (day_of_week's own formula, inlined once to seed it, then just rotated +1/-1 mod 7 per calendar day instead of recomputed from scratch each step) and only advancing the workday counter when is_weekday's own check (dow in 2..6, Monday-Friday) passes on the newly-stepped-to day; the start_date itself is never counted or required to be a weekday, only the days actually stepped past it -- distinct from excel_edate (steps by whole MONTHS with an end-of-month day clamp, no weekday-skipping at all) and days_between/excel_days (a plain elapsed-day span between two already-known dates, never walks day-by-day to derive a brand-new one).
//! tags: excel, workday, date, calendar, weekday, weekend, business-day, working-day, skip-weekend, day-of-week, zeller, step, stepping-loop, gregorian, datetime
//! entry: ExcelWorkday::run
//! limits: escalates (halt 0xFF06, out_of_domain) if month is outside 1-12, if year is 0 with month 1 or 2 (the Zeller year-1 adjustment would underflow, the same limitation day_of_week.rs/excel_weekday.rs document), if stepping backward would take the date before year 0, or if the bounded stepping loop's generous safety margin (2*days+16 calendar-day steps) is ever exceeded (should be unreachable, since at most 7 calendar days are ever needed to bank 5 workdays); escalates (halt 0xFF05, needs_wider_math) if stepping forward would take the year past 65535; like every day-by-day loop in this pack, cost scales with the magnitude of days, so very large day counts cost proportionally more steps to execute.
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

struct ExcelWorkday {
    year: u16,
    month: u16,
    day: u16,
    num_days: u16,
    direction: u16,
    new_year: u16,
    new_month: u16,
    new_day: u16,
}
impl ExcelWorkday {
    fn run(&mut self) -> u16 {
        if self.month < 1u16 || self.month > 12u16 {
            halt(0xFF06u16);
        }

        // day_of_week's own Zeller's-congruence formula (cell80/cells/calendrical-checksum/day_of_week.rs),
        // inlined once to seed the running weekday code -- each further calendar-day step below
        // just rotates it by +1/-1 mod 7 instead of recomputing Zeller from scratch.
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

        let max_steps = (self.num_days as u32) * 2u32 + 16u32;
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

            let is_wd = (dow >= 2u16 && dow <= 6u16) as u16;
            if is_wd == 1u16 {
                counted = counted + 1u16;
            }
        }

        self.new_year = y;
        self.new_month = m;
        self.new_day = d;
        1u16
    }
}
