//! Debounce a noisy 0/1 signal: only confirms a change to `input` once it's held for `threshold` consecutive steps; output is the last confirmed-stable value.
//! tags: debounce, signal, filter, stable, agentic, state
//! entry: Debounce::run
struct Debounce { input: u16, last_stable: u16, count: u16, threshold: u16, output: u16 }
impl Debounce {
    fn run(&mut self) -> u16 {
        if self.input == self.last_stable {
            self.count = 0u16;
        } else {
            self.count = self.count + 1u16;
            if self.count >= self.threshold {
                self.last_stable = self.input;
                self.count = 0u16;
            }
        }
        self.output = self.last_stable;
        self.output
    }
}
