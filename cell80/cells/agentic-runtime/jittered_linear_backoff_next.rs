//! Full-jitter additive backoff: computes linear_backoff_next's own capped-additive ceiling (min(current+step, cap), starting at step when current is 0), then scales it down by a caller-supplied rand_bps (0-9999 basis points) via a u32 intermediate, the same way jittered_backoff_next scales backoff_next's exponential ceiling.
//! tags: backoff, retry, linear, additive, jitter, random, rate-limit, agentic, state
//! entry: JitteredLinearBackoff::run
struct JitteredLinearBackoff { current: u16, step: u16, cap: u16, rand_bps: u16, next: u16 }
impl JitteredLinearBackoff {
    fn run(&mut self) -> u16 {
        let grown = self.current.saturating_add(self.step);
        let ceiling = if grown > self.cap { self.cap } else { grown };
        let scaled = ceiling as u32 * self.rand_bps as u32 / 10000u32;
        let n = if (scaled >> 16u32) as u16 != 0u16 { 65535u16 } else { scaled as u16 };
        self.next = n;
        self.next
    }
}
