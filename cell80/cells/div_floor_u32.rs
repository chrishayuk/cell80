//! Floor division of two u32 values: a / b, rounded down. Escalates (needs_wider_math) if b is zero.
//! tags: math, divide, floor, wide, u32
//! entry: DivFloor::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if b == 0
struct DivFloor { a: u32, b: u32, quotient: u32 }
impl DivFloor {
    fn run(&mut self) -> u16 {
        if self.b == 0u32 { halt(0xFF05u16); }
        self.quotient = self.a / self.b;
        1u16
    }
}
