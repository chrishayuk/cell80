//! Consecutive-streak counter: increments while input is nonzero, resets to 0 the moment input is 0.
//! tags: streak, counter, consecutive, run-length, stream, state
//! entry: Streak::run
struct Streak { input: u16, streak: u16 }
impl Streak {
    fn run(&mut self) -> u16 {
        if self.input != 0u16 {
            self.streak = self.streak + 1u16;
        } else {
            self.streak = 0u16;
        }
        self.streak
    }
}
