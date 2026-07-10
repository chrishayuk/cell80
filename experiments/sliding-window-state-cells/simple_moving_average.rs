//! Simple moving average over a fixed trailing window of the last 8 values — a true sliding window (subtracts the value leaving the window each step), distinct from accumulate_step/running_variance_step which are cumulative over the whole stream and never forget a sample. Self-initializing: the average is over however many samples have arrived until the window fills, then always over exactly the last 8.
//! tags: moving, average, sma, window, sliding, rolling, trailing, stream, state, wide
//! entry: SimpleMovingAverage::run
//! limits: fixed 8-sample trailing window, not caller-configurable; the divisor is min(samples_seen, 8), so it's never zero
struct SimpleMovingAverage { value: u16, window: [u16; 8], head: u16, count: u16, sum: u32, avg: u16 }
impl SimpleMovingAverage {
    fn run(&mut self) -> u16 {
        let full = self.count == 8u16;
        let evict = if full { self.window[self.head as usize] as u32 } else { 0u32 };
        self.window[self.head as usize] = self.value;
        self.sum = self.sum - evict + (self.value as u32);
        if !full { self.count = self.count + 1u16; }
        self.head = (self.head + 1u16) % 8u16;
        self.avg = (self.sum / (self.count as u32)) as u16;
        self.avg
    }
}
