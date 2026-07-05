//! Checked exact exponentiation at u32: base^exp, escalating the moment a multiply step would overflow (distinct from pow_small, which saturates at u16 — this stays exact or hands off). 0^0 = 1.
//! tags: math, power, exponent, pow, checked, wide, u32, overflow, escalate
//! entry: PowChecked::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if any intermediate multiply exceeds u32::MAX
struct PowChecked { base: u32, exp: u32, result: u32 }
impl PowChecked {
    fn run(&mut self) -> u16 {
        let mut r = 1u32;
        let mut i = 0u32;
        while i < self.exp {
            let p = r.wrapping_mul(self.base);
            if r != 0u32 && p / r != self.base { halt(0xFF05u16); }
            r = p;
            i = i + 1u32;
        }
        self.result = r;
        1u16
    }
}
