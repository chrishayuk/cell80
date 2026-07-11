//! Exact squared perpendicular distance from a point (px,py) to the infinite line through two other points (x1,y1)-(x2,y2), as a fraction: num = the (x2-x1)*(py-y1) - (y2-y1)*(px-x1) cross product squared, den = the line segment's own squared length (dx^2+dy^2) -- the pack's only distance primitive today, geom_distance_3d, is point-to-point; this reuses orientation2d's own cross-product core directly (magnitude only, sign discarded by the final square) rather than reinventing it.
//! tags: geometry, distance, point-to-line, perpendicular, line, cross-product, fraction, squared, signed, sign-magnitude, wide, u32, checked, escalate
//! entry: PointLineDistSq::run
//! limits: escalates (halt 0xFF06, out_of_domain) if the two line-defining points (x1,y1) and (x2,y2) coincide (den == 0, the line is undefined); escalates (halt 0xFF05, needs_wider_math) if any intermediate product or sum, including the final square, overflows u32 via the shared add_checked_u32/mul_checked_u32 kernels
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct PointLineDistSq { x1: i16, y1: i16, x2: i16, y2: i16, px: i16, py: i16, num: u32, den: u32 }
impl PointLineDistSq {
    fn run(&mut self) -> u16 {
        let x1_mag = i16_mag(self.x1);
        let x1_neg_f = 1u16 - i16_neg(self.x1);
        let y1_mag = i16_mag(self.y1);
        let y1_neg_f = 1u16 - i16_neg(self.y1);

        // dx = x2 - x1 (the line segment's own x delta)
        let x2_mag = i16_mag(self.x2);
        let x2_neg = i16_neg(self.x2);
        let mut dx_mag = 0u32;
        let mut dx_neg = 0u16;
        if x2_neg == x1_neg_f {
            dx_mag = add_checked_u32(x2_mag, x1_mag);
            dx_neg = x2_neg;
        } else if x2_mag >= x1_mag {
            dx_mag = x2_mag - x1_mag;
            dx_neg = if dx_mag == 0u32 { 0u16 } else { x2_neg };
        } else {
            dx_mag = x1_mag - x2_mag;
            dx_neg = x1_neg_f;
        }

        // dy = y2 - y1 (the line segment's own y delta)
        let y2_mag = i16_mag(self.y2);
        let y2_neg = i16_neg(self.y2);
        let mut dy_mag = 0u32;
        let mut dy_neg = 0u16;
        if y2_neg == y1_neg_f {
            dy_mag = add_checked_u32(y2_mag, y1_mag);
            dy_neg = y2_neg;
        } else if y2_mag >= y1_mag {
            dy_mag = y2_mag - y1_mag;
            dy_neg = if dy_mag == 0u32 { 0u16 } else { y2_neg };
        } else {
            dy_mag = y1_mag - y2_mag;
            dy_neg = y1_neg_f;
        }

        // den = dx^2 + dy^2, the line segment's own squared length
        let dx_sq = mul_checked_u32(dx_mag, dx_mag);
        let dy_sq = mul_checked_u32(dy_mag, dy_mag);
        let den = add_checked_u32(dx_sq, dy_sq);
        if den == 0u32 { halt(0xFF06u16); }

        // dpy = py - y1
        let py_mag = i16_mag(self.py);
        let py_neg = i16_neg(self.py);
        let mut dpy_mag = 0u32;
        let mut dpy_neg = 0u16;
        if py_neg == y1_neg_f {
            dpy_mag = add_checked_u32(py_mag, y1_mag);
            dpy_neg = py_neg;
        } else if py_mag >= y1_mag {
            dpy_mag = py_mag - y1_mag;
            dpy_neg = if dpy_mag == 0u32 { 0u16 } else { py_neg };
        } else {
            dpy_mag = y1_mag - py_mag;
            dpy_neg = y1_neg_f;
        }

        // dpx = px - x1
        let px_mag = i16_mag(self.px);
        let px_neg = i16_neg(self.px);
        let mut dpx_mag = 0u32;
        let mut dpx_neg = 0u16;
        if px_neg == x1_neg_f {
            dpx_mag = add_checked_u32(px_mag, x1_mag);
            dpx_neg = px_neg;
        } else if px_mag >= x1_mag {
            dpx_mag = px_mag - x1_mag;
            dpx_neg = if dpx_mag == 0u32 { 0u16 } else { px_neg };
        } else {
            dpx_mag = x1_mag - px_mag;
            dpx_neg = x1_neg_f;
        }

        // p1 = dx * dpy
        let p1_mag = mul_checked_u32(dx_mag, dpy_mag);
        let p1_neg = if dx_neg == dpy_neg { 0u16 } else { 1u16 };

        // p2 = dy * dpx
        let p2_mag = mul_checked_u32(dy_mag, dpx_mag);
        let p2_neg = if dy_neg == dpx_neg { 0u16 } else { 1u16 };

        // cross = p1 - p2 (magnitude only -- the final square erases the sign, so
        // cross's own sign is never tracked past this point, unlike orientation2d).
        let p2_neg_f = if p2_neg == 0u16 { 1u16 } else { 0u16 };
        let mut cross_mag = 0u32;
        if p1_neg == p2_neg_f {
            cross_mag = add_checked_u32(p1_mag, p2_mag);
        } else if p1_mag >= p2_mag {
            cross_mag = p1_mag - p2_mag;
        } else {
            cross_mag = p2_mag - p1_mag;
        }

        let num = mul_checked_u32(cross_mag, cross_mag);
        self.num = num;
        self.den = den;
        1u16
    }
}
