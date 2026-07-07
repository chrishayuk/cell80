//! 16 times the squared area of a triangle with integer sides (a, b, c), via Heron's formula rearranged to avoid square roots entirely: 16·Area² = (a+b+c)(−a+b+c)(a−b+c)(a+b−c). Always a non-negative integer for a valid triangle — comparable, summable, and equality-testable without ever taking a root.
//! tags: geometry, triangle, area, heron, squared, wide, u32, checked, escalate, aime
//! entry: Heron16A2::run
//! limits: escalates (halt 0xFF06, out_of_domain) if a, b, c do not form a valid (non-degenerate) triangle; escalates (halt 0xFF05, needs_wider_math) if either factor-pair product overflows u32
struct Heron16A2 { a: u16, b: u16, c: u16, result: u32 }
impl Heron16A2 {
    fn run(&mut self) -> u16 {
        let aw = self.a as u32;
        let bw = self.b as u32;
        let cw = self.c as u32;
        if aw + bw <= cw || bw + cw <= aw || aw + cw <= bw { halt(0xFF06u16); }
        let s1 = aw + bw + cw;
        let s2 = bw + cw - aw;
        let s3 = aw + cw - bw;
        let s4 = aw + bw - cw;
        let p1 = mul_checked_u32(s1, s2);
        let p2 = mul_checked_u32(s3, s4);
        let r = mul_checked_u32(p1, p2);
        self.result = r;
        1u16
    }
}
