//! Excel FV(rate, nper, pmt, [pv], [type]): future value of an investment with nper whole periods of a level payment pmt, simulated period-by-period as an account balance (each period: balance = balance*(1+rate), with pmt added before growth when due=1/annuity-due, or after growth when due=0/ordinary -- the omittable default; `due` stands in for Excel's `type` arg, a reserved word here) rather than the closed-form pmt*((1+rate)^nper-1)/rate divide, which is 0/0 exactly when rate == 0 and this loop sidesteps entirely; outflow-negative like Excel (pv and pmt are the cash paid in, negative; fv is the cash returned); pv is omittable, defaulting to 0.0; distinct from PV (solves for the opposite unknown) and PMT (solves for the payment itself) in the same family.
//! tags: finance, excel, fv, future-value, annuity, annuity-due, ordinary-annuity, compounding, time-value-of-money, periodic-payment, balance, simulate, f32, float, softfloat
//! entry: ExcelFv::run
//! limits: escalates (halt 0xFF08, float_domain) if the computed future value is NaN; escalates (halt 0xFF07, float_overflow) if it is otherwise non-finite
struct ExcelFv { rate: f32, nper: u16, pmt: f32, pv: f32, due: u16, fv: f32 }
impl ExcelFv {
    fn run(&mut self) -> u16 {
        let onerate = 1.0f32 + self.rate;
        let mut bal = self.pv;
        let mut i = 0u16;
        while i < self.nper {
            let pre = if self.due == 1u16 { bal + self.pmt } else { bal };
            let grown = pre * onerate;
            let post = if self.due == 1u16 { grown } else { grown + self.pmt };
            bal = post;
            i = i + 1u16;
        }
        let fv = -bal;
        if fv.is_nan() {
            halt(0xFF08u16);
        }
        let fin = fv.is_finite();
        if !fin {
            halt(0xFF07u16);
        }
        self.fv = fv;
        1u16
    }
}
