//! Rising-edge detector: reports 1 only on the exact step a 0/1 signal transitions from 0 to 1, 0 otherwise — an edge (transition) test, distinct from hysteresis (dead-zone latch), debounce_step (N-consecutive confirmation), streak_step (consecutive-run counter), and cooldown_step (decrement timer).
//! tags: edge, rising-edge, transition, trigger, signal, agentic, state
//! entry: RisingEdge::run
struct RisingEdge { input: u16, prev: u16, edge: u16 }
impl RisingEdge {
    fn run(&mut self) -> u16 {
        self.edge = ((self.input != 0u16) && (self.prev == 0u16)) as u16;
        self.prev = self.input;
        self.edge
    }
}
