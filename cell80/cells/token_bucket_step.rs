//! Token-bucket rate limiter step: refill by `refill`, cap at `capacity`, then try to spend `cost`; 1 if allowed, 0 if not enough tokens (tokens still refill either way).
//! tags: rate-limit, token-bucket, budget, agentic, throttle, state
//! entry: TokenBucket::run
struct TokenBucket { tokens: u16, capacity: u16, refill: u16, cost: u16, allowed: u16 }
impl TokenBucket {
    fn run(&mut self) -> u16 {
        let refilled = self.tokens + self.refill;
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
