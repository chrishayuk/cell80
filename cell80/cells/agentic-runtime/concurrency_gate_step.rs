//! Counting-semaphore concurrency gate: on release (`release`!=0) decrements `in_flight` (floored at 0) and reports allowed=1; on acquire (`release`==0) admits and increments `in_flight` only if strictly under `max_concurrent`, else reports allowed=0 and leaves `in_flight` unchanged -- the event-driven acquire/release hold-count pattern token_bucket_step's fixed per-call refill does not cover.
//! tags: concurrency, semaphore, gate, in-flight, acquire, release, agentic, state, throttle
//! entry: ConcurrencyGateStep::run
struct ConcurrencyGateStep { in_flight: u16, max_concurrent: u16, release: u16, allowed: u16 }
impl ConcurrencyGateStep {
    fn run(&mut self) -> u16 {
        if self.release != 0u16 {
            if self.in_flight > 0u16 {
                self.in_flight = self.in_flight - 1u16;
            }
            self.allowed = 1u16;
        } else {
            let admit = self.in_flight < self.max_concurrent;
            if admit {
                self.in_flight = self.in_flight + 1u16;
            }
            self.allowed = admit as u16;
        }
        self.allowed
    }
}
