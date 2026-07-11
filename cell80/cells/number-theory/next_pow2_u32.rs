//! Wide sibling of next_pow2: smallest power of two >= n at u32 width (0 if it would exceed u32::MAX; next_pow2_u32(0) = 1) -- the same left-shift-until-large-enough loop as next_pow2, run at u32 width so it also covers next_pow2's blind spot past 65535.
//! tags: number, power, round-up, pow2, ceiling, bits, wide, u32, large, number-theory
//! entry: NextPow2Wide::run
//! limits: returns 0 if the answer would exceed u32::MAX (the smallest power of two >= n overflows 32 bits), matching next_pow2's own past-ceiling convention
struct NextPow2Wide { n: u32, result: u32 }
impl NextPow2Wide {
    fn run(&mut self) -> u16 {
        let mut p = 1u32;
        while p < self.n && p != 0u32 { p = p << 1u32; }
        self.result = p;
        1u16
    }
}
