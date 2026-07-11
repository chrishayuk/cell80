//! Falling-edge detector: reports 1 only on the exact step a 0/1 signal transitions from 1 to 0, 0 otherwise — the mirror-image counterpart of rising_edge_step's 0->1 test.
//! tags: edge, falling-edge, transition, trigger, signal, agentic, state
//! entry: FallingEdge::run
struct FallingEdge { input: u16, prev: u16, edge: u16 }
impl FallingEdge {
    fn run(&mut self) -> u16 {
        self.edge = ((self.input == 0u16) && (self.prev != 0u16)) as u16;
        self.prev = self.input;
        self.edge
    }
}
