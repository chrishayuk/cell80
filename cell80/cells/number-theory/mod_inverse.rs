//! Modular multiplicative inverse of a mod m: the x in [0, m) with a*x == 1 (mod m), via the iterative extended Euclidean algorithm. The Bezout coefficient tracked along the way can go negative, so it's carried as a sign-magnitude pair inline (no shared smag_* subroutine call — a u32 value still can't cross more than one call boundary), the same convention smag_add/pow_mod_u32 use.
//! tags: inverse, reciprocal, euclidean, bezout, gcd, number, modular, modulo, wide, u32, checked, escalate, aime, number-theory
//! entry: ModInverse::run
//! limits: escalates (halt 0xFF06, out_of_domain) if m == 0 or gcd(a, m) != 1 (no inverse exists); escalates (halt 0xFF05, needs_wider_math) on the rare intermediate overflow
struct ModInverse { a: u32, m: u32, result: u32 }
impl ModInverse {
    fn run(&mut self) -> u16 {
        if self.m == 0u32 { halt(0xFF06u16); }
        let mut old_r = self.a % self.m;
        let mut r = self.m;
        let mut old_s_mag = 1u32;
        let mut old_s_neg = 0u16;
        let mut s_mag = 0u32;
        let mut s_neg = 0u16;
        while r != 0u32 {
            let q = old_r / r;
            let new_r = old_r - q * r;
            old_r = r;
            r = new_r;

            let qs_mag = q.wrapping_mul(s_mag);
            if q != 0u32 && qs_mag / q != s_mag { halt(0xFF05u16); }
            let qs_neg = if qs_mag == 0u32 { 0u16 } else { s_neg };
            let b_mag = qs_mag;
            let b_neg = if qs_neg == 0u16 { 1u16 } else { 0u16 };

            let mut new_mag = 0u32;
            let mut new_neg = 0u16;
            if old_s_neg == b_neg {
                let sum = add_checked_u32(old_s_mag, b_mag);
                new_mag = sum;
                new_neg = old_s_neg;
            } else if old_s_mag >= b_mag {
                new_mag = old_s_mag - b_mag;
                new_neg = old_s_neg;
            } else {
                new_mag = b_mag - old_s_mag;
                new_neg = b_neg;
            }

            old_s_mag = s_mag;
            old_s_neg = s_neg;
            s_mag = new_mag;
            s_neg = new_neg;
        }
        if old_r != 1u32 { halt(0xFF06u16); }
        let mut x = old_s_mag % self.m;
        if old_s_neg == 1u16 && x != 0u32 { x = self.m - x; }
        self.result = x;
        1u16
    }
}
