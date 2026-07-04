//! Capped exponential backoff: next = min(current * 2, cap), starting at 1 when current is 0.
//! tags: backoff, retry, exponential, rate-limit, agentic, state
//! entry: Backoff::run
struct Backoff { current: u16, cap: u16, next: u16 }
impl Backoff {
    fn run(&mut self) -> u16 {
        let n = if self.current == 0u16 {
            if self.cap == 0u16 { 0u16 } else { 1u16 }
        } else if self.current > self.cap / 2u16 {
            self.cap
        } else {
            self.current * 2u16
        };
        self.next = n;
        self.next
    }
}
