//! Exact parametric-fraction solution for where the two infinite lines through (x1,y1)-(x2,y2) and (x3,y3)-(x4,y4) cross: t=t_num/den is how far along line 1 (P1+t*(P2-P1)), u=u_num/den is how far along line 2, sharing one denominator den=cross(d1,d2) of the two direction vectors d1=(x2-x1,y2-y1) and d2=(x4-x3,y4-y3) -- the safely-computable core segments_intersect_int's own cell-index.md summary names as still open (geom_line_intersection, the literal Cartesian crossing point, needs ~48-bit intermediates and is structurally blocked), since t_num=cross(w,d2) and u_num=cross(w,d1) with w=(x3-x1,y3-y1) only need the same bounded coordinate-difference cross-products orientation2d and point_line_dist_sq already handle safely, and unlike linear_solve_1var's fraction this is deliberately left unreduced (no gcd), just sign-normalized so den is always positive.
//! tags: geometry, line, intersect, infinite, parametric, fraction, cross-product, direction-vector, signed, wide, u32, checked, escalate
//! entry: LineIntersectParamsFrac::run
//! limits: escalates (halt 0xFF06, out_of_domain) if den == 0 (the two lines are parallel or coincident); escalates (halt 0xFF05, needs_wider_math) if any intermediate product or sum overflows u32 via the shared add_checked_u32/mul_checked_u32 kernels
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct LineIntersectParamsFrac { x1: i16, y1: i16, x2: i16, y2: i16, x3: i16, y3: i16, x4: i16, y4: i16, t_num_mag: u32, t_num_neg: u16, u_num_mag: u32, u_num_neg: u16, den: u32 }
impl LineIntersectParamsFrac {
    fn run(&mut self) -> u16 {
        let x1_mag = i16_mag(self.x1); let x1_neg_f = 1u16 - i16_neg(self.x1);
        let y1_mag = i16_mag(self.y1); let y1_neg_f = 1u16 - i16_neg(self.y1);
        let x3_mag = i16_mag(self.x3); let x3_neg = i16_neg(self.x3); let x3_neg_f = 1u16 - x3_neg;
        let y3_mag = i16_mag(self.y3); let y3_neg = i16_neg(self.y3); let y3_neg_f = 1u16 - y3_neg;
        let x2_mag = i16_mag(self.x2); let x2_neg = i16_neg(self.x2);
        let y2_mag = i16_mag(self.y2); let y2_neg = i16_neg(self.y2);
        let x4_mag = i16_mag(self.x4); let x4_neg = i16_neg(self.x4);
        let y4_mag = i16_mag(self.y4); let y4_neg = i16_neg(self.y4);

        // d1x = x2 - x1
        let mut d1x_mag = 0u32; let mut d1x_neg = 0u16;
        if x2_neg == x1_neg_f { d1x_mag = add_checked_u32(x2_mag, x1_mag); d1x_neg = x2_neg; }
        else if x2_mag >= x1_mag { d1x_mag = x2_mag - x1_mag; d1x_neg = if d1x_mag == 0u32 { 0u16 } else { x2_neg }; }
        else { d1x_mag = x1_mag - x2_mag; d1x_neg = x1_neg_f; }

        // d1y = y2 - y1
        let mut d1y_mag = 0u32; let mut d1y_neg = 0u16;
        if y2_neg == y1_neg_f { d1y_mag = add_checked_u32(y2_mag, y1_mag); d1y_neg = y2_neg; }
        else if y2_mag >= y1_mag { d1y_mag = y2_mag - y1_mag; d1y_neg = if d1y_mag == 0u32 { 0u16 } else { y2_neg }; }
        else { d1y_mag = y1_mag - y2_mag; d1y_neg = y1_neg_f; }

        // d2x = x4 - x3
        let mut d2x_mag = 0u32; let mut d2x_neg = 0u16;
        if x4_neg == x3_neg_f { d2x_mag = add_checked_u32(x4_mag, x3_mag); d2x_neg = x4_neg; }
        else if x4_mag >= x3_mag { d2x_mag = x4_mag - x3_mag; d2x_neg = if d2x_mag == 0u32 { 0u16 } else { x4_neg }; }
        else { d2x_mag = x3_mag - x4_mag; d2x_neg = x3_neg_f; }

        // d2y = y4 - y3
        let mut d2y_mag = 0u32; let mut d2y_neg = 0u16;
        if y4_neg == y3_neg_f { d2y_mag = add_checked_u32(y4_mag, y3_mag); d2y_neg = y4_neg; }
        else if y4_mag >= y3_mag { d2y_mag = y4_mag - y3_mag; d2y_neg = if d2y_mag == 0u32 { 0u16 } else { y4_neg }; }
        else { d2y_mag = y3_mag - y4_mag; d2y_neg = y3_neg_f; }

        // wx = x3 - x1
        let mut wx_mag = 0u32; let mut wx_neg = 0u16;
        if x3_neg == x1_neg_f { wx_mag = add_checked_u32(x3_mag, x1_mag); wx_neg = x3_neg; }
        else if x3_mag >= x1_mag { wx_mag = x3_mag - x1_mag; wx_neg = if wx_mag == 0u32 { 0u16 } else { x3_neg }; }
        else { wx_mag = x1_mag - x3_mag; wx_neg = x1_neg_f; }

        // wy = y3 - y1
        let mut wy_mag = 0u32; let mut wy_neg = 0u16;
        if y3_neg == y1_neg_f { wy_mag = add_checked_u32(y3_mag, y1_mag); wy_neg = y3_neg; }
        else if y3_mag >= y1_mag { wy_mag = y3_mag - y1_mag; wy_neg = if wy_mag == 0u32 { 0u16 } else { y3_neg }; }
        else { wy_mag = y1_mag - y3_mag; wy_neg = y1_neg_f; }

        // den = cross(d1, d2) = d1x*d2y - d1y*d2x
        let dp1_mag = mul_checked_u32(d1x_mag, d2y_mag);
        let dp1_neg = if d1x_neg == d2y_neg { 0u16 } else { 1u16 };
        let dp2_mag = mul_checked_u32(d1y_mag, d2x_mag);
        let dp2_neg = if d1y_neg == d2x_neg { 0u16 } else { 1u16 };
        let dp2_neg_f = if dp2_neg == 0u16 { 1u16 } else { 0u16 };
        let mut den_mag = 0u32; let mut den_neg = 0u16;
        if dp1_neg == dp2_neg_f { den_mag = add_checked_u32(dp1_mag, dp2_mag); den_neg = dp1_neg; }
        else if dp1_mag >= dp2_mag { den_mag = dp1_mag - dp2_mag; den_neg = if den_mag == 0u32 { 0u16 } else { dp1_neg }; }
        else { den_mag = dp2_mag - dp1_mag; den_neg = dp2_neg_f; }

        if den_mag == 0u32 { halt(0xFF06u16); }

        // t_num = cross(w, d2) = wx*d2y - wy*d2x
        let tp1_mag = mul_checked_u32(wx_mag, d2y_mag);
        let tp1_neg = if wx_neg == d2y_neg { 0u16 } else { 1u16 };
        let tp2_mag = mul_checked_u32(wy_mag, d2x_mag);
        let tp2_neg = if wy_neg == d2x_neg { 0u16 } else { 1u16 };
        let tp2_neg_f = if tp2_neg == 0u16 { 1u16 } else { 0u16 };
        let mut t_num_mag = 0u32; let mut t_num_neg = 0u16;
        if tp1_neg == tp2_neg_f { t_num_mag = add_checked_u32(tp1_mag, tp2_mag); t_num_neg = tp1_neg; }
        else if tp1_mag >= tp2_mag { t_num_mag = tp1_mag - tp2_mag; t_num_neg = if t_num_mag == 0u32 { 0u16 } else { tp1_neg }; }
        else { t_num_mag = tp2_mag - tp1_mag; t_num_neg = tp2_neg_f; }

        // u_num = cross(w, d1) = wx*d1y - wy*d1x
        let up1_mag = mul_checked_u32(wx_mag, d1y_mag);
        let up1_neg = if wx_neg == d1y_neg { 0u16 } else { 1u16 };
        let up2_mag = mul_checked_u32(wy_mag, d1x_mag);
        let up2_neg = if wy_neg == d1x_neg { 0u16 } else { 1u16 };
        let up2_neg_f = if up2_neg == 0u16 { 1u16 } else { 0u16 };
        let mut u_num_mag = 0u32; let mut u_num_neg = 0u16;
        if up1_neg == up2_neg_f { u_num_mag = add_checked_u32(up1_mag, up2_mag); u_num_neg = up1_neg; }
        else if up1_mag >= up2_mag { u_num_mag = up1_mag - up2_mag; u_num_neg = if u_num_mag == 0u32 { 0u16 } else { up1_neg }; }
        else { u_num_mag = up2_mag - up1_mag; u_num_neg = up2_neg_f; }

        // Normalize so den is always positive: flip both numerators' signs if den was negative
        // (dividing top and bottom of a fraction by -1 leaves its value unchanged).
        let mut final_t_num_neg = t_num_neg;
        if den_neg == 1u16 { final_t_num_neg = 1u16 - t_num_neg; }
        if t_num_mag == 0u32 { final_t_num_neg = 0u16; }

        let mut final_u_num_neg = u_num_neg;
        if den_neg == 1u16 { final_u_num_neg = 1u16 - u_num_neg; }
        if u_num_mag == 0u32 { final_u_num_neg = 0u16; }

        self.t_num_mag = t_num_mag;
        self.t_num_neg = final_t_num_neg;
        self.u_num_mag = u_num_mag;
        self.u_num_neg = final_u_num_neg;
        self.den = den_mag;
        1u16
    }
}
