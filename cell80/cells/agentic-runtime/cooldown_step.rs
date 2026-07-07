//! Countdown-to-ready state cell: decrements cooldown by 1 (floored at 0) each call, reporting 1 once it reaches 0 — distinct from counter_step (modular increment, round-robin) and backoff_next (exponential growth); no existing agentic-runtime cell does a plain decrement-to-zero.
//! tags: cooldown, countdown, ready, timer, wait, agentic, throttle, state
//! entry: CooldownStep::run
struct CooldownStep { cooldown: u16, ready: u16 }
impl CooldownStep {
    fn run(&mut self) -> u16 {
        if self.cooldown > 0u16 {
            self.cooldown = self.cooldown - 1u16;
        }
        self.ready = (self.cooldown == 0u16) as u16;
        self.ready
    }
}
