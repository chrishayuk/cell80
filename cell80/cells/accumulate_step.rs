//! Running sum + count over a stream of values (sum saturates at 65535). Compose with safe_div(sum, count) for a running mean.
//! tags: running, sum, count, accumulate, stream, stats, mean, average, state
//! entry: Accumulate::run
struct Accumulate { value: u16, sum: u16, count: u16 }
impl Accumulate {
    fn run(&mut self) -> u16 {
        let s = self.sum.wrapping_add(self.value);
        let capped = if s < self.sum { 65535u16 } else { s };
        self.sum = capped;
        self.count = self.count + 1u16;
        self.sum
    }
}
