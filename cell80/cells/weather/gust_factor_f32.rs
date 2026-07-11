//! Gust factor: the ratio of peak (gust) wind speed to the mean (sustained) wind speed observed over the same period -- a standard meteorological and aviation term (used in METAR/TAF gust groups and turbulence/wind-shear reporting) quantifying how much stronger gusts are than the average wind, e.g. a gust factor of 1.5 means gusts run 50% above the mean; distinct from a bare division primitive because a mean_wind_speed of zero or less isn't just an arithmetic edge case here but an invalid wind observation to form a ratio against, so it escalates rather than silently returning infinity or a negative "gust factor".
//! tags: weather, meteorology, wind, gust, gust-factor, wind-speed, turbulence, aviation, metar, ratio, f32, float, softfloat
//! entry: GustFactor::run
//! limits: escalates (halt 0xFF06, out_of_domain) if mean_wind_speed <= 0.0 (a non-positive mean wind speed is not a valid observation to form a gust factor against); escalates (halt 0xFF08, float_domain) / (halt 0xFF07, float_overflow) if the computed ratio is NaN / non-finite
struct GustFactor {
    peak_gust_speed: f32,
    mean_wind_speed: f32,
    gust_factor: f32,
}
impl GustFactor {
    fn run(&mut self) -> u16 {
        if self.mean_wind_speed <= 0.0f32 {
            halt(0xFF06u16);
        }
        let g = self.peak_gust_speed / self.mean_wind_speed;
        if g.is_nan() {
            halt(0xFF08u16);
        }
        let g_fin = g.is_finite();
        if !g_fin {
            halt(0xFF07u16);
        }
        self.gust_factor = g;
        1u16
    }
}
