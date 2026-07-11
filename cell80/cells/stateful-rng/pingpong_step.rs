//! Bounded ping-pong index: bounces `pos` between 0 and `limit`, reversing direction at each bound before stepping instead of wrapping to 0 (limit 0 pins pos at 0) — a triangle-wave counter distinct from counter_step's round-robin wrap, useful for an oscillating animation frame or alternating dispatch slot. `dir` 0=increasing, 1=decreasing; the caller threads `pos` and `dir` through — re-supply both fields each call.
//! tags: counter, bounce, oscillate, triangle-wave, ping-pong, index, animation, alternate, reverse, state, dispatch
//! entry: PingPong::run
struct PingPong { pos: u16, dir: u16, limit: u16 }
impl PingPong {
    fn run(&mut self) -> u16 {
        let at_top = self.dir == 0u16 && self.pos >= self.limit;
        let at_bottom = self.dir == 1u16 && self.pos == 0u16;
        let dir = if at_top { 1u16 } else if at_bottom { 0u16 } else { self.dir };
        let pos = if self.limit == 0u16 { 0u16 } else if dir == 0u16 { self.pos + 1u16 } else { self.pos - 1u16 };
        self.dir = dir;
        self.pos = pos;
        pos
    }
}
