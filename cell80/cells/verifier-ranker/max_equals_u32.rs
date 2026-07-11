//! Verifies a claimed wide max: returns 1 if claimed == (if a > b { a } else { b }), else 0, matching max_u32's own tie-break (b wins ties) — the reverse-equation counterpart of max_u32 (that one computes; this one checks a candidate answer and always returns a verdict).
//! tags: verify, verifier, equation, max, maximum, larger, bigger, compare, wide, u32, reverse-equation
//! entry: MaxEqualsWide::run
struct MaxEqualsWide { a: u32, b: u32, claimed: u32 }
impl MaxEqualsWide {
    fn run(&mut self) -> u16 {
        let m = if self.a > self.b { self.a } else { self.b };
        (m == self.claimed) as u16
    }
}
