//! Twice the area of a quadrilateral from four integer vertices (x1,y1)..(x4,y4), generalizing shoelace_area_x2's triangle formula to |x1*(y2-y4) + x2*(y3-y1) + x3*(y4-y2) + x4*(y1-y3)| — always an integer. Coordinates are unsigned; the four (y-difference)*(x-coordinate) terms are combined as sign-magnitude values inline (no shared smag_* subroutine call — a u32 value still can't cross more than one call boundary), the same pattern shoelace_area_x2 uses, extended to a fourth term.
//! tags: geometry, distance, area, quadrilateral, polygon, shoelace, coordinate, wide, u32, checked, escalate
//! entry: ShoelaceAreaX2Quad::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if any term or the running sum overflows u32
struct ShoelaceAreaX2Quad { x1: u16, y1: u16, x2: u16, y2: u16, x3: u16, y3: u16, x4: u16, y4: u16, result: u32 }
impl ShoelaceAreaX2Quad {
    fn run(&mut self) -> u16 {
        let mag_d1 = if self.y2 >= self.y4 { self.y2 - self.y4 } else { self.y4 - self.y2 };
        let neg_d1 = if self.y2 >= self.y4 { 0u16 } else { 1u16 };
        let mag_d2 = if self.y3 >= self.y1 { self.y3 - self.y1 } else { self.y1 - self.y3 };
        let neg_d2 = if self.y3 >= self.y1 { 0u16 } else { 1u16 };
        let mag_d3 = if self.y4 >= self.y2 { self.y4 - self.y2 } else { self.y2 - self.y4 };
        let neg_d3 = if self.y4 >= self.y2 { 0u16 } else { 1u16 };
        let mag_d4 = if self.y1 >= self.y3 { self.y1 - self.y3 } else { self.y3 - self.y1 };
        let neg_d4 = if self.y1 >= self.y3 { 0u16 } else { 1u16 };

        let x1w = self.x1 as u32;
        let md1 = mag_d1 as u32;
        let p1 = x1w.wrapping_mul(md1);
        if x1w != 0u32 && p1 / x1w != md1 { halt(0xFF05u16); }
        let mag_t1 = p1;
        let neg_t1 = neg_d1;

        let x2w = self.x2 as u32;
        let md2 = mag_d2 as u32;
        let p2 = x2w.wrapping_mul(md2);
        if x2w != 0u32 && p2 / x2w != md2 { halt(0xFF05u16); }
        let mag_t2 = p2;
        let neg_t2 = neg_d2;

        let x3w = self.x3 as u32;
        let md3 = mag_d3 as u32;
        let p3 = x3w.wrapping_mul(md3);
        if x3w != 0u32 && p3 / x3w != md3 { halt(0xFF05u16); }
        let mag_t3 = p3;
        let neg_t3 = neg_d3;

        let x4w = self.x4 as u32;
        let md4 = mag_d4 as u32;
        let p4 = x4w.wrapping_mul(md4);
        if x4w != 0u32 && p4 / x4w != md4 { halt(0xFF05u16); }
        let mag_t4 = p4;
        let neg_t4 = neg_d4;

        let mut mag_s = 0u32;
        let mut neg_s = 0u16;
        if neg_t1 == neg_t2 {
            let s = mag_t1.wrapping_add(mag_t2);
            if s < mag_t1 { halt(0xFF05u16); }
            mag_s = s;
            neg_s = neg_t1;
        } else if mag_t1 >= mag_t2 {
            mag_s = mag_t1 - mag_t2;
            neg_s = neg_t1;
        } else {
            mag_s = mag_t2 - mag_t1;
            neg_s = neg_t2;
        }

        if neg_s == neg_t3 {
            let s = mag_s.wrapping_add(mag_t3);
            if s < mag_s { halt(0xFF05u16); }
            mag_s = s;
            neg_s = neg_t3;
        } else if mag_s >= mag_t3 {
            mag_s = mag_s - mag_t3;
        } else {
            mag_s = mag_t3 - mag_s;
            neg_s = neg_t3;
        }

        let mut mag_final = 0u32;
        if neg_s == neg_t4 {
            let s = mag_s.wrapping_add(mag_t4);
            if s < mag_s { halt(0xFF05u16); }
            mag_final = s;
        } else if mag_s >= mag_t4 {
            mag_final = mag_s - mag_t4;
        } else {
            mag_final = mag_t4 - mag_s;
        }

        self.result = mag_final;
        1u16
    }
}
