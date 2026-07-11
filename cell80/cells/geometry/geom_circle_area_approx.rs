//! Floor(pi * r^2) for an integer radius r, via a Q8.8 fixed-pi constant (804 = round(3.14159265*256)): area = (r*r * 804) >> 8 -- the fixed-point pack's already-proven scale applied to circle area, distinct from any exact-fraction geometry cell since pi itself is irrational and this deliberately commits to a bounded fixed-point truncation error instead.
//! tags: geometry, circle, area, pi, fixed-point, q8.8, wide, u32, checked, escalate
//! entry: GeomCircleAreaApprox::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if r*r or r*r*804 would overflow u32
struct GeomCircleAreaApprox { r: u16, area: u32 }
impl GeomCircleAreaApprox {
    fn run(&mut self) -> u16 {
        let rw = self.r as u32;
        let r_sq = mul_checked_u32(rw, rw);
        let scaled = mul_checked_u32(r_sq, 804u32);
        self.area = scaled >> 8u32;
        1u16
    }
}
