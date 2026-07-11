//! Excel WEEKDAY(serial_date, [return_type]): day of the week for a Gregorian date as a number in whichever numbering convention the caller selects (return_type 1, the default = Sunday..Saturday as 1..7; 2 = Monday..Sunday as 1..7; 3 = Monday..Sunday as 0..6) -- day_of_week's own Zeller's-congruence formula (cell80/cells/calendrical-checksum/day_of_week.rs, which returns its OWN fixed 0=Saturday..6=Friday code) is inlined to get the raw weekday, then rotated into the requested convention; getting that final remap exactly right, not the Zeller arithmetic itself (already proven by the landed day_of_week cell), is the entire point of this cell, distinct from day_of_week (fixed single convention, no return_type at all) and from is_weekday/is_weekend (consume day_of_week's convention directly, never remap it).
//! tags: excel, weekday, day-of-week, return-type, numbering-convention, convention, remap, rotate, zeller, calendar, date, gregorian, datetime
//! entry: ExcelWeekday::run
//! limits: escalates (halt 0xFF06, out_of_domain) if month is outside 1-12, if return_type is not 1, 2, or 3, or if year is 0 with month 1 or 2 (the Zeller year-1 adjustment would underflow, the same limitation day_of_week.rs documents)
struct ExcelWeekday {
    year: u16,
    month: u16,
    day: u16,
    return_type: u16,
    weekday: u16,
}
impl ExcelWeekday {
    fn run(&mut self) -> u16 {
        if self.month < 1u16 || self.month > 12u16 {
            halt(0xFF06u16);
        }
        if self.return_type != 1u16 && self.return_type != 2u16 && self.return_type != 3u16 {
            halt(0xFF06u16);
        }

        // day_of_week's own Zeller's-congruence formula (cell80/cells/calendrical-checksum/day_of_week.rs),
        // inlined -- each cell compiles standalone against the shared kernel prelude only,
        // so cross-cell logic is duplicated rather than called.
        let mut m = self.month;
        let mut y = self.year;
        if m < 3u16 {
            if y == 0u16 {
                halt(0xFF06u16);
            }
            m = m + 12u16;
            y = y - 1u16;
        }
        let k = y % 100u16;
        let j = y / 100u16;
        let term = (13u16 * (m + 1u16)) / 5u16;
        let dow = (self.day + term + k + k / 4u16 + j / 4u16 + 5u16 * j) % 7u16;

        // dow is day_of_week's own fixed convention: 0=Saturday, 1=Sunday, 2=Monday,
        // 3=Tuesday, 4=Wednesday, 5=Thursday, 6=Friday. Rotate it onto a Sunday=0..Saturday=6
        // pivot first -- the common starting point every return_type below is derived from.
        let from_sunday = (dow + 6u16) % 7u16;

        let result = match self.return_type {
            1u16 => from_sunday + 1u16,
            2u16 => (from_sunday + 6u16) % 7u16 + 1u16,
            3u16 => (from_sunday + 6u16) % 7u16,
            _ => 0u16,
        };

        self.weekday = result;
        1u16
    }
}
