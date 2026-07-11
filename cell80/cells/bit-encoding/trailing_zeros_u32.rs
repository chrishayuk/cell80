//! Count of trailing (low) zero bits in a full 32-bit value (32 for x == 0), widened from trailing_zeros's 16-bit domain -- needs a u32 state field since trailing_zeros's fn run(x: u16) cannot accept a 32-bit input under the 16-bit calling convention.
//! tags: bits, trailing-zeros, ctz, count, low, zeros, wide, u32
//! entry: TrailingZerosU32::run
struct TrailingZerosU32 { x: u32, out: u16 }
impl TrailingZerosU32 {
    fn run(&mut self) -> u16 {
        let mut v = self.x;
        let mut c = 0u16;
        while c < 32u16 && (v & 1u32) == 0u32 {
            v = v >> 1u32;
            c = c + 1u16;
        }
        self.out = c;
        c
    }
}
