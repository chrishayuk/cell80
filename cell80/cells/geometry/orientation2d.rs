//! Turn direction of three 2D points (x1,y1)->(x2,y2)->(x3,y3), via the sign of the cross product (x2-x1)*(y3-y1) - (y2-y1)*(x3-x1): -1 clockwise, 0 collinear, 1 counter-clockwise -- the turn-direction primitive segments_intersect_int and convex-hull checks build on. Distinct from matrix_det_2x2 (which takes 4 raw coefficients directly): the 4 difference terms here are themselves derived from 6 point coordinates via a sign-magnitude subtract first, since a raw i16 - i16 coordinate difference can overflow i16's own range (e.g. 32767 - (-32768)).
//! tags: geometry, orientation, cross-product, turn, clockwise, counter-clockwise, collinear, predicate, sign, wide, u32, checked, escalate
//! entry: Orientation2d::run
//! limits: escalates (halt 0xFF05, needs_wider_math) on the (rare) intermediate overflow the shared add_checked_u32/mul_checked_u32 kernels guard
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct Orientation2d { x1: i16, y1: i16, x2: i16, y2: i16, x3: i16, y3: i16, sign: i16 }
impl Orientation2d {
    fn run(&mut self) -> u16 {
        let x1_mag = i16_mag(self.x1);
        let x1_neg_f = 1u16 - i16_neg(self.x1);
        let y1_mag = i16_mag(self.y1);
        let y1_neg_f = 1u16 - i16_neg(self.y1);

        // dx1 = x2 - x1
        let x2_mag = i16_mag(self.x2);
        let x2_neg = i16_neg(self.x2);
        let mut dx1_mag = 0u32;
        let mut dx1_neg = 0u16;
        if x2_neg == x1_neg_f {
            dx1_mag = add_checked_u32(x2_mag, x1_mag);
            dx1_neg = x2_neg;
        } else if x2_mag >= x1_mag {
            dx1_mag = x2_mag - x1_mag;
            dx1_neg = if dx1_mag == 0u32 { 0u16 } else { x2_neg };
        } else {
            dx1_mag = x1_mag - x2_mag;
            dx1_neg = x1_neg_f;
        }

        // dy1 = y3 - y1
        let y3_mag = i16_mag(self.y3);
        let y3_neg = i16_neg(self.y3);
        let mut dy1_mag = 0u32;
        let mut dy1_neg = 0u16;
        if y3_neg == y1_neg_f {
            dy1_mag = add_checked_u32(y3_mag, y1_mag);
            dy1_neg = y3_neg;
        } else if y3_mag >= y1_mag {
            dy1_mag = y3_mag - y1_mag;
            dy1_neg = if dy1_mag == 0u32 { 0u16 } else { y3_neg };
        } else {
            dy1_mag = y1_mag - y3_mag;
            dy1_neg = y1_neg_f;
        }

        // dy2 = y2 - y1
        let y2_mag = i16_mag(self.y2);
        let y2_neg = i16_neg(self.y2);
        let mut dy2_mag = 0u32;
        let mut dy2_neg = 0u16;
        if y2_neg == y1_neg_f {
            dy2_mag = add_checked_u32(y2_mag, y1_mag);
            dy2_neg = y2_neg;
        } else if y2_mag >= y1_mag {
            dy2_mag = y2_mag - y1_mag;
            dy2_neg = if dy2_mag == 0u32 { 0u16 } else { y2_neg };
        } else {
            dy2_mag = y1_mag - y2_mag;
            dy2_neg = y1_neg_f;
        }

        // dx2 = x3 - x1
        let x3_mag = i16_mag(self.x3);
        let x3_neg = i16_neg(self.x3);
        let mut dx2_mag = 0u32;
        let mut dx2_neg = 0u16;
        if x3_neg == x1_neg_f {
            dx2_mag = add_checked_u32(x3_mag, x1_mag);
            dx2_neg = x3_neg;
        } else if x3_mag >= x1_mag {
            dx2_mag = x3_mag - x1_mag;
            dx2_neg = if dx2_mag == 0u32 { 0u16 } else { x3_neg };
        } else {
            dx2_mag = x1_mag - x3_mag;
            dx2_neg = x1_neg_f;
        }

        // p1 = dx1 * dy1
        let p1_mag = mul_checked_u32(dx1_mag, dy1_mag);
        let p1_neg = if dx1_neg == dy1_neg { 0u16 } else { 1u16 };

        // p2 = dy2 * dx2
        let p2_mag = mul_checked_u32(dy2_mag, dx2_mag);
        let p2_neg = if dy2_neg == dx2_neg { 0u16 } else { 1u16 };

        // cross = p1 - p2
        let p2_neg_f = if p2_neg == 0u16 { 1u16 } else { 0u16 };
        let mut cross_mag = 0u32;
        let mut cross_neg = 0u16;
        if p1_neg == p2_neg_f {
            cross_mag = add_checked_u32(p1_mag, p2_mag);
            cross_neg = p1_neg;
        } else if p1_mag >= p2_mag {
            cross_mag = p1_mag - p2_mag;
            cross_neg = if cross_mag == 0u32 { 0u16 } else { p1_neg };
        } else {
            cross_mag = p2_mag - p1_mag;
            cross_neg = p2_neg_f;
        }

        let s = if cross_mag == 0u32 {
            0i16
        } else if cross_neg == 1u16 {
            -1i16
        } else {
            1i16
        };
        self.sign = s;
        1u16
    }
}
