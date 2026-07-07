//! Circuit-breaker state machine step: closed(0) counts failures and opens at the threshold; open(1) waits for cooldown then tries half-open(2); half-open resolves to closed on success or back to open on failure.
//! tags: circuit-breaker, resilience, state-machine, agentic, retry, fault-tolerance
//! entry: CircuitBreaker::run
struct CircuitBreaker { state: u16, fail_count: u16, fail_threshold: u16, cooldown_elapsed: u16, success: u16 }
impl CircuitBreaker {
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
            }
        } else {
            if self.success != 0u16 {
                self.state = 0u16;
                self.fail_count = 0u16;
            } else {
                self.state = 1u16;
            }
        }
        self.state
    }
}
