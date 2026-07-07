//! Modular addition at wide u32 width: (a + b) mod m — reduces both operands mod m first, so a and b need not already be canonical residues.
//! tags: number, modular, modulo, add, sum, wide, u32, checked, escalate, aime, number-theory
//! entry: ModAddWide::run
//! limits: escalates (halt 0xFF06, out_of_domain) if m == 0; escalates (halt 0xFF05, needs_wider_math) if the reduced sum overflows u32 (m near u32::MAX)
struct ModAddWide { a: u32, b: u32, m: u32, result: u32 }
impl ModAddWide {
    fn run(&mut self) -> u16 {
        if self.m == 0u32 { halt(0xFF06u16); }
        let ra = self.a % self.m;
        let rb = self.b % self.m;
        let s = add_checked_u32(ra, rb);
        let r = if s >= self.m { s - self.m } else { s };
        self.result = r;
        1u16
    }
}
