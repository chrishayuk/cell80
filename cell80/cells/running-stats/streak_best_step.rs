//! Consecutive-streak counter that also remembers the longest streak ever seen: increments/resets exactly like streak_step, then updates `best` whenever the current streak exceeds it — the same running-extreme extension running_min_max_step applies to raw values, applied here to the derived streak count itself.
//! tags: streak, counter, consecutive, run-length, best, max, running, stream, state
//! entry: StreakBest::run
struct StreakBest { input: u16, streak: u16, best: u16 }
impl StreakBest {
    fn run(&mut self) -> u16 {
        if self.input != 0u16 {
            self.streak = self.streak + 1u16;
        } else {
            self.streak = 0u16;
        }
        if self.streak > self.best {
            self.best = self.streak;
        }
        self.best
    }
}
