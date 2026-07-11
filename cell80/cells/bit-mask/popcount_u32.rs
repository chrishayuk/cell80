//! Population count of a full 32-bit value: the number of set bits in x, widened from popcount's 16-bit domain -- needs a u32 state field since popcount's fn run(x: u16) cannot accept a 32-bit input under the 16-bit calling convention.
//! tags: bits, popcount, count, ones, hamming-weight, bitcount, wide, u32
//! entry: PopcountU32::run
struct PopcountU32 { x: u32, out: u16 }
impl PopcountU32 {
    fn run(&mut self) -> u16 {
        let mut v = self.x;
        let mut c = 0u16;
        while v != 0u32 {
            c = c + ((v & 1u32) as u16);
            v = v >> 1u32;
        }
        self.out = c;
        c
    }
}
