//! Verifies a claimed wide difference: returns 1 if a >= b and a - b == remainder, else 0 (including when a < b, since an unsigned difference can't be negative) — the wide sibling of diff_equals (which works over u16).
//! tags: verify, verifier, equation, difference, subtract, wide, u32, check, plan, reverse-equation
//! entry: DiffEqualsWide::run
struct DiffEqualsWide { a: u32, b: u32, remainder: u32 }
impl DiffEqualsWide {
    fn run(&mut self) -> u16 {
        if self.a < self.b {
            0u16
        } else {
            ((self.a - self.b) == self.remainder) as u16
        }
    }
}
