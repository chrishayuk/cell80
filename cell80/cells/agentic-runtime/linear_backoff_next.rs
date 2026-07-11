//! Additive-growth backoff step: next = min(current + step, cap), starting at `step` when current is 0 — the arithmetic-growth dual of backoff_next's capped-exponential growth (doubling vs. adding a fixed step each call).
//! tags: backoff, retry, linear, additive, rate-limit, agentic, state
//! entry: LinearBackoff::run
struct LinearBackoff { current: u16, step: u16, cap: u16, next: u16 }
impl LinearBackoff {
    fn run(&mut self) -> u16 {
        let grown = self.current.saturating_add(self.step);
        let n = if grown > self.cap { self.cap } else { grown };
        self.next = n;
        self.next
    }
}
