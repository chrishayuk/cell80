//! Hysteresis (Schmitt-trigger) state: turns on at value >= high, turns off at value <= low, else holds the prior state (the dead zone between them).
//! tags: hysteresis, schmitt-trigger, threshold, dead-zone, agentic, state
//! entry: Hysteresis::run
struct Hysteresis { value: u16, low: u16, high: u16, state: u16 }
impl Hysteresis {
    fn run(&mut self) -> u16 {
        if self.value >= self.high {
            self.state = 1u16;
        } else if self.value <= self.low {
            self.state = 0u16;
        }
        self.state
    }
}
