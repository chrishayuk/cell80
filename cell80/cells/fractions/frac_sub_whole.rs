//! Subtract a whole number from a fraction: n/d - whole, reduced to lowest terms via the shared gcd_u32 kernel — the frac-minus-whole sibling missing alongside frac_add_whole and frac_sub_from_whole, e.g. 7/2 - 1 = 5/2.
//! tags: fraction, frac, subtract, whole, integer, mixed, wide, u32, checked, escalate
//! entry: FracSubWhole::run
//! limits: escalates (halt 0xFF06, out_of_domain) if d == 0; escalates (halt 0xFF05, needs_wider_math) if whole*d overflows u32 or whole*d > n (the result would be negative)
struct FracSubWhole { n: u32, d: u32, whole: u32, num: u32, den: u32 }
impl FracSubWhole {
    fn run(&mut self) -> u16 {
        if self.d == 0u32 { halt(0xFF06u16); }
        let wd = mul_checked_u32(self.whole, self.d);
        if wd > self.n { halt(0xFF05u16); }
        let num_raw = self.n - wd;
        if num_raw == 0u32 {
            self.num = 0u32;
            self.den = 1u32;
            return 1u16;
        }
        let g = gcd_u32(num_raw, self.d);
        self.num = num_raw / g;
        self.den = self.d / g;
        1u16
    }
}
