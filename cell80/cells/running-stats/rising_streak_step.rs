//! Consecutive-increase streak counter over a stream: increments while the new value is strictly greater than the immediately preceding one, resets to 0 otherwise (and on the first call, self-initializing via `seen` like running_min_max_step).
//! tags: streak, counter, consecutive, run-length, trend, increasing, rising, stream, state
//! entry: RisingStreak::run
struct RisingStreak { value: u16, prev: u16, streak: u16, seen: u16 }
impl RisingStreak {
    fn run(&mut self) -> u16 {
        if self.seen == 0u16 {
            self.streak = 0u16;
            self.seen = 1u16;
        } else {
            if self.value > self.prev {
                self.streak = self.streak + 1u16;
            } else {
                self.streak = 0u16;
            }
        }
        self.prev = self.value;
        self.streak
    }
}
