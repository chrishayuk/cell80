//! Exact squared distance from a point (px,py) to the closest point on the FINITE segment (x1,y1)-(x2,y2): projects (px,py) onto the segment via the dot-product parameter t_num/t_den, clamps to the nearer endpoint (den=1, exact squared endpoint distance) when the foot falls outside [0,1], otherwise reuses point_line_dist_sq's own cross-product-squared fraction (num/den = perpendicular distance to the infinite line) -- point_line_dist_sq's own summary is explicitly scoped to the infinite line, so this is the pack's first finite-segment point-distance primitive.
//! tags: geometry, distance, point-to-segment, finite, segment, clamp, endpoint, projection, dot-product, cross-product, fraction, squared, sign-magnitude, wide, u32, checked, escalate
//! entry: PointSegmentDistSq::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if any intermediate product or sum overflows u32 via the shared add_checked_u32/mul_checked_u32 kernels; a degenerate segment (x1,y1)==(x2,y2) does not halt, it returns the exact squared distance to that single point instead (den=1)
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct PointSegmentDistSq { px: i16, py: i16, x1: i16, y1: i16, x2: i16, y2: i16, num: u32, den: u32 }
impl PointSegmentDistSq {
    fn run(&mut self) -> u16 {
        let x1_mag = i16_mag(self.x1);
        let x1_neg_f = 1u16 - i16_neg(self.x1);
        let y1_mag = i16_mag(self.y1);
        let y1_neg_f = 1u16 - i16_neg(self.y1);

        let x2_mag = i16_mag(self.x2);
        let x2_neg = i16_neg(self.x2);
        let x2_neg_f = 1u16 - x2_neg;
        let y2_mag = i16_mag(self.y2);
        let y2_neg = i16_neg(self.y2);
        let y2_neg_f = 1u16 - y2_neg;

        let px_mag = i16_mag(self.px);
        let px_neg = i16_neg(self.px);
        let py_mag = i16_mag(self.py);
        let py_neg = i16_neg(self.py);

        // dx = x2 - x1, dy = y2 - y1 (the segment's own vector, B - A)
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

        // dpx = px - x1, dpy = py - y1 (vector A -> P)
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

        // dbx = px - x2, dby = py - y2 (vector B -> P; magnitude only, each is squared
        // standalone below so no further sign combination is ever needed for these two).
        let mut dbx_mag = 0u32;
        if px_neg == x2_neg_f {
            dbx_mag = add_checked_u32(px_mag, x2_mag);
        } else if px_mag >= x2_mag {
            dbx_mag = px_mag - x2_mag;
        } else {
            dbx_mag = x2_mag - px_mag;
        }

        let mut dby_mag = 0u32;
        if py_neg == y2_neg_f {
            dby_mag = add_checked_u32(py_mag, y2_mag);
        } else if py_mag >= y2_mag {
            dby_mag = py_mag - y2_mag;
        } else {
            dby_mag = y2_mag - py_mag;
        }

        // t_den = dx^2 + dy^2 (the segment's own squared length)
        let dx_sq = mul_checked_u32(dx_mag, dx_mag);
        let dy_sq = mul_checked_u32(dy_mag, dy_mag);
        let t_den = add_checked_u32(dx_sq, dy_sq);

        if t_den == 0u32 {
            // Degenerate segment (x1,y1) == (x2,y2): the closest point is that single
            // point, so return the exact squared distance to it instead of halting.
            let dpx_sq = mul_checked_u32(dpx_mag, dpx_mag);
            let dpy_sq = mul_checked_u32(dpy_mag, dpy_mag);
            self.num = add_checked_u32(dpx_sq, dpy_sq);
            self.den = 1u32;
            return 1u16;
        }

        // t_num = dpx*dx + dpy*dy, the dot product of (P-A) with (B-A); the projection
        // parameter is t_num/t_den, but we only ever need its sign and its comparison
        // against t_den, never the fraction itself.
        let q1_mag = mul_checked_u32(dpx_mag, dx_mag);
        let q1_neg = if dpx_neg == dx_neg { 0u16 } else { 1u16 };
        let q2_mag = mul_checked_u32(dpy_mag, dy_mag);
        let q2_neg = if dpy_neg == dy_neg { 0u16 } else { 1u16 };
        let mut t_num_mag = 0u32;
        let mut t_num_neg = 0u16;
        if q1_neg == q2_neg {
            t_num_mag = add_checked_u32(q1_mag, q2_mag);
            t_num_neg = q1_neg;
        } else if q1_mag >= q2_mag {
            t_num_mag = q1_mag - q2_mag;
            t_num_neg = if t_num_mag == 0u32 { 0u16 } else { q1_neg };
        } else {
            t_num_mag = q2_mag - q1_mag;
            t_num_neg = q2_neg;
        }

        if t_num_neg == 1u16 || t_num_mag == 0u32 {
            // Foot of the perpendicular falls at or before A -- clamp to (x1,y1).
            let dpx_sq = mul_checked_u32(dpx_mag, dpx_mag);
            let dpy_sq = mul_checked_u32(dpy_mag, dpy_mag);
            self.num = add_checked_u32(dpx_sq, dpy_sq);
            self.den = 1u32;
            return 1u16;
        }

        if t_num_mag >= t_den {
            // Foot of the perpendicular falls at or beyond B -- clamp to (x2,y2).
            let dbx_sq = mul_checked_u32(dbx_mag, dbx_mag);
            let dby_sq = mul_checked_u32(dby_mag, dby_mag);
            self.num = add_checked_u32(dbx_sq, dby_sq);
            self.den = 1u32;
            return 1u16;
        }

        // Foot lands strictly inside the segment: same cross^2 / (dx^2+dy^2) fraction
        // point_line_dist_sq returns for the infinite line (magnitude only -- the final
        // square erases the sign, exactly as point_line_dist_sq's own cross_mag does).
        let p1_mag = mul_checked_u32(dx_mag, dpy_mag);
        let p1_neg = if dx_neg == dpy_neg { 0u16 } else { 1u16 };
        let p2_mag = mul_checked_u32(dy_mag, dpx_mag);
        let p2_neg = if dy_neg == dpx_neg { 0u16 } else { 1u16 };
        let p2_neg_f = if p2_neg == 0u16 { 1u16 } else { 0u16 };
        let mut cross_mag = 0u32;
        if p1_neg == p2_neg_f {
            cross_mag = add_checked_u32(p1_mag, p2_mag);
        } else if p1_mag >= p2_mag {
            cross_mag = p1_mag - p2_mag;
        } else {
            cross_mag = p2_mag - p1_mag;
        }

        self.num = mul_checked_u32(cross_mag, cross_mag);
        self.den = t_den;
        1u16
    }
}
