//! Raise a fraction n/d to a whole-number power k: (n/d)^k = n^k/d^k, each built by repeated mul_checked_u32 (geometric_nth_checked_u32's technique) rather than a native exponent op, reduced once at the end via gcd_u32.
//! tags: fraction, frac, power, exponent, pow, whole, checked, wide, u32, escalate
//! entry: FracPow::run
//! limits: escalates (halt 0xFF06, out_of_domain) if d == 0; escalates (halt 0xFF05, needs_wider_math) the moment n^k or d^k overflows u32; k == 0 returns 1/1
struct FracPow { n: u32, d: u32, k: u32, num: u32, den: u32 }
impl FracPow {
    fn run(&mut self) -> u16 {
        if self.d == 0u32 { halt(0xFF06u16); }
        let mut num_raw = 1u32;
        let mut den_raw = 1u32;
        let mut i = 0u32;
        while i < self.k {
            num_raw = mul_checked_u32(num_raw, self.n);
            den_raw = mul_checked_u32(den_raw, self.d);
            i = i + 1u32;
        }
        if num_raw == 0u32 {
            self.num = 0u32;
            self.den = 1u32;
            return 1u16;
        }
        let g = gcd_u32(num_raw, den_raw);
        self.num = num_raw / g;
        self.den = den_raw / g;
        1u16
    }
}
