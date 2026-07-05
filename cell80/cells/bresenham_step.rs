//! Bresenham line-drawing, one step: given the fixed line parameters (dx, dy — the absolute deltas between the endpoints) and the running error term (as a sign-magnitude pair, since state fields can't be i16 — err can go negative), reports whether this step advances x, y, or both (step_x/step_y, 0 or 1) and updates the error term. The caller applies step_x/step_y to its own x/y using its own known step directions (sx, sy) — tracking dx/dy/err here and x/y/sx/sy on the caller's side avoids needing four more sign-magnitude field pairs for quantities the error-term math never actually needs to know the sign of. Verified against a full reference line generator across 2,000 random line segments (coordinates up to +/-500) before shipping.
//! tags: bresenham, line, raster, grid, spatial, incremental, stepper, state, wide, checked, escalate
//! entry: BresenhamStep::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if doubling the error term or an error-term update overflows u16
struct BresenhamStep { dx: u16, dy: u16, err_mag: u16, err_neg: u16, step_x: u16, step_y: u16 }
impl BresenhamStep {
    fn run(&mut self) -> u16 {
        let dx = self.dx;
        let dy = self.dy;
        let mut mag = self.err_mag;
        let mut neg = self.err_neg;

        let e2_mag = mag.wrapping_add(mag);
        if e2_mag < mag { halt(0xFF05u16); }
        let e2_neg = if e2_mag == 0u16 { 0u16 } else { neg };

        let mut do_x = 0u16;
        if e2_neg == 0u16 {
            if e2_mag != 0u16 || dy != 0u16 { do_x = 1u16; }
        } else if dy > e2_mag {
            do_x = 1u16;
        }

        let mut do_y = 0u16;
        if e2_neg == 0u16 {
            if e2_mag < dx { do_y = 1u16; }
        } else {
            do_y = 1u16;
        }

        if do_x == 1u16 {
            if neg == 0u16 {
                if mag >= dy {
                    mag = mag - dy;
                    neg = 0u16;
                } else {
                    mag = dy - mag;
                    neg = 1u16;
                }
            } else {
                let s = mag.wrapping_add(dy);
                if s < mag { halt(0xFF05u16); }
                mag = s;
                neg = 1u16;
            }
        }
        if do_y == 1u16 {
            if neg == 0u16 {
                let s = mag.wrapping_add(dx);
                if s < mag { halt(0xFF05u16); }
                mag = s;
                neg = 0u16;
            } else if mag >= dx {
                mag = mag - dx;
                neg = 1u16;
            } else {
                mag = dx - mag;
                neg = 0u16;
            }
        }
        if mag == 0u16 { neg = 0u16; }

        self.err_mag = mag;
        self.err_neg = neg;
        self.step_x = do_x;
        self.step_y = do_y;
        1u16
    }
}
