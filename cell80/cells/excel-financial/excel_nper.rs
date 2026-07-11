//! Excel NPER(rate, pmt, pv, [fv], [type]): the number of periods a level-payment annuity needs to grow pv (plus the stream of pmt payments) into -fv, solved in closed form -- the unknown sits in the exponent, so this is the pack's first cell through the owned F2 transcendentals (nper = ln((pmt*f - fv*rate) / (pmt*f + pv*rate)) / ln(1+rate), f = 1 + rate*due), where FV/PV/PMT (the same five-argument family solving for other unknowns) stay polynomial and RATE has no closed form at all (Excel itself iterates). rate == 0 degenerates to the linear -(pv+fv)/pmt. Outflow-negative like Excel (pv/pmt are cash paid in); fv and due are omittable, defaulting to 0.0/0. Returns a fractional period count (Excel does not round it either); the result lives in the wide `nper` state field, returns 1.
//! tags: finance, excel, nper, periods, number-of-periods, annuity, loan, term, time-value-of-money, logarithm, transcendental, f32, float, softfloat
//! entry: ExcelNper::run
//! kernel_bank: on
//! accuracy: <= 8 ulp (two fln calls at <= 2 ulp each, one fdiv; rustz80's F2 harness pins the kernels -- known limit: ln(1+rate) loses relative accuracy below rate ~ 1e-4, the fln1p gap, documented not hidden)
//! limits: escalates (halt 0xFF06, out_of_domain) on Excel's #NUM! cases: rate == 0 with pmt == 0 (no payment, no growth -- nothing ever changes), 1 + rate <= 0, or a non-positive logarithm argument (the annuity can never reach fv); escalates (halt 0xFF08, float_domain) on a NaN result, (halt 0xFF07, float_overflow) on a non-finite one
struct ExcelNper { rate: f32, pmt: f32, pv: f32, fv: f32, due: u16, nper: f32 }
impl ExcelNper {
    fn run(&mut self) -> u16 {
        let mut nper = 0.0f32;
        if self.rate == 0.0f32 {
            if self.pmt == 0.0f32 {
                halt(0xFF06u16);
            }
            nper = -(self.pv + self.fv) / self.pmt;
        } else {
            let onerate = 1.0f32 + self.rate;
            if onerate <= 0.0f32 {
                halt(0xFF06u16);
            }
            let f = if self.due == 1u16 { onerate } else { 1.0f32 };
            let pf = self.pmt * f;
            let num = pf - self.fv * self.rate;
            let den = pf + self.pv * self.rate;
            let arg = num / den;
            if arg <= 0.0f32 {
                halt(0xFF06u16);
            }
            nper = arg.ln() / onerate.ln();
        }
        if nper.is_nan() {
            halt(0xFF08u16);
        }
        let fin = nper.is_finite();
        if !fin {
            halt(0xFF07u16);
        }
        self.nper = nper;
        1u16
    }
}
