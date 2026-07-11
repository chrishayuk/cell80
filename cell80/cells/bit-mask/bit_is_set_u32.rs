//! Returns 1 if bit number `bit` (0-31) of a 32-bit value x is set, else 0: (x >> bit) & 1, widened from bit_is_set's 16-bit domain -- needs a u32 state field since bit_is_set's fn run(x: u16, bit: u16) cannot accept a 32-bit input under the 16-bit calling convention.
//! tags: bits, test, get, flag, is-set, check, wide, u32
//! entry: BitIsSetU32::run
struct BitIsSetU32 { x: u32, bit: u16, out: u16 }
impl BitIsSetU32 {
    fn run(&mut self) -> u16 {
        // u32 shifts only take a constant literal amount in this dialect, so a
        // variable bit index can't shift `x` directly. Split x into its low/high
        // 16-bit halves (each split off by a *constant* shift) and shift whichever
        // half holds the target bit by a runtime amount, which 16-bit shifts allow.
        let lo = self.x as u16;
        let hi = (self.x >> 16u32) as u16;
        let r = if self.bit < 16u16 {
            (lo >> self.bit) & 1u16
        } else {
            (hi >> (self.bit - 16u16)) & 1u16
        };
        self.out = r;
        r
    }
}
