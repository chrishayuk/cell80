//! A fraction of a whole number, rounded up: ceil(n/d * whole) — the ceiling sibling of frac_of_whole (escalates if inexact) and frac_of_whole_floor (rounds down, never escalates on inexactness); uses the q+1-if-remainder technique from checked-arithmetic's div_ceil_u32 so a near-u32::MAX product never risks an intermediate overflow.
//! tags: fraction, frac, of, whole, multiply, ceil, ceiling, round, up, wide, u32, checked, escalate, percent, scale, dollars, cents, hours, minutes, ratio
//! entry: FracOfWholeCeil::run
//! limits: escalates (halt 0xFF06, out_of_domain) if d == 0; escalates (halt 0xFF05, needs_wider_math) if n * whole overflows u32
struct FracOfWholeCeil { n: u32, d: u32, whole: u32, result: u32 }
impl FracOfWholeCeil {
    fn run(&mut self) -> u16 {
        if self.d == 0u32 { halt(0xFF06u16); }
        let p = mul_checked_u32(self.n, self.whole);
        let q = p / self.d;
        let r = p % self.d;
        let rounded = if r != 0u32 { q + 1u32 } else { q };
        self.result = rounded;
        1u16
    }
}
