//! Excel DAYS(end_date, start_date): signed number of days from start_date to end_date, computed as end_date's Rata-Die-style serial day number minus start_date's and returned sign-magnitude (i16 isn't a usable state field) -- distinct from days_between (cell80/cells/calendrical-checksum/days_between.rs), which throws the sign away and always returns the unsigned, order-independent absolute span, so a caller who needs to know WHICH date came first, not just how far apart the two are, needs this cell instead.
//! tags: excel, days, date, date-span, signed, sign-magnitude, negative, elapsed, subtract, rata-die, julian-day, gregorian, datetime, wide, u32, checked, escalate
//! entry: ExcelDays::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the day difference exceeds u16::MAX (roughly 179 years apart), matching days_between's own limit
fn serial_day(y: u16, m: u16, d: u16) -> u32 {
    let y32 = y as u32;
    let m32 = m as u32;
    let d32 = d as u32;
    let a = (14u32 - m32) / 12u32;
    let yy = y32 + 4800u32 - a;
    let mm = m32 + 12u32 * a - 3u32;
    d32 + (153u32 * mm + 2u32) / 5u32 + 365u32 * yy + yy / 4u32 - yy / 100u32 + yy / 400u32 - 32045u32
}

struct ExcelDays {
    y_end: u16,
    m_end: u16,
    d_end: u16,
    y_start: u16,
    m_start: u16,
    d_start: u16,
    days_mag: u16,
    days_neg: u16,
}
impl ExcelDays {
    fn run(&mut self) -> u16 {
        let s_end = serial_day(self.y_end, self.m_end, self.d_end);
        let s_start = serial_day(self.y_start, self.m_start, self.d_start);
        let diff = if s_end >= s_start { s_end - s_start } else { s_start - s_end };
        let neg = if s_end >= s_start { 0u16 } else { 1u16 };
        if diff > 65535u32 {
            halt(0xFF05u16);
        }
        self.days_mag = diff as u16;
        self.days_neg = neg;
        1u16
    }
}
