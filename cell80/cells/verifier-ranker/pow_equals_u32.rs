//! Verifies a claimed wide power: returns 1 if base^exp == total, else 0, including when an intermediate multiply overflows u32 — the reverse-equation counterpart of pow_checked_u32.
//! tags: verify, verifier, equation, power, exponent, pow, wide, u32, check, plan, reverse-equation
//! entry: PowEqualsWide::run
struct PowEqualsWide { base: u32, exp: u32, total: u32 }
impl PowEqualsWide {
    fn run(&mut self) -> u16 {
        let mut r = 1u32;
        let mut i = 0u32;
        let mut overflowed = 0u16;
        while i < self.exp {
            let p = r.wrapping_mul(self.base);
            if r != 0u32 && p / r != self.base { overflowed = 1u16; }
            r = p;
            i = i + 1u32;
        }
        if overflowed == 1u16 { 0u16 } else { (r == self.total) as u16 }
    }
}
