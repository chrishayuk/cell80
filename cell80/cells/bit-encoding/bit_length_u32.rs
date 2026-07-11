//! Number of bits needed to represent a full 32-bit value x: index of the highest set bit + 1 (0 for x == 0), widened from bit_length's 16-bit domain -- needs a u32 state field since bit_length's fn run(x: u16) cannot accept a 32-bit input under the 16-bit calling convention.
//! tags: bits, bit-length, msb, highest-bit, log2, magnitude, wide, u32
//! entry: BitLengthU32::run
struct BitLengthU32 { x: u32, out: u16 }
impl BitLengthU32 {
    fn run(&mut self) -> u16 {
        let mut v = self.x;
        let mut c = 0u16;
        while v != 0u32 {
            c = c + 1u16;
            v = v >> 1u32;
        }
        self.out = c;
        c
    }
}
