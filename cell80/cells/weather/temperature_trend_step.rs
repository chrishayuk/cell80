//! Classify a temperature reading as rising, falling, or steady relative to the previous reading by testing the signed change (current_reading - previous_reading) against a caller-supplied noise threshold band: 2 if the change exceeds +threshold (rising), 0 if the change falls below -threshold (falling), 1 otherwise, inclusive of both boundaries (steady) -- a stateless three-way classification returned as a named u16 code, not a raw signed delta, distinct from agentic-runtime's rising_edge_step/falling_edge_step (which fire on a 0/1 signal's one-shot transition, not a magnitude-threshold comparison over two f32 readings) and from control-systems/deadband (which returns the continuous signed offset-from-center itself, not a 3-way rising/falling/steady classification).
//! tags: weather, temperature, temperature-trend, trend, classification, threshold, noise-band, rising, falling, steady, meteorology, f32, float, softfloat
//! entry: TemperatureTrendStep::run
//! limits: escalates (halt 0xFF08, float_domain) if the rising or falling margin -- rising_margin = (current_reading - previous_reading) - threshold, falling_margin = -((current_reading - previous_reading) + threshold), each combining all three inputs -- comes out NaN; escalates (halt 0xFF07, float_overflow) if either margin is non-finite; threshold is expected non-negative for a meaningful noise band, but a negative threshold still produces a well-defined (if unconventional) classification rather than needing its own domain halt.
struct TemperatureTrendStep {
    current_reading: f32,
    previous_reading: f32,
    threshold: f32,
    trend: u16,
}
impl TemperatureTrendStep {
    fn run(&mut self) -> u16 {
        let delta = self.current_reading - self.previous_reading;
        let rising_margin = delta - self.threshold;
        let falling_margin = -(delta + self.threshold);

        if rising_margin.is_nan() || falling_margin.is_nan() {
            halt(0xFF08u16);
        }
        let rm_fin = rising_margin.is_finite();
        let fm_fin = falling_margin.is_finite();
        if !rm_fin || !fm_fin {
            halt(0xFF07u16);
        }

        let code = if rising_margin > 0.0f32 {
            2u16
        } else if falling_margin > 0.0f32 {
            0u16
        } else {
            1u16
        };

        self.trend = code;
        code
    }
}
