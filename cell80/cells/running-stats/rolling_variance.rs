//! Windowed (population) variance over the last 8 values — the sliding-window sibling of running_variance_step, which is cumulative over the whole stream and never forgets a sample; this one recomputes the squared-deviation sum by walking the ring each call, so an old outlier ages out after 8 samples. Self-initializing (variance over however many samples have arrived until the window fills); the result lives in the wide `var` state field, returns 1 like its running sibling — compose or read `var` by name.
//! tags: rolling, variance, window, sliding, trailing, stats, stream, state, wide, u32, checked, escalate
//! entry: RollingVariance::run
//! limits: fixed 8-sample trailing window, not caller-configurable; escalates (halt 0xFF05, needs_wider_math) if the squared-deviation sum overflows u32 — guaranteed safe while the window's spread stays under ~23169, the full-u16-spread worst case needs a u64 it doesn't have
struct RollingVariance { value: u16, window: [u16; 8], head: u16, count: u16, sum: u32, var: u32 }
impl RollingVariance {
    fn run(&mut self) -> u16 {
        let full = self.count == 8u16;
        let evict = if full { self.window[self.head as usize] as u32 } else { 0u32 };
        self.window[self.head as usize] = self.value;
        self.sum = self.sum - evict + (self.value as u32);
        if !full { self.count = self.count + 1u16; }
        self.head = (self.head + 1u16) % 8u16;

        // Truncated integer mean of the live window, then the squared-deviation
        // walk. Each square fits u32 (65535^2 < u32::MAX); the SUM of squares is
        // the checked step.
        let mean = self.sum / (self.count as u32);
        let oldest = if self.count == 8u16 { self.head } else { 0u16 };
        let mut ssd = 0u32;
        let mut j = 0u16;
        while j < self.count {
            let idx = (oldest + j) % 8u16;
            let v = self.window[idx as usize] as u32;
            let dev = if v >= mean { v - mean } else { mean - v };
            ssd = add_checked_u32(ssd, dev * dev);
            j = j + 1u16;
        }
        self.var = ssd / (self.count as u32);
        1u16
    }
}
