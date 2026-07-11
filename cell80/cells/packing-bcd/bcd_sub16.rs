//! Subtracts two packed 4-digit BCD u16 values (one decimal digit per nibble) via a 4-nibble decimal-borrow chain, producing the packed-BCD difference mod 10000 plus a borrow-out flag -- the 4-nibble width extension of bcd_sub's 2-nibble form, completing this pack's 2-digit/4-digit x add/subtract grid.
//! tags: bcd, decimal, subtract, borrow, binary-coded-decimal, packed, decimal-correction, arithmetic, u16, four-digit, wide
//! entry: BcdSub16::run
struct BcdSub16 { a: u16, b: u16, diff: u16, borrow: u16 }
impl BcdSub16 {
    fn run(&mut self) -> u16 {
        let d0_a = self.a & 0xFu16;
        let d1_a = (self.a >> 4u16) & 0xFu16;
        let d2_a = (self.a >> 8u16) & 0xFu16;
        let d3_a = (self.a >> 12u16) & 0xFu16;
        let d0_b = self.b & 0xFu16;
        let d1_b = (self.b >> 4u16) & 0xFu16;
        let d2_b = (self.b >> 8u16) & 0xFu16;
        let d3_b = (self.b >> 12u16) & 0xFu16;

        let b0 = (d0_a < d0_b) as u16;
        let r0 = if b0 == 1u16 { d0_a + 10u16 - d0_b } else { d0_a - d0_b };

        let b1 = (d1_a < d1_b + b0) as u16;
        let r1 = if b1 == 1u16 { d1_a + 10u16 - d1_b - b0 } else { d1_a - d1_b - b0 };

        let b2 = (d2_a < d2_b + b1) as u16;
        let r2 = if b2 == 1u16 { d2_a + 10u16 - d2_b - b1 } else { d2_a - d2_b - b1 };

        let b3 = (d3_a < d3_b + b2) as u16;
        let r3 = if b3 == 1u16 { d3_a + 10u16 - d3_b - b2 } else { d3_a - d3_b - b2 };

        self.diff = (r3 << 12u16) | (r2 << 8u16) | (r1 << 4u16) | r0;
        self.borrow = b3;
        1u16
    }
}
