//! Clear bit number `bit` (0-31) of a 32-bit value x to 0, the u32-width sibling of clear_bit -- needs a u32 state field since clear_bit's fn run(x: u16, bit: u16) cannot accept a 32-bit input under the 16-bit calling convention; splits x into hi/lo 16-bit halves (each split off by a constant shift) and clears within whichever half holds the target bit using a runtime-indexed 16-bit shift, the same technique bit_is_set_u32 uses to read a bit.
//! tags: bits, clear, unset, disable, flag, off, wide, u32
//! entry: ClearBitU32::run
struct ClearBitU32 { x: u32, bit: u16, out: u32 }
impl ClearBitU32 {
    fn run(&mut self) -> u16 {
        let lo = self.x as u16;
        let hi = (self.x >> 16u32) as u16;
        let new_lo = if self.bit < 16u16 {
            lo ^ (lo & (1u16 << self.bit))
        } else {
            lo
        };
        let new_hi = if self.bit < 16u16 {
            hi
        } else {
            hi ^ (hi & (1u16 << (self.bit - 16u16)))
        };
        self.out = ((new_hi as u32) << 16u32) | (new_lo as u32);
        1u16
    }
}
