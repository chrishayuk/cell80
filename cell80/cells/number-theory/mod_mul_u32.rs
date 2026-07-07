//! Modular multiplication at wide u32 width: (a * b) mod m — reduces both operands mod m first, then multiplies; the non-exponentiating sibling of pow_mod_u32, sharing its overflow bound.
//! tags: number, modular, modulo, multiply, product, wide, u32, checked, escalate, aime, number-theory
//! entry: ModMulWide::run
//! limits: escalates (halt 0xFF06, out_of_domain) if m == 0; escalates (halt 0xFF05, needs_wider_math) if m > 65536 (the product of two reduced operands would overflow u32)
struct ModMulWide { a: u32, b: u32, m: u32, result: u32 }
impl ModMulWide {
    fn run(&mut self) -> u16 {
        if self.m == 0u32 { halt(0xFF06u16); }
        if self.m > 65536u32 { halt(0xFF05u16); }
        let ra = self.a % self.m;
        let rb = self.b % self.m;
        self.result = ra * rb % self.m;
        1u16
    }
}
