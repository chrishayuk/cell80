//! Median (middle value) of three wide u32 values via median3's exact modular-arithmetic trick (a+b+c-min-max using wrapping_add/wrapping_sub) — the wide sibling of median3 (which works over u16 and can't rank values beyond 65535, e.g. money totals in cents).
//! tags: median, middle, three, stat, midpoint, central, wide, u32, large
//! entry: Median3Wide::run
struct Median3Wide { a: u32, b: u32, c: u32, result: u32 }
impl Median3Wide {
    fn run(&mut self) -> u16 {
        let m1 = if self.a < self.b { self.a } else { self.b };
        let lo = if m1 < self.c { m1 } else { self.c };
        let m2 = if self.a > self.b { self.a } else { self.b };
        let hi = if m2 > self.c { m2 } else { self.c };
        let v = self.a.wrapping_add(self.b).wrapping_add(self.c).wrapping_sub(lo).wrapping_sub(hi);
        self.result = v;
        1u16
    }
}
