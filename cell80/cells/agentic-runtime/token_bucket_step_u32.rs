//! Wide/checked sibling of token_bucket_step: refill u32 `tokens` by `refill` (a checked add, escalating instead of silently wrapping), cap at `capacity`, then try to spend `cost`, setting `allowed` to 1/0 (tokens refill either way) — the wide-sibling convention already established by is_lt/is_lt_u32 and min/min_u32, closing the one asymmetry left after rate_window_update (this pack's other rate limiter) was already built at u32/checked/escalate width.
//! tags: rate-limit, token-bucket, budget, agentic, throttle, state, wide, u32, checked, escalate
//! entry: TokenBucketU32::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if tokens + refill would exceed u32::MAX
struct TokenBucketU32 { tokens: u32, capacity: u32, refill: u32, cost: u32, allowed: u16 }
impl TokenBucketU32 {
    fn run(&mut self) -> u16 {
        let refilled = add_checked_u32(self.tokens, self.refill);
        let capped = if refilled > self.capacity { self.capacity } else { refilled };
        let ok = capped >= self.cost;
        if ok {
            self.tokens = capped - self.cost;
        } else {
            self.tokens = capped;
        }
        self.allowed = ok as u16;
        self.allowed
    }
}
