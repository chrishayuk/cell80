//! Dead-man's-switch / heartbeat watchdog: a `pet` signal resets elapsed ticks to 0 and clears the trip, otherwise ticks increments (capped at `timeout`) and a sticky `tripped` alarm sets once ticks reaches timeout, clearing only on the next pet — distinct from cooldown_step (plain countdown-to-zero), hysteresis (value-threshold latch) and debounce_step (signal-stability confirmation), none of which detect loss-of-liveness from an external reset signal.
//! tags: watchdog, heartbeat, dead-man-switch, timeout, liveness, alarm, agentic, state
//! entry: WatchdogStep::run
struct WatchdogStep { ticks: u16, timeout: u16, pet: u16, tripped: u16 }
impl WatchdogStep {
    fn run(&mut self) -> u16 {
        if self.pet != 0u16 {
            self.ticks = 0u16;
            self.tripped = 0u16;
        } else {
            if self.ticks < self.timeout {
                self.ticks = self.ticks + 1u16;
            }
            if self.ticks >= self.timeout {
                self.tripped = 1u16;
            }
        }
        self.tripped
    }
}
