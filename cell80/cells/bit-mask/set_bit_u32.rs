//! Set bit number `bit` (0-31) of a 32-bit value x to 1, widened from set_bit's 16-bit domain via a hi/lo u16-half split with constant-shift OR -- needs a u32 state field since set_bit's fn run(x: u16, bit: u16) cannot accept a 32-bit input under the 16-bit calling convention.
//! tags: bits, set, enable, flag, or, on, wide, u32
//! entry: SetBitU32::run
struct SetBitU32 { x: u32, bit: u16, out: u32 }
impl SetBitU32 {
    fn run(&mut self) -> u16 {
        // u32 shifts only take a constant literal amount in this dialect, so a
        // variable bit index can't shift `x` directly. Split x into its low/high
        // 16-bit halves (each split off by a *constant* shift), OR the target bit
        // into whichever half holds it using a runtime shift (which 16-bit shifts
        // allow), then recombine the halves back into the u32 result.
        let lo = self.x as u16;
        let hi = (self.x >> 16u32) as u16;
        let new_lo = if self.bit < 16u16 { lo | (1u16 << self.bit) } else { lo };
        let new_hi = if self.bit < 16u16 { hi } else { hi | (1u16 << (self.bit - 16u16)) };
        self.out = (new_hi as u32) << 16u32 | (new_lo as u32);
        1u16
    }
}
