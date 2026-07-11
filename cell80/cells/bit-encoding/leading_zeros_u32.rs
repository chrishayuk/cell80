//! Count of leading (high) zero bits in a full 32-bit value (32 for x == 0), the u32-width sibling of leading_zeros -- needs a u32 state field since leading_zeros's fn run(x: u16) cannot accept a 32-bit input under the 16-bit calling convention.
//! tags: bits, leading-zeros, clz, count, high, zeros, wide, u32
//! entry: LeadingZerosU32::run
struct LeadingZerosU32 { x: u32, out: u16 }
impl LeadingZerosU32 {
    fn run(&mut self) -> u16 {
        let mut v = self.x;
        let mut c = 0u16;
        while c < 32u16 && (v & 0x80000000u32) == 0u32 {
            v = v << 1u32;
            c = c + 1u16;
        }
        self.out = c;
        c
    }
}
