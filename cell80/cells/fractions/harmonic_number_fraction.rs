//! The nth harmonic number H_n = 1/1 + 1/2 + ... + 1/n as a single exact reduced fraction, accumulated internally via a bounded loop of frac_add-style common-denominator-then-reduce steps (num/den + 1/i -> (num*i+den)/(den*i), then gcd_u32 reduce) -- distinct from frac_add's fixed 2-term sum, this sums an arbitrary run-length sequence of unit fractions in one call.
//! tags: fraction, frac, harmonic, series, sum, sequence, loop, gcd, wide, u32, checked, escalate
//! entry: HarmonicNumberFraction::run
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0; escalates (halt 0xFF05, needs_wider_math) the moment an intermediate common-denominator multiply or numerator add overflows u32 (around n in the low-to-mid 20s, given the harmonic denominator's fast growth)
struct HarmonicNumberFraction { n: u32, num: u32, den: u32 }
impl HarmonicNumberFraction {
    fn run(&mut self) -> u16 {
        if self.n == 0u32 { halt(0xFF06u16); }
        let mut num = 1u32;
        let mut den = 1u32;
        let mut i = 2u32;
        while i <= self.n {
            let t1 = mul_checked_u32(num, i);
            let num_raw = add_checked_u32(t1, den);
            let den_raw = mul_checked_u32(den, i);
            let g = gcd_u32(num_raw, den_raw);
            num = num_raw / g;
            den = den_raw / g;
            i = i + 1u32;
        }
        self.num = num;
        self.den = den;
        1u16
    }
}
