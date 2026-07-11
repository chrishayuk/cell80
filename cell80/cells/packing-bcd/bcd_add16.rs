//! Add two packed 4-digit BCD u16 values via a 4-nibble decimal-carry chain (bcd_add's per-nibble ADD+DAA idiom run across all four digit positions instead of two): each nibble sum over 9 is corrected by +6 and its carry ripples into the next nibble, producing the packed-BCD sum mod 10000 plus a carry-out flag when the true decimal sum reaches or exceeds 10000.
//! tags: bcd, decimal, add, arithmetic, pack, carry, binary-coded-decimal, daa, decimal-adjust, wide, nibble, four-digit
//! entry: BcdAdd16::run
//! limits: a and b must each be a valid packed BCD u16 (all four nibbles 0-9, i.e. 0x0000-0x9999)
struct BcdAdd16 { a: u16, b: u16, sum: u16, carry: u16 }
impl BcdAdd16 {
    fn run(&mut self) -> u16 {
        let a0 = self.a & 0xFu16;
        let a1 = (self.a >> 4u16) & 0xFu16;
        let a2 = (self.a >> 8u16) & 0xFu16;
        let a3 = (self.a >> 12u16) & 0xFu16;
        let b0 = self.b & 0xFu16;
        let b1 = (self.b >> 4u16) & 0xFu16;
        let b2 = (self.b >> 8u16) & 0xFu16;
        let b3 = (self.b >> 12u16) & 0xFu16;

        let s0 = a0 + b0;
        let d0 = if s0 > 9u16 { (s0 + 6u16) & 0xFu16 } else { s0 };
        let c0 = if s0 > 9u16 { 1u16 } else { 0u16 };

        let s1 = a1 + b1 + c0;
        let d1 = if s1 > 9u16 { (s1 + 6u16) & 0xFu16 } else { s1 };
        let c1 = if s1 > 9u16 { 1u16 } else { 0u16 };

        let s2 = a2 + b2 + c1;
        let d2 = if s2 > 9u16 { (s2 + 6u16) & 0xFu16 } else { s2 };
        let c2 = if s2 > 9u16 { 1u16 } else { 0u16 };

        let s3 = a3 + b3 + c2;
        let d3 = if s3 > 9u16 { (s3 + 6u16) & 0xFu16 } else { s3 };
        let carry_out = if s3 > 9u16 { 1u16 } else { 0u16 };

        let s = (d3 << 12u16) | (d2 << 8u16) | (d1 << 4u16) | d0;
        self.sum = s;
        self.carry = carry_out;
        1u16
    }
}
