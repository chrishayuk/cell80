//! Sliding-window-counter rate limiter step: on rollover (now - window_start >= window_size) carries curr_count into prev_count, resets curr_count to 0 and window_start to now, then admits if curr_count + prev_count*(window_size - elapsed)/window_size is under limit (incrementing curr_count on admit) -- the weighted blend of the two windows fixes rate_window_update's hard-reset boundary-burst gap without token_bucket_step's continuous-refill model.
//! tags: rate-limit, sliding-window, rate-window, throttle, agentic, budget, state, wide, u32, checked, escalate
//! entry: SlidingWindowCounterStep::run
//! limits: escalates (halt 0xFF06, out_of_domain) if now < window_start, or (halt 0xFF05, needs_wider_math) if prev_count*remaining or curr_count+weighted_prev would exceed u32::MAX
struct SlidingWindowCounterStep { now: u32, window_start: u32, window_size: u32, prev_count: u32, curr_count: u32, limit: u32 }
impl SlidingWindowCounterStep {
    fn run(&mut self) -> u16 {
        if self.now < self.window_start { halt(0xFF06u16); }
        let elapsed_before = self.now - self.window_start;
        if elapsed_before >= self.window_size {
            self.prev_count = self.curr_count;
            self.curr_count = 0u32;
            self.window_start = self.now;
        }
        let elapsed = self.now - self.window_start;
        let remaining = self.window_size - elapsed;
        let weighted = mul_checked_u32(self.prev_count, remaining);
        let weighted_prev = if self.window_size != 0u32 { weighted / self.window_size } else { 0u32 };
        let estimate = add_checked_u32(self.curr_count, weighted_prev);
        let mut allowed = 0u16;
        if estimate < self.limit {
            self.curr_count = self.curr_count + 1u32;
            allowed = 1u16;
        }
        allowed
    }
}
