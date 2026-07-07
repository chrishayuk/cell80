//! Ceiling division of two u32 values: the smallest integer >= a / b. Escalates (needs_wider_math) if b is zero.
//! tags: math, divide, ceiling, round-up, wide, u32
//! entry: DivCeil::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if b == 0
struct DivCeil { a: u32, b: u32, quotient: u32 }
impl DivCeil {
    fn run(&mut self) -> u16 {
        if self.b == 0u32 { halt(0xFF05u16); }
        // q + 1 if there's a remainder, computed this way (not `(a+b-1)/b`) so a near
        // u32::MAX never risks an intermediate overflow — "checked" is the whole point.
        let q = self.a / self.b;
        let r = self.a % self.b;
        let rounded = if r != 0u32 { q + 1u32 } else { q };
        self.quotient = rounded;
        1u16
    }
}
