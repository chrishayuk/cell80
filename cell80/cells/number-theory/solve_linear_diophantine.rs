//! A particular integer solution (x, y) to a*x + b*y = c via the extended Euclidean algorithm: builds gcd(a, b) plus Bezout coefficients x0, y0 with a*x0 + b*y0 == gcd(a, b) (the same chain extended_gcd computes), then scales by c/gcd(a, b) -- escalating when gcd(a, b) does not evenly divide c, i.e. when no integer solution exists for the given target c.
//! tags: number, diophantine, linear, equation, bezout, extended-euclidean, gcd, wide, u32, checked, escalate, out_of_domain, number-theory
//! entry: LinearDiophantine::run
//! limits: escalates (halt 0xFF06, out_of_domain) if gcd(a, b) does not evenly divide c (no integer solution exists, including a == b == 0 with c != 0); escalates (halt 0xFF05, needs_wider_math) on Bezout-chain or final c/gcd scaling overflow
struct LinearDiophantine { a: u32, b: u32, c: u32, x_mag: u32, x_neg: u16, y_mag: u32, y_neg: u16 }
impl LinearDiophantine {
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
        let x0_neg = if old_s_mag == 0u32 { 0u16 } else { old_s_neg };
        let y0_neg = if old_t_mag == 0u32 { 0u16 } else { old_t_neg };
        let g = old_r;

        if g == 0u32 {
            if self.c != 0u32 { halt(0xFF06u16); }
            self.x_mag = 0u32;
            self.x_neg = 0u16;
            self.y_mag = 0u32;
            self.y_neg = 0u16;
            return 1u16;
        }
        if self.c % g != 0u32 { halt(0xFF06u16); }
        let k = self.c / g;

        let x_mag = old_s_mag.wrapping_mul(k);
        if old_s_mag != 0u32 && x_mag / old_s_mag != k { halt(0xFF05u16); }
        let y_mag = old_t_mag.wrapping_mul(k);
        if old_t_mag != 0u32 && y_mag / old_t_mag != k { halt(0xFF05u16); }

        let x_neg_final = if x_mag == 0u32 { 0u16 } else { x0_neg };
        let y_neg_final = if y_mag == 0u32 { 0u16 } else { y0_neg };
        self.x_mag = x_mag;
        self.x_neg = x_neg_final;
        self.y_mag = y_mag;
        self.y_neg = y_neg_final;
        1u16
    }
}
