//! Convert an improper fraction n/d to a mixed number: whole + num/den, where the remaining fraction is reduced to lowest terms via the shared gcd_u32 kernel (num=0, den=1 if n divides evenly by d).
//! tags: fraction, frac, mixed, mixed-number, whole, remainder, wide, u32, checked
//! entry: FracToMixed::run
//! limits: escalates (halt 0xFF06, out_of_domain) if d == 0
struct FracToMixed { n: u32, d: u32, whole: u32, num: u32, den: u32 }
impl FracToMixed {
    fn run(&mut self) -> u16 {
        if self.d == 0u32 { halt(0xFF06u16); }
        self.whole = self.n / self.d;
        let rem = self.n % self.d;
        if rem == 0u32 {
            self.num = 0u32;
            self.den = 1u32;
            return 1u16;
        }
        let g = gcd_u32(rem, self.d);
        self.num = rem / g;
        self.den = self.d / g;
        1u16
    }
}
