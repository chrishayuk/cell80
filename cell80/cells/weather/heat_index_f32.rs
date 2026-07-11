//! Apparent ("feels like") temperature from air temperature and relative humidity via the US National Weather Service's Rothfusz regression -- a pure polynomial in T (Fahrenheit) and RH (percent, 0-100), no transcendentals needed; the first weather/meteorology cell in the library. Implements only the core Rothfusz regression (the NWS's documented valid range T>=80F, RH>=40%) and deliberately omits the NWS's further piecewise low-RH/high-RH adjustment terms used outside that range -- a documented simplification, not an oversight.
//! tags: weather, meteorology, heat-index, apparent-temperature, feels-like, rothfusz, regression, polynomial, humidity, temperature, fahrenheit, f32, float, softfloat
//! entry: HeatIndexF32::run
//! limits: T (`t`) is degrees Fahrenheit, RH (`rh`) is relative humidity as a PERCENT in 0-100 (NOT a 0-1 fraction); this is the bare Rothfusz core regression, valid per the NWS for T>=80F and RH>=40% -- it does NOT apply the NWS's additional low-RH subtraction adjustment (RH<13%, 80F<=T<=112F), high-RH addition adjustment (RH>85%, 80F<=T<=87F), or the simple T/RH-average formula the NWS uses below T=80F; escalates (halt 0xFF08, float_domain) if the polynomial result is NaN, (halt 0xFF07, float_overflow) if it is infinite.
struct HeatIndexF32 {
    t: f32,
    rh: f32,
    hi: f32,
}
impl HeatIndexF32 {
    fn run(&mut self) -> u16 {
        let t2 = self.t * self.t;
        let rh2 = self.rh * self.rh;
        let t_rh = self.t * self.rh;

        let term0 = -42.379f32;
        let term1 = 2.04901523f32 * self.t;
        let term2 = 10.14333127f32 * self.rh;
        let term3 = 0.22475541f32 * t_rh;
        let term4 = 0.00683783f32 * t2;
        let term5 = 0.05481717f32 * rh2;
        let term6 = 0.00122874f32 * t2 * self.rh;
        let term7 = 0.00085282f32 * self.t * rh2;
        let term8 = 0.00000199f32 * t2 * rh2;

        let result = term0 + term1 + term2 - term3 - term4 - term5 + term6 + term7 - term8;

        if result.is_nan() {
            halt(0xFF08u16);
        }
        let fin = result.is_finite();
        if !fin {
            halt(0xFF07u16);
        }

        self.hi = result;
        1u16
    }
}
