//! Extended Euclidean algorithm: gcd(a, b) plus the Bezout coefficients x, y with a*x + b*y == gcd(a, b). mod_inverse and crt_solve_pair each inline one Bezout chain internally already (a u32 value still can't cross more than one call boundary, so there's no shared subroutine to call) -- this is the standalone two-chain version those two only compute half of.
//! tags: number, gcd, euclidean, bezout, extended, coefficients, modular, wide, u32, checked
//! entry: ExtendedGcd::run
//! limits: escalates (halt 0xFF05, needs_wider_math) on the rare intermediate overflow
struct ExtendedGcd { a: u32, b: u32, gcd: u32, x_mag: u32, x_neg: u16, y_mag: u32, y_neg: u16 }
impl ExtendedGcd {
    fn run(&mut self) -> u16 {
        let mut old_r = self.a;
        let mut r = self.b;
        let mut old_s_mag = 1u32;
        let mut old_s_neg = 0u16;
        let mut s_mag = 0u32;
        let mut s_neg = 0u16;
        let mut old_t_mag = 0u32;
        let mut old_t_neg = 0u16;
        let mut t_mag = 1u32;
        let mut t_neg = 0u16;
        while r != 0u32 {
            let q = old_r / r;
            let new_r = old_r - q * r;
            old_r = r;
            r = new_r;

            let qs_mag = q.wrapping_mul(s_mag);
            if q != 0u32 && qs_mag / q != s_mag { halt(0xFF05u16); }
            let qs_neg = if qs_mag == 0u32 { 0u16 } else { s_neg };
            let sb_neg = if qs_neg == 0u16 { 1u16 } else { 0u16 };
            let mut new_s_mag = 0u32;
            let mut new_s_neg = 0u16;
            if old_s_neg == sb_neg {
                new_s_mag = add_checked_u32(old_s_mag, qs_mag);
                new_s_neg = old_s_neg;
            } else if old_s_mag >= qs_mag {
                new_s_mag = old_s_mag - qs_mag;
                new_s_neg = old_s_neg;
            } else {
                new_s_mag = qs_mag - old_s_mag;
                new_s_neg = sb_neg;
            }
            old_s_mag = s_mag;
            old_s_neg = s_neg;
            s_mag = new_s_mag;
            s_neg = new_s_neg;

            let qt_mag = q.wrapping_mul(t_mag);
            if q != 0u32 && qt_mag / q != t_mag { halt(0xFF05u16); }
            let qt_neg = if qt_mag == 0u32 { 0u16 } else { t_neg };
            let tb_neg = if qt_neg == 0u16 { 1u16 } else { 0u16 };
            let mut new_t_mag = 0u32;
            let mut new_t_neg = 0u16;
            if old_t_neg == tb_neg {
                new_t_mag = add_checked_u32(old_t_mag, qt_mag);
                new_t_neg = old_t_neg;
            } else if old_t_mag >= qt_mag {
                new_t_mag = old_t_mag - qt_mag;
                new_t_neg = old_t_neg;
            } else {
                new_t_mag = qt_mag - old_t_mag;
                new_t_neg = tb_neg;
            }
            old_t_mag = t_mag;
            old_t_neg = t_neg;
            t_mag = new_t_mag;
            t_neg = new_t_neg;
        }
        let x_neg_final = if old_s_mag == 0u32 { 0u16 } else { old_s_neg };
        let y_neg_final = if old_t_mag == 0u32 { 0u16 } else { old_t_neg };
        self.gcd = old_r;
        self.x_mag = old_s_mag;
        self.x_neg = x_neg_final;
        self.y_mag = old_t_mag;
        self.y_neg = y_neg_final;
        1u16
    }
}
