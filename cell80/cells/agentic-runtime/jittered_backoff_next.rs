//! Full-jitter backoff: computes backoff_next's own capped-exponential ceiling, then scales it down by a caller-supplied rand_bps (0-9999 basis points) via a u32 intermediate, returning a randomized value in [0, ceiling] instead of backoff_next's fixed climb.
//! tags: backoff, retry, exponential, jitter, random, rate-limit, agentic, state
//! entry: JitteredBackoff::run
struct JitteredBackoff { current: u16, cap: u16, rand_bps: u16, next: u16 }
impl JitteredBackoff {
    fn run(&mut self) -> u16 {
        let ceiling = if self.current == 0u16 {
            if self.cap == 0u16 { 0u16 } else { 1u16 }
        } else if self.current > self.cap / 2u16 {
            self.cap
        } else {
            self.current * 2u16
        };
        let scaled = ceiling as u32 * self.rand_bps as u32 / 10000u32;
        let n = if (scaled >> 16u32) as u16 != 0u16 { 65535u16 } else { scaled as u16 };
        self.next = n;
        self.next
    }
}
