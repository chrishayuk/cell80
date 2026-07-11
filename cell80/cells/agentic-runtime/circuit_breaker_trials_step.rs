//! Circuit-breaker state machine identical to circuit_breaker_step's closed/open/half-open shape, except half-open only resolves to closed after `success_threshold` CONSECUTIVE successes (the tally resets to 0 on any failure, dropping straight back to open) instead of circuit_breaker_step's fixed single-success resolution.
//! tags: circuit-breaker, resilience, state-machine, agentic, retry, fault-tolerance, consecutive, trials
//! entry: CircuitBreakerTrials::run
struct CircuitBreakerTrials { state: u16, fail_count: u16, fail_threshold: u16, cooldown_elapsed: u16, success: u16, success_count: u16, success_threshold: u16 }
impl CircuitBreakerTrials {
    fn run(&mut self) -> u16 {
        if self.state == 0u16 {
            if self.success != 0u16 {
                self.fail_count = 0u16;
            } else {
                self.fail_count = self.fail_count + 1u16;
                if self.fail_count >= self.fail_threshold {
                    self.state = 1u16;
                }
            }
        } else if self.state == 1u16 {
            if self.cooldown_elapsed != 0u16 {
                self.state = 2u16;
                self.success_count = 0u16;
            }
        } else {
            if self.success != 0u16 {
                self.success_count = self.success_count + 1u16;
                if self.success_count >= self.success_threshold {
                    self.state = 0u16;
                    self.fail_count = 0u16;
                    self.success_count = 0u16;
                }
            } else {
                self.success_count = 0u16;
                self.state = 1u16;
            }
        }
        self.state
    }
}
