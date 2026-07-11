//! Edge-triggered toggle latch (T flip-flop): flips a sticky 0/1 `state` on every rising edge of `trigger` since the last call, holding between edges -- distinct from rising_edge_step (reports a one-shot pulse, no persistent state) and hysteresis (latches on a value band, not an edge event).
//! tags: toggle, flip-flop, latch, edge, trigger, agentic, state
//! entry: ToggleStep::run
struct ToggleStep { trigger: u16, prev: u16, state: u16 }
impl ToggleStep {
    fn run(&mut self) -> u16 {
        if (self.trigger != 0u16) && (self.prev == 0u16) {
            self.state = (self.state == 0u16) as u16;
        }
        self.prev = self.trigger;
        self.state
    }
}
