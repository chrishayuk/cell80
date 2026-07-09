//! Convert a t-statistic to effect size r: r = t / sqrt(t^2 + df), a Q8.8 fixed-point value always bounded to [-1, 1] (t^2 <= t^2+df, so |t|/sqrt(t^2+df) <= 1 by construction). Computed by scaling t^2+df up by 256 before taking an integer square root (the same precision-preserving order q_sqrt uses -- sqrt first, divide last, loses far less precision than dividing by a truncated integer sqrt directly), then dividing a further-scaled numerator by that root in one step.
//! tags: statistics, effect-size, t-statistic, hypothesis-test, fixed-point, q8.8, wide, u32, checked, escalate
//! entry: EffectSizeR::run
//! scale: 8
//! limits: escalates (halt 0xFF05, needs_wider_math) if t^2, t^2+df, (t^2+df)*256, or the scaled numerator overflows u32 -- realistic |t| into the low thousands with df into the hundreds of thousands stays well inside this bound
struct EffectSizeR { t: i16, df: u32, r_mag: u16, r_neg: u16 }
impl EffectSizeR {
    fn run(&mut self) -> u16 {
        let t_neg = if self.t < 0i16 { 1u16 } else { 0u16 };
        let t_mag = if self.t < 0i16 { (0u16.wrapping_sub(self.t as u16)) as u32 } else { self.t as u16 as u32 };
        let t2 = mul_checked_u32(t_mag, t_mag);
        let mag = add_checked_u32(t2, self.df);
        let scaled_mag = mul_checked_u32(mag, 256u32);

        let mut val = scaled_mag;
        let mut res = 0u32;
        let mut bit = 1u32 << 30u32;
        while bit > val { bit = bit >> 2u32; }
        while bit != 0u32 {
            if val >= res + bit {
                val = val - (res + bit);
                res = (res >> 1u32) + bit;
            } else {
                res = res >> 1u32;
            }
            bit = bit >> 2u32;
        }
        let s2 = res;

        if s2 == 0u32 {
            self.r_mag = 0u16;
            self.r_neg = 0u16;
            return 1u16;
        }
        let num = mul_checked_u32(t_mag, 4096u32);
        let mut r_q8 = num / s2;
        if r_q8 > 256u32 { r_q8 = 256u32; }
        self.r_mag = r_q8 as u16;
        self.r_neg = t_neg;
        1u16
    }
}
