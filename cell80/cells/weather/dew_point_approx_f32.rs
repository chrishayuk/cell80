//! Approximate dew point temperature Td (Celsius) from air temperature T (Celsius) and relative humidity RH (percent, 0-100), via the well-known linear stand-in Td = T - ((100 - RH) / 5) -- NOT the exact psychrometric (Magnus-formula) dew point, which needs a natural log this dialect's kernels don't carry (a genuine, out-of-scope gap, not something quietly reintroduced here); this approximation is commonly cited as accurate to within about 1 degree C only for RH > 50%, degrading below that, so the cell is deliberately named dew_point_approx_f32 (not dew_point_f32) so it is never mistaken for that exact version.
//! tags: weather, meteorology, dew-point, dew-point-approx, humidity, relative-humidity, temperature, linear-approximation, approximation, magnus-formula-gap, f32, float, softfloat
//! entry: DewPointApproxF32::run
//! limits: escalates (halt 0xFF06, out_of_domain) if rh_pct is outside [0.0, 100.0] (RH is only meaningful as a percentage in that range); escalates (halt 0xFF08, float_domain) if the computed dew point is NaN, or (halt 0xFF07, float_overflow) if it is non-finite (both only reachable from a non-finite temp_c input, since the arithmetic itself -- one subtract, one divide, one subtract -- cannot itself produce a non-finite result from finite, in-domain inputs); the ~1 degree C accuracy this formula is commonly cited for holds only above roughly RH 50% and is NOT itself checked or enforced by this cell -- a caller needing better accuracy at low RH, or an exact result, needs the (currently out-of-scope, ln-dependent) Magnus formula instead, which this cell deliberately does not attempt.
struct DewPointApproxF32 {
    temp_c: f32,
    rh_pct: f32,
    dew_point_c: f32,
}
impl DewPointApproxF32 {
    fn run(&mut self) -> u16 {
        if self.rh_pct < 0.0f32 || self.rh_pct > 100.0f32 {
            halt(0xFF06u16);
        }
        let deficit = (100.0f32 - self.rh_pct) / 5.0f32;
        let td = self.temp_c - deficit;
        if td.is_nan() {
            halt(0xFF08u16);
        }
        let fin = td.is_finite();
        if !fin {
            halt(0xFF07u16);
        }
        self.dew_point_c = td;
        1u16
    }
}
