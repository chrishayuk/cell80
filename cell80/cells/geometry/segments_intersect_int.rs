//! Boolean predicate: do finite segments (x1,y1)-(x2,y2) and (x3,y3)-(x4,y4) properly intersect, via the standard four-orientation-sign-test algorithm (each orientation is a 2D cross-product sign, tracked as an exact sign-magnitude pair since the dialect has no i32) plus the collinear-overlap edge case (a zero orientation checked against the other segment's bounding box) -- distinct from aabb_intersect (rectangle overlap, no segment direction at all) and from geom_line_intersection (solves for the infinite lines' crossing point, an exact fraction, which reduces to matrix_solve_2x2) since this only asks whether the two *finite* segments cross, as a 0/1 verdict, never a coordinate.
//! tags: geometry, segment, intersect, orientation, cross-product, predicate, collinear, coordinate, signed
//! entry: SegmentsIntersect::run
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct SegmentsIntersect { x1: i16, y1: i16, x2: i16, y2: i16, x3: i16, y3: i16, x4: i16, y4: i16, result: u16 }
impl SegmentsIntersect {
    fn run(&mut self) -> u16 {
        let x1m = i16_mag(self.x1); let x1n = i16_neg(self.x1);
        let y1m = i16_mag(self.y1); let y1n = i16_neg(self.y1);
        let x2m = i16_mag(self.x2); let x2n = i16_neg(self.x2);
        let y2m = i16_mag(self.y2); let y2n = i16_neg(self.y2);
        let x3m = i16_mag(self.x3); let x3n = i16_neg(self.x3);
        let y3m = i16_mag(self.y3); let y3n = i16_neg(self.y3);
        let x4m = i16_mag(self.x4); let x4n = i16_neg(self.x4);
        let y4m = i16_mag(self.y4); let y4n = i16_neg(self.y4);

        // Every coordinate difference below is a plain "Q - P" sign-magnitude combine (the
        // linear_solve_1var / linear_eq_holds shape): same sign adds (bounded by 32768+32768,
        // so a plain u32 add never risks overflow), opposite sign subtracts the smaller
        // magnitude from the larger and takes the larger operand's sign.
        let x3nf = 1u16 - x3n;
        let mut d34x_mag = 0u32; let mut d34x_neg = 0u16;
        if x4n == x3nf { d34x_mag = x4m + x3m; d34x_neg = x4n; }
        else if x4m >= x3m { d34x_mag = x4m - x3m; d34x_neg = if d34x_mag == 0u32 { 0u16 } else { x4n }; }
        else { d34x_mag = x3m - x4m; d34x_neg = x3nf; }

        let y3nf = 1u16 - y3n;
        let mut d34y_mag = 0u32; let mut d34y_neg = 0u16;
        if y4n == y3nf { d34y_mag = y4m + y3m; d34y_neg = y4n; }
        else if y4m >= y3m { d34y_mag = y4m - y3m; d34y_neg = if d34y_mag == 0u32 { 0u16 } else { y4n }; }
        else { d34y_mag = y3m - y4m; d34y_neg = y3nf; }

        let x1nf = 1u16 - x1n;
        let mut d12x_mag = 0u32; let mut d12x_neg = 0u16;
        if x2n == x1nf { d12x_mag = x2m + x1m; d12x_neg = x2n; }
        else if x2m >= x1m { d12x_mag = x2m - x1m; d12x_neg = if d12x_mag == 0u32 { 0u16 } else { x2n }; }
        else { d12x_mag = x1m - x2m; d12x_neg = x1nf; }

        let y1nf = 1u16 - y1n;
        let mut d12y_mag = 0u32; let mut d12y_neg = 0u16;
        if y2n == y1nf { d12y_mag = y2m + y1m; d12y_neg = y2n; }
        else if y2m >= y1m { d12y_mag = y2m - y1m; d12y_neg = if d12y_mag == 0u32 { 0u16 } else { y2n }; }
        else { d12y_mag = y1m - y2m; d12y_neg = y1nf; }

        // d1 = orient(P3, P4, P1) = cross((P4-P3), (P1-P3))
        let mut c1x_mag = 0u32; let mut c1x_neg = 0u16;
        if x1n == x3nf { c1x_mag = x1m + x3m; c1x_neg = x1n; }
        else if x1m >= x3m { c1x_mag = x1m - x3m; c1x_neg = if c1x_mag == 0u32 { 0u16 } else { x1n }; }
        else { c1x_mag = x3m - x1m; c1x_neg = x3nf; }
        let mut c1y_mag = 0u32; let mut c1y_neg = 0u16;
        if y1n == y3nf { c1y_mag = y1m + y3m; c1y_neg = y1n; }
        else if y1m >= y3m { c1y_mag = y1m - y3m; c1y_neg = if c1y_mag == 0u32 { 0u16 } else { y1n }; }
        else { c1y_mag = y3m - y1m; c1y_neg = y3nf; }
        let t1a_mag = d34x_mag * c1y_mag;
        let t1a_neg = if d34x_neg == c1y_neg { 0u16 } else { 1u16 };
        let t1b_mag = d34y_mag * c1x_mag;
        let t1b_neg = if d34y_neg == c1x_neg { 0u16 } else { 1u16 };
        // d1 = t1a - t1b: opposite signs of t1a/t1b add (magnitudes only matter for the
        // zero check, since we only need d1's sign classification, not its exact value);
        // same sign subtracts, and the smaller-magnitude side flips to the *other* sign.
        let t1b_neg_f = 1u16 - t1b_neg;
        let mut d1_zero = 0u16; let mut d1_neg = 0u16;
        if t1a_neg == t1b_neg_f {
            if t1a_mag == 0u32 && t1b_mag == 0u32 { d1_zero = 1u16; } else { d1_neg = t1a_neg; }
        } else if t1a_mag == t1b_mag { d1_zero = 1u16; }
        else if t1a_mag > t1b_mag { d1_neg = t1a_neg; }
        else { d1_neg = t1b_neg_f; }

        // d2 = orient(P3, P4, P2) = cross((P4-P3), (P2-P3))
        let mut c2x_mag = 0u32; let mut c2x_neg = 0u16;
        if x2n == x3nf { c2x_mag = x2m + x3m; c2x_neg = x2n; }
        else if x2m >= x3m { c2x_mag = x2m - x3m; c2x_neg = if c2x_mag == 0u32 { 0u16 } else { x2n }; }
        else { c2x_mag = x3m - x2m; c2x_neg = x3nf; }
        let mut c2y_mag = 0u32; let mut c2y_neg = 0u16;
        if y2n == y3nf { c2y_mag = y2m + y3m; c2y_neg = y2n; }
        else if y2m >= y3m { c2y_mag = y2m - y3m; c2y_neg = if c2y_mag == 0u32 { 0u16 } else { y2n }; }
        else { c2y_mag = y3m - y2m; c2y_neg = y3nf; }
        let t2a_mag = d34x_mag * c2y_mag;
        let t2a_neg = if d34x_neg == c2y_neg { 0u16 } else { 1u16 };
        let t2b_mag = d34y_mag * c2x_mag;
        let t2b_neg = if d34y_neg == c2x_neg { 0u16 } else { 1u16 };
        let t2b_neg_f = 1u16 - t2b_neg;
        let mut d2_zero = 0u16; let mut d2_neg = 0u16;
        if t2a_neg == t2b_neg_f {
            if t2a_mag == 0u32 && t2b_mag == 0u32 { d2_zero = 1u16; } else { d2_neg = t2a_neg; }
        } else if t2a_mag == t2b_mag { d2_zero = 1u16; }
        else if t2a_mag > t2b_mag { d2_neg = t2a_neg; }
        else { d2_neg = t2b_neg_f; }

        // d3 = orient(P1, P2, P3) = cross((P2-P1), (P3-P1))
        let mut c3x_mag = 0u32; let mut c3x_neg = 0u16;
        if x3n == x1nf { c3x_mag = x3m + x1m; c3x_neg = x3n; }
        else if x3m >= x1m { c3x_mag = x3m - x1m; c3x_neg = if c3x_mag == 0u32 { 0u16 } else { x3n }; }
        else { c3x_mag = x1m - x3m; c3x_neg = x1nf; }
        let mut c3y_mag = 0u32; let mut c3y_neg = 0u16;
        if y3n == y1nf { c3y_mag = y3m + y1m; c3y_neg = y3n; }
        else if y3m >= y1m { c3y_mag = y3m - y1m; c3y_neg = if c3y_mag == 0u32 { 0u16 } else { y3n }; }
        else { c3y_mag = y1m - y3m; c3y_neg = y1nf; }
        let t3a_mag = d12x_mag * c3y_mag;
        let t3a_neg = if d12x_neg == c3y_neg { 0u16 } else { 1u16 };
        let t3b_mag = d12y_mag * c3x_mag;
        let t3b_neg = if d12y_neg == c3x_neg { 0u16 } else { 1u16 };
        let t3b_neg_f = 1u16 - t3b_neg;
        let mut d3_zero = 0u16; let mut d3_neg = 0u16;
        if t3a_neg == t3b_neg_f {
            if t3a_mag == 0u32 && t3b_mag == 0u32 { d3_zero = 1u16; } else { d3_neg = t3a_neg; }
        } else if t3a_mag == t3b_mag { d3_zero = 1u16; }
        else if t3a_mag > t3b_mag { d3_neg = t3a_neg; }
        else { d3_neg = t3b_neg_f; }

        // d4 = orient(P1, P2, P4) = cross((P2-P1), (P4-P1))
        let mut c4x_mag = 0u32; let mut c4x_neg = 0u16;
        if x4n == x1nf { c4x_mag = x4m + x1m; c4x_neg = x4n; }
        else if x4m >= x1m { c4x_mag = x4m - x1m; c4x_neg = if c4x_mag == 0u32 { 0u16 } else { x4n }; }
        else { c4x_mag = x1m - x4m; c4x_neg = x1nf; }
        let mut c4y_mag = 0u32; let mut c4y_neg = 0u16;
        if y4n == y1nf { c4y_mag = y4m + y1m; c4y_neg = y4n; }
        else if y4m >= y1m { c4y_mag = y4m - y1m; c4y_neg = if c4y_mag == 0u32 { 0u16 } else { y4n }; }
        else { c4y_mag = y1m - y4m; c4y_neg = y1nf; }
        let t4a_mag = d12x_mag * c4y_mag;
        let t4a_neg = if d12x_neg == c4y_neg { 0u16 } else { 1u16 };
        let t4b_mag = d12y_mag * c4x_mag;
        let t4b_neg = if d12y_neg == c4x_neg { 0u16 } else { 1u16 };
        let t4b_neg_f = 1u16 - t4b_neg;
        let mut d4_zero = 0u16; let mut d4_neg = 0u16;
        if t4a_neg == t4b_neg_f {
            if t4a_mag == 0u32 && t4b_mag == 0u32 { d4_zero = 1u16; } else { d4_neg = t4a_neg; }
        } else if t4a_mag == t4b_mag { d4_zero = 1u16; }
        else if t4a_mag > t4b_mag { d4_neg = t4a_neg; }
        else { d4_neg = t4b_neg_f; }

        // Collinear-overlap edge case: a zero orientation puts the third point on the infinite
        // line through the other segment; a bounding-box containment check confirms it also
        // falls on that segment's finite extent, not just the line through it.
        let mut minx34 = self.x3; if self.x4 < minx34 { minx34 = self.x4; }
        let mut maxx34 = self.x3; if self.x4 > maxx34 { maxx34 = self.x4; }
        let mut miny34 = self.y3; if self.y4 < miny34 { miny34 = self.y4; }
        let mut maxy34 = self.y3; if self.y4 > maxy34 { maxy34 = self.y4; }
        let on1 = ((minx34 <= self.x1) && (self.x1 <= maxx34) && (miny34 <= self.y1) && (self.y1 <= maxy34)) as u16;
        let on2 = ((minx34 <= self.x2) && (self.x2 <= maxx34) && (miny34 <= self.y2) && (self.y2 <= maxy34)) as u16;

        let mut minx12 = self.x1; if self.x2 < minx12 { minx12 = self.x2; }
        let mut maxx12 = self.x1; if self.x2 > maxx12 { maxx12 = self.x2; }
        let mut miny12 = self.y1; if self.y2 < miny12 { miny12 = self.y2; }
        let mut maxy12 = self.y1; if self.y2 > maxy12 { maxy12 = self.y2; }
        let on3 = ((minx12 <= self.x3) && (self.x3 <= maxx12) && (miny12 <= self.y3) && (self.y3 <= maxy12)) as u16;
        let on4 = ((minx12 <= self.x4) && (self.x4 <= maxx12) && (miny12 <= self.y4) && (self.y4 <= maxy12)) as u16;

        let hit = ((d1_zero == 0u16) && (d2_zero == 0u16) && (d1_neg != d2_neg)
            && (d3_zero == 0u16) && (d4_zero == 0u16) && (d3_neg != d4_neg))
            || ((d1_zero == 1u16) && (on1 == 1u16))
            || ((d2_zero == 1u16) && (on2 == 1u16))
            || ((d3_zero == 1u16) && (on3 == 1u16))
            || ((d4_zero == 1u16) && (on4 == 1u16));
        self.result = hit as u16;
        self.result
    }
}
