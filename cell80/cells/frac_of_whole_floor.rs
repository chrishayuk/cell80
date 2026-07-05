//! A fraction of a whole number, rounded down: floor(n/d * whole) — the floor sibling of frac_of_whole (which escalates if the result isn't exact). Never escalates on an inexact split (e.g. "90% of 23" is a real, non-exact GSM8K-style shape, unlike "3/4 of 20"); still escalates if the multiply overflows.
//! tags: fraction, frac, of, whole, multiply, floor, round, down, wide, u32, checked, escalate, percent, scale, dollars, cents, hours, minutes, ratio
//! entry: FracOfWholeFloor::run
//! limits: escalates (halt 0xFF06, out_of_domain) if d == 0; escalates (halt 0xFF05, needs_wider_math) if n * whole overflows u32
struct FracOfWholeFloor { n: u32, d: u32, whole: u32, result: u32 }
impl FracOfWholeFloor {
    fn run(&mut self) -> u16 {
        if self.d == 0u32 { halt(0xFF06u16); }
        let p = mul_checked_u32(self.n, self.whole);
        self.result = p / self.d;
        1u16
    }
}
