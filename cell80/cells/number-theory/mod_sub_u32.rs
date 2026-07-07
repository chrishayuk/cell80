//! Modular subtraction at wide u32 width: (a - b) mod m, always returned in [0, m) — e.g. 3 - 5 mod 7 = 5, not a negative remainder.
//! tags: number, modular, modulo, subtract, difference, wide, u32, checked, escalate, aime, number-theory
//! entry: ModSubWide::run
//! limits: escalates (halt 0xFF06, out_of_domain) if m == 0
struct ModSubWide { a: u32, b: u32, m: u32, result: u32 }
impl ModSubWide {
    fn run(&mut self) -> u16 {
        if self.m == 0u32 { halt(0xFF06u16); }
        let ra = self.a % self.m;
        let rb = self.b % self.m;
        let r = if ra >= rb { ra - rb } else { self.m - (rb - ra) };
        self.result = r;
        1u16
    }
}
