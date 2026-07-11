//! Toggle (flip) bit number `bit` (0-31) of a 32-bit value x, the u32-width sibling of toggle_bit -- needs a u32 state field since toggle_bit's fn run(x: u16, bit: u16) cannot accept a 32-bit input under the 16-bit calling convention; splits x into hi/lo 16-bit halves (each split off by a constant shift) and XORs the target bit within whichever half holds it using a runtime-indexed 16-bit shift, the same technique set_bit_u32 and clear_bit_u32 use.
//! tags: bits, toggle, flip, xor, flag, invert, wide, u32
//! entry: ToggleBitU32::run
struct ToggleBitU32 { x: u32, bit: u16, out: u32 }
impl ToggleBitU32 {
    fn run(&mut self) -> u16 {
        let lo = self.x as u16;
        let hi = (self.x >> 16u32) as u16;
        let new_lo = if self.bit < 16u16 {
            lo ^ (1u16 << self.bit)
        } else {
            lo
        };
        let new_hi = if self.bit < 16u16 {
            hi
        } else {
            hi ^ (1u16 << (self.bit - 16u16))
        };
        self.out = ((new_hi as u32) << 16u32) | (new_lo as u32);
        1u16
    }
}
