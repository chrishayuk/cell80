//! Wind chill temperature from air temperature and wind speed, the NWS 2001 formula WC = 35.74 + 0.6215*T - 35.75*V^0.16 + 0.4275*T*V^0.16 (T in Fahrenheit, V in mph, valid only for V>=3mph and T<=50F per the NWS's own published domain) -- the fractional exponent V^0.16 = V^(4/25), the 25th root of V^4, is composed rather than reinvented: the caller runs nth_root_f32(V^4, 25) once upstream and feeds the settled root in here as v_pow4_25th_root, the same already-computed-input convention excel-financial's day-count-fraction family (excel_accrint, excel_pricedisc, excel_received, etc.) already established for a shared sub-computation no single cell should re-derive; the first cell in a new weather pack, distinct from a future heat_index_f32 sibling this pack's own naming convention already anticipates (heat index blends temperature with humidity, not wind, and would need a still-unavailable transcendental this cell does not).
//! tags: weather, wind-chill, apparent-temperature, temperature, wind-speed, nws, meteorology, fractional-exponent, nth-root, compose, f32, float, softfloat
//! entry: WindChillF32::run
//! limits: escalates (halt 0xFF06, out_of_domain) if v < 3.0 (the NWS wind chill formula is undefined/unreliable below its own 3 mph calm-wind floor), t > 50.0 (the NWS only publishes wind chill at or below 50F), or v_pow4_25th_root <= 0.0 (the caller-computed 25th root of a positive v^4 must itself be strictly positive -- a sanity check on the already-computed input, not a re-derivation of it); escalates (halt 0xFF08, float_domain) if the result is NaN, or (halt 0xFF07, float_overflow) if it is infinite.
struct WindChillF32 {
    t: f32,
    v: f32,
    v_pow4_25th_root: f32,
    wc: f32,
}
impl WindChillF32 {
    fn run(&mut self) -> u16 {
        if self.v < 3.0f32 {
            halt(0xFF06u16);
        }
        if self.t > 50.0f32 {
            halt(0xFF06u16);
        }
        if self.v_pow4_25th_root <= 0.0f32 {
            halt(0xFF06u16);
        }

        let v016 = self.v_pow4_25th_root;
        let wc = 35.74f32 + 0.6215f32 * self.t - 35.75f32 * v016 + 0.4275f32 * self.t * v016;

        if wc.is_nan() {
            halt(0xFF08u16);
        }
        let fin = wc.is_finite();
        if !fin {
            halt(0xFF07u16);
        }

        self.wc = wc;
        1u16
    }
}
