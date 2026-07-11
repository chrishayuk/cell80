//! Linearly-weighted moving average over a fixed trailing window of the last 8 values (the newest sample weighs 8, the oldest 1 — the classic WMA that reacts faster than simple_moving_average's equal weighting), recomputed by walking the ring each call since every sample's weight changes as newer ones arrive. Self-initializing: until the window fills, the weights are 1..count and the divisor is their sum, so it's never zero.
//! tags: weighted, moving, average, wma, window, sliding, rolling, trailing, recency, stream, state, wide
//! entry: WeightedMovingAverage::run
//! limits: fixed 8-sample trailing window, not caller-configurable; linear 1..8 weights only (exponential smoothing is a different cell shape — it needs no window at all)
struct WeightedMovingAverage { value: u16, window: [u16; 8], head: u16, count: u16, wavg: u16 }
impl WeightedMovingAverage {
    fn run(&mut self) -> u16 {
        self.window[self.head as usize] = self.value;
        self.head = (self.head + 1u16) % 8u16;
        if self.count < 8u16 { self.count = self.count + 1u16; }
        // Oldest sample: at the write head once the ring is full, else slot 0 (the
        // ring hasn't wrapped yet). Weight j+1 for the j-th oldest — newest heaviest.
        let oldest = if self.count == 8u16 { self.head } else { 0u16 };
        let mut num = 0u32;
        let mut den = 0u32;
        let mut j = 0u16;
        while j < self.count {
            let idx = (oldest + j) % 8u16;
            let w = (j + 1u16) as u32;
            num = num + w * (self.window[idx as usize] as u32);
            den = den + w;
            j = j + 1u16;
        }
        self.wavg = (num / den) as u16;
        self.wavg
    }
}
