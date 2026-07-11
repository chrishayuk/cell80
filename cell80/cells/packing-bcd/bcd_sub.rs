//! Subtract two packed 2-digit BCD bytes (tens in the high nibble, units in the low nibble) via per-nibble decimal-borrow correction, producing the packed-BCD difference plus a borrow-out flag -- bcd_add's reverse-equation counterpart, the direction this pack was missing.
//! tags: bcd, decimal, subtract, borrow, binary-coded-decimal, packed, decimal-correction, arithmetic
//! entry: BcdSub::run
struct BcdSub { a: u16, b: u16, diff: u16, borrow: u16 }
impl BcdSub {
    fn run(&mut self) -> u16 {
        let lo_a = self.a & 0xFu16;
        let hi_a = (self.a >> 4u16) & 0xFu16;
        let lo_b = self.b & 0xFu16;
        let hi_b = (self.b >> 4u16) & 0xFu16;

        let lo_borrow = (lo_a < lo_b) as u16;
        let lo_diff = if lo_borrow == 1u16 { lo_a + 10u16 - lo_b } else { lo_a - lo_b };

        let hi_borrow = (hi_a < hi_b + lo_borrow) as u16;
        let hi_diff = if hi_borrow == 1u16 { hi_a + 10u16 - hi_b - lo_borrow } else { hi_a - hi_b - lo_borrow };

        self.diff = (hi_diff << 4u16) | lo_diff;
        self.borrow = hi_borrow;
        1u16
    }
}
