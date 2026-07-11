//! Add two packed 2-digit BCD bytes via per-nibble decimal-carry correction (the Z80 ADD+DAA idiom): each nibble sum over 9 is corrected by +6, producing the packed-BCD sum mod 100 plus a carry-out flag when the true decimal sum reaches or exceeds 100.
//! tags: bcd, decimal, add, arithmetic, pack, carry, binary-coded-decimal, daa, decimal-adjust
//! entry: BcdAdd::run
//! limits: a and b must each be a valid packed BCD byte (nibbles 0-9, i.e. 0x00-0x99)
struct BcdAdd { a: u16, b: u16, sum: u16, carry: u16 }
impl BcdAdd {
    fn run(&mut self) -> u16 {
        let a_lo = self.a & 0xFu16;
        let a_hi = (self.a >> 4u16) & 0xFu16;
        let b_lo = self.b & 0xFu16;
        let b_hi = (self.b >> 4u16) & 0xFu16;

        let lo_sum = a_lo + b_lo;
        let lo_digit = if lo_sum > 9u16 { (lo_sum + 6u16) & 0xFu16 } else { lo_sum };
        let nibble_carry = if lo_sum > 9u16 { 1u16 } else { 0u16 };

        let hi_sum = a_hi + b_hi + nibble_carry;
        let hi_digit = if hi_sum > 9u16 { (hi_sum + 6u16) & 0xFu16 } else { hi_sum };
        let carry_out = if hi_sum > 9u16 { 1u16 } else { 0u16 };

        let s = (hi_digit << 4u16) | lo_digit;
        self.sum = s;
        self.carry = carry_out;
        1u16
    }
}
