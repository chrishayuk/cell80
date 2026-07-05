//! Fixed-window rate limiter step: given the current time `now`, the running window's start and size, and the count so far, rolls over to a fresh window (starting at `now`) once `now - window_start >= window_size`, then allows the event if `count < limit` (incrementing count) — distinct from token_bucket_step's smooth refill-and-spend model, this is the simpler "N events per window" shape. The caller threads window_start/count through repeated calls, matching backoff_next/token_bucket_step's convention.
//! tags: rate-limit, rate-window, sliding-window, throttle, agentic, budget, state, wide, u32, checked, escalate
//! entry: RateWindowUpdate::run
//! limits: escalates (halt 0xFF06, out_of_domain) if now < window_start (time moving backward is a caller bug, not a rate-limit decision)
struct RateWindowUpdate { now: u32, window_start: u32, window_size: u32, count: u32, limit: u32 }
impl RateWindowUpdate {
    fn run(&mut self) -> u16 {
        if self.now < self.window_start { halt(0xFF06u16); }
        if self.now - self.window_start >= self.window_size {
            self.window_start = self.now;
            self.count = 0u32;
        }
        let mut allowed = 0u16;
        if self.count < self.limit {
            self.count = self.count + 1u32;
            allowed = 1u16;
        }
        allowed
    }
}
