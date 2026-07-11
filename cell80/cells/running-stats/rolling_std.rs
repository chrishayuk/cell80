//! Windowed standard deviation over the last 8 values: rolling_variance's squared-deviation walk over the same ring, then floor(sqrt(variance)) via the branch-free bitwise integer square root running_stddev_step uses, inlined for the same reason (u32 magnitudes are state-cell-local, never crossing a call boundary). The sliding sibling of running_stddev_step — an old outlier ages out after 8 samples instead of haunting the stream forever.
//! tags: rolling, stddev, standard-deviation, window, sliding, trailing, stats, stream, state, wide, u32, checked, escalate, sqrt
//! entry: RollingStd::run
//! limits: fixed 8-sample trailing window, not caller-configurable; escalates (halt 0xFF05, needs_wider_math) if the squared-deviation sum overflows u32 — guaranteed safe while the window's spread stays under ~23169, same wall as rolling_variance
struct RollingStd { value: u16, window: [u16; 8], head: u16, count: u16, sum: u32, std: u16 }
impl RollingStd {
    fn run(&mut self) -> u16 {
        let full = self.count == 8u16;
        let evict = if full { self.window[self.head as usize] as u32 } else { 0u32 };
        self.window[self.head as usize] = self.value;
        self.sum = self.sum - evict + (self.value as u32);
        if !full { self.count = self.count + 1u16; }
        self.head = (self.head + 1u16) % 8u16;

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
        let variance = ssd / (self.count as u32);

        // Branch-free bitwise integer square root (running_stddev_step's inline).
        let mut val = variance;
        let mut res = 0u32;
        let mut bit = 1u32 << 30u32;
        while bit > val {
            bit = bit >> 2u32;
        }
        while bit != 0u32 {
            if val >= res + bit {
                val = val - (res + bit);
                res = (res >> 1u32) + bit;
            } else {
                res = res >> 1u32;
            }
            bit = bit >> 2u32;
        }
        let std = res as u16;
        self.std = std;
        std
    }
}
