//! Remainder of two u32 values: a % b. Escalates (needs_wider_math) if b is zero.
//! tags: math, modulo, remainder, wide, u32
//! entry: ModU32::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if b == 0
struct ModU32 { a: u32, b: u32, rem: u32 }
impl ModU32 {
    fn run(&mut self) -> u16 {
        if self.b == 0u32 { halt(0xFF05u16); }
        self.rem = self.a % self.b;
        1u16
    }
}
