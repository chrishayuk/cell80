//! Absolute number of days between two Gregorian dates (order-independent), computed as a Rata-Die-style serial day number for each date subtracted from the other -- day_of_year only gives ordinal position within a single year, nothing else in the pack spans years.
//! tags: calendar, date, days-between, span, elapsed, duration, rata-die, julian-day, gregorian, wide, u32, checked, escalate
//! entry: DaysBetween::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the day difference exceeds u16::MAX (roughly 179 years apart)
fn serial_day(y: u16, m: u16, d: u16) -> u32 {
    let y32 = y as u32;
    let m32 = m as u32;
    let d32 = d as u32;
    let a = (14u32 - m32) / 12u32;
    let yy = y32 + 4800u32 - a;
    let mm = m32 + 12u32 * a - 3u32;
    d32 + (153u32 * mm + 2u32) / 5u32 + 365u32 * yy + yy / 4u32 - yy / 100u32 + yy / 400u32 - 32045u32
}

struct DaysBetween { y1: u16, m1: u16, d1: u16, y2: u16, m2: u16, d2: u16, days: u16 }
impl DaysBetween {
    fn run(&mut self) -> u16 {
        let s1 = serial_day(self.y1, self.m1, self.d1);
        let s2 = serial_day(self.y2, self.m2, self.d2);
        let diff = if s1 >= s2 { s1 - s2 } else { s2 - s1 };
        if diff > 65535u32 { halt(0xFF05u16); }
        let result = diff as u16;
        self.days = result;
        1u16
    }
}
