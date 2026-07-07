//! Running min/max tracker over a stream of values: updates min/max (self-initializing on the first call via `seen`), returns the current range (max - min).
//! tags: running, min, max, range, stream, stats, tracker, state
//! entry: RunningMinMax::run
struct RunningMinMax { value: u16, min: u16, max: u16, seen: u16 }
impl RunningMinMax {
    fn run(&mut self) -> u16 {
        if self.seen == 0u16 {
            self.min = self.value;
            self.max = self.value;
            self.seen = 1u16;
        } else {
            if self.value < self.min { self.min = self.value; }
            if self.value > self.max { self.max = self.value; }
        }
        self.max - self.min
    }
}
