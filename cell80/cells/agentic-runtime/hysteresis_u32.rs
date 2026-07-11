//! Wide sibling of hysteresis: identical Schmitt-trigger dead-zone latch (turns on at value >= high, turns off at value <= low, else holds the prior state) but over u32 value/low/high fields — closes the u32 gap left after token_bucket_step_u32 established the wide-sibling convention for this pack.
//! tags: hysteresis, schmitt-trigger, threshold, dead-zone, agentic, state, wide, u32
//! entry: HysteresisU32::run
struct HysteresisU32 { value: u32, low: u32, high: u32, state: u16 }
impl HysteresisU32 {
    fn run(&mut self) -> u16 {
        if self.value >= self.high {
            self.state = 1u16;
        } else if self.value <= self.low {
            self.state = 0u16;
        }
        self.state
    }
}
