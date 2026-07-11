//! Evaluate a fixed-degree-4 polynomial a*x^4 + b*x^3 + c*x^2 + d*x + e at a given x via Horner's method ((((a*x+b)*x+c)*x+d)*x+e), checking every multiply and add along the way so it escalates the instant any partial product or sum would overflow u32.
//! tags: number, polynomial, horner, quartic, evaluate, sequence, series, math, checked, wide, u32, escalate
//! entry: HornerQuartic::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if any partial multiply or add overflows u32
struct HornerQuartic { a: u32, b: u32, c: u32, d: u32, e: u32, x: u32, result: u32 }
impl HornerQuartic {
    fn run(&mut self) -> u16 {
        let mut acc = self.a;
        acc = mul_checked_u32(acc, self.x);
        acc = add_checked_u32(acc, self.b);
        acc = mul_checked_u32(acc, self.x);
        acc = add_checked_u32(acc, self.c);
        acc = mul_checked_u32(acc, self.x);
        acc = add_checked_u32(acc, self.d);
        acc = mul_checked_u32(acc, self.x);
        acc = add_checked_u32(acc, self.e);
        self.result = acc;
        1u16
    }
}
