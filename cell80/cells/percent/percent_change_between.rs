//! Direction-agnostic percent-scale change between two values (before and after, either order): pct_mag = |after-before|*100/before (saturating at 65535), pct_neg = 1 if after < before else 0, 0/0 if before == 0 -- the percent pack's own before/after pair, at u16 width and the pack's saturate (not halt) convention, distinct from money-bps's bps_change_between (bps scale, u32, halts on before == 0).
//! tags: percent, percentage, change, rate, delta, direction, before, after, sign-magnitude, saturate
//! entry: PercentChangeBetween::run
struct PercentChangeBetween { before: u16, after: u16, pct_mag: u16, pct_neg: u16 }
impl PercentChangeBetween {
    fn run(&mut self) -> u16 {
        let mut mag = 0u16;
        let mut neg = 0u16;
        if self.before != 0u16 {
            neg = (self.after < self.before) as u16;
            let diff = iabs_diff(self.after, self.before);
            let q = diff as u32 * 100u32 / self.before as u32;
            mag = if (q >> 16u32) as u16 != 0u16 { 65535u16 } else { q as u16 };
        }
        self.pct_mag = mag;
        self.pct_neg = neg;
        1u16
    }
}
