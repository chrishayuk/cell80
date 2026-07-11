//! Smallest of three wide u32 values, min_u32(min_u32(a,b),c) — the arity-3 sibling min_u32/max_u32 lack in this pack; distinct from argmin3_u32, which returns the winning index, not the value.
//! tags: math, min, minimum, three, min3, triple, smallest, least, lesser, compare, select, wide, u32, large
//! entry: Min3Wide::run
struct Min3Wide { a: u32, b: u32, c: u32, result: u32 }
impl Min3Wide {
    fn run(&mut self) -> u16 {
        let m1 = if self.a < self.b { self.a } else { self.b };
        let m2 = if m1 < self.c { m1 } else { self.c };
        self.result = m2;
        1u16
    }
}
