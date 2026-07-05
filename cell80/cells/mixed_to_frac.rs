//! Convert a mixed number (whole + num/den) to a single improper fraction: n = whole*den + num, d = den — the exact inverse of frac_to_mixed.
//! tags: fraction, frac, mixed, mixed-number, improper, convert, wide, u32, checked, escalate
//! entry: MixedToFrac::run
//! limits: escalates (halt 0xFF06, out_of_domain) if den == 0; escalates (halt 0xFF05, needs_wider_math) if whole*den or the final sum overflows u32
struct MixedToFrac { whole: u32, num: u32, den: u32, n: u32, d: u32 }
impl MixedToFrac {
    fn run(&mut self) -> u16 {
        if self.den == 0u32 { halt(0xFF06u16); }
        let wd = mul_checked_u32(self.whole, self.den);
        let n_raw = wd.wrapping_add(self.num);
        if n_raw < wd { halt(0xFF05u16); }
        self.n = n_raw;
        self.d = self.den;
        1u16
    }
}
