//! Verifies a claimed wide remainder: 0 if b == 0, else 1 if a % b == rem, else 0 — the verifier counterpart of mod_u32 (that one computes and escalates on a zero divisor; this one checks a candidate leftover count and always returns a verdict).
//! tags: verify, verifier, equation, remainder, modulo, wide, u32, check, plan, reverse-equation
//! entry: RemainderEqualsU32::run
struct RemainderEqualsU32 { a: u32, b: u32, rem: u32 }
impl RemainderEqualsU32 {
    fn run(&mut self) -> u16 {
        if self.b == 0u32 {
            0u16
        } else {
            ((self.a % self.b) == self.rem) as u16
        }
    }
}
