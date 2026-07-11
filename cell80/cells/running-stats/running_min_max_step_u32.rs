//! Wide sibling of running_min_max_step for a stream of u32 values: updates min/max (self-initializing on the first call via `seen`), stores the current range (max - min) in `range`, returns a 1u16 success flag (caller reads range/min/max back as fields).
//! tags: running, min, max, range, stream, stats, tracker, state, wide, u32
//! entry: RunningMinMaxU32::run
struct RunningMinMaxU32 { value: u32, min: u32, max: u32, range: u32, seen: u16 }
impl RunningMinMaxU32 {
    fn run(&mut self) -> u16 {
        if self.seen == 0u16 {
            self.min = self.value;
            self.max = self.value;
            self.seen = 1u16;
        } else {
            if self.value < self.min { self.min = self.value; }
            if self.value > self.max { self.max = self.value; }
        }
        self.range = self.max - self.min;
        1u16
    }
}
