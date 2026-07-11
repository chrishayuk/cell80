//! Round-to-nearest division of two u32 values (ties up): the wide, escalating sibling of round_div. Escalates (needs_wider_math) if b is zero.
//! tags: math, divide, round, round-to-nearest, ties-up, wide, u32
//! entry: RoundDivChecked::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if b == 0
struct RoundDivChecked { a: u32, b: u32, quotient: u32 }
impl RoundDivChecked {
    fn run(&mut self) -> u16 {
        if self.b == 0u32 { halt(0xFF05u16); }
        let q = self.a / self.b;
        let r = self.a % self.b;
        // Round up on a tie (r/b >= 1/2), tested as r >= b - r rather than 2*r >= b —
        // b - r never overflows (r < b), while 2*r can silently wrap near u32::MAX.
        let rounded = if r >= self.b - r { q + 1u32 } else { q };
        self.quotient = rounded;
        1u16
    }
}
