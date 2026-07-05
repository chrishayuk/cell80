//! Chinese Remainder Theorem for two congruences: the unique x in [0, m1*m2) with x == r1 (mod m1) and x == r2 (mod m2), when m1 and m2 are coprime. Computes the inverse of m1 modulo m2 via an inlined extended Euclidean algorithm (the same one mod_inverse uses — duplicated here rather than called, since a u32 value still can't cross more than one call boundary), then combines it with the standard closed-form x = r1 + m1*((r2-r1)*inv(m1, m2) mod m2).
//! tags: number, modular, modulo, chinese-remainder-theorem, crt, congruence, wide, u32, checked, escalate, aime, number-theory
//! entry: CrtSolvePair::run
//! limits: escalates (halt 0xFF06, out_of_domain) if m1 == 0, m2 == 0, or m1 and m2 aren't coprime; escalates (halt 0xFF05, needs_wider_math) if m1*m2 or an intermediate product overflows u32
struct CrtSolvePair { r1: u32, m1: u32, r2: u32, m2: u32, result: u32 }
impl CrtSolvePair {
    fn run(&mut self) -> u16 {
        if self.m1 == 0u32 || self.m2 == 0u32 { halt(0xFF06u16); }
        let a1 = self.r1 % self.m1;
        let a2 = self.r2 % self.m2;

        let mut old_r = self.m1 % self.m2;
        let mut r = self.m2;
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
                let sum = old_s_mag.wrapping_add(b_mag);
                if sum < old_s_mag { halt(0xFF05u16); }
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
        let mut inv = old_s_mag % self.m2;
        if old_s_neg == 1u16 && inv != 0u32 { inv = self.m2 - inv; }

        let a1_mod_m2 = a1 % self.m2;
        let diff = if a2 >= a1_mod_m2 { a2 - a1_mod_m2 } else { self.m2 - (a1_mod_m2 - a2) };

        let prod = diff.wrapping_mul(inv);
        if diff != 0u32 && prod / diff != inv { halt(0xFF05u16); }
        let t = prod % self.m2;

        let mt = self.m1.wrapping_mul(t);
        if self.m1 != 0u32 && mt / self.m1 != t { halt(0xFF05u16); }
        let x = a1.wrapping_add(mt);
        if x < a1 { halt(0xFF05u16); }

        let modulus = self.m1.wrapping_mul(self.m2);
        if self.m1 != 0u32 && modulus / self.m1 != self.m2 { halt(0xFF05u16); }

        self.result = x % modulus;
        1u16
    }
}
