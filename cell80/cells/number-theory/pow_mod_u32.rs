//! Modular exponentiation at wide u32 width: (base^exp) mod m — the wide sibling of pow_mod (u16 domain, m <= 256); lifts the modulus ceiling to 65536, wide enough for AIME's "find the remainder mod 1000" finishing move. Returns 0 if m == 0, matching pow_mod's convention.
//! tags: number, modular, exponent, pow-mod, modulo, wide, u32, checked, escalate, aime, number-theory
//! entry: PowModWide::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if m > 65536 (the squared intermediate would overflow u32)
struct PowModWide { base: u32, exp: u32, m: u32, result: u32 }
impl PowModWide {
    fn run(&mut self) -> u16 {
        if self.m > 65536u32 { halt(0xFF05u16); }
        let mut r = 0u32;
        if self.m != 0u32 {
            r = 1u32 % self.m;
            let mut b = self.base % self.m;
            let mut e = self.exp;
            while e != 0u32 {
                if e % 2u32 == 1u32 { r = r * b % self.m; }
                b = b * b % self.m;
                e = e / 2u32;
            }
        }
        self.result = r;
        1u16
    }
}
