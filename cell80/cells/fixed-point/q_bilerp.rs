//! Bilinear interpolation of four Q8.8 corner values by two Q0.8 fractions tx (x-axis) and ty (y-axis): lerp(lerp(q00,q10,tx), lerp(q01,q11,tx), ty) -- q_lerp is this pack's only interpolation primitive and it is strictly 1D, so this state cell inlines q_lerp's own a+/-diff*t>>8 technique three times (top edge, bottom edge, then across those two) since cells can't call each other.
//! tags: fixed-point, q8.8, q0.8, lerp, interpolate, bilinear, bilerp, 2d, grid, blend
//! entry: QBilerp::run
struct QBilerp {
    q00: u16,
    q10: u16,
    q01: u16,
    q11: u16,
    tx: u16,
    ty: u16,
    out: u16,
}
impl QBilerp {
    fn run(&mut self) -> u16 {
        let top = lerp_step(self.q00, self.q10, self.tx);
        let bottom = lerp_step(self.q01, self.q11, self.tx);
        let out = lerp_step(top, bottom, self.ty);
        self.out = out;
        out
    }
}
fn lerp_step(a: u16, b: u16, t: u16) -> u16 {
    if b >= a {
        let diff = (b - a) as u32;
        a + ((diff * t as u32) >> 8u32) as u16
    } else {
        let diff = (a - b) as u32;
        a - ((diff * t as u32) >> 8u32) as u16
    }
}
