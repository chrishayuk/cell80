//! Verifies a claimed wide min: returns 1 if claimed == (if a < b { a } else { b }), else 0 -- the reverse-equation counterpart of min_u32, the direct complement of max_equals_u32.
//! tags: verify, verifier, equation, min, minimum, smaller, smallest, least, lesser, wide, u32, check, plan, reverse-equation
//! entry: MinEqualsWide::run
struct MinEqualsWide { a: u32, b: u32, claimed: u32 }
impl MinEqualsWide {
    fn run(&mut self) -> u16 {
        let m = if self.a < self.b { self.a } else { self.b };
        (m == self.claimed) as u16
    }
}
