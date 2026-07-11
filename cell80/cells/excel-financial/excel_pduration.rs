//! Excel PDURATION(rate, pv, fv): the number of periods a lump sum pv needs at a fixed per-period rate to reach fv, (ln(fv) - ln(pv)) / ln(1+rate) -- pure compound growth of a single deposit, no payment stream, which is exactly what separates it from NPER (the annuity sibling whose pmt term makes the closed form a quotient inside the log rather than a difference of logs). The unknown sits in the exponent, so this rides the owned F2 transcendentals (three fln calls). Returns a fractional period count in the wide `pduration` state field, returns 1.
//! tags: finance, excel, pduration, periods, duration, compound, growth, lump-sum, time-value-of-money, logarithm, transcendental, f32, float, softfloat
//! entry: ExcelPduration::run
//! kernel_bank: on
//! accuracy: <= 12 ulp (three fln calls at <= 2 ulp each, one fsub, one fdiv; rustz80's F2 harness pins the kernels -- known limit: ln(1+rate) loses relative accuracy below rate ~ 1e-4, the fln1p gap, documented not hidden)
//! limits: escalates (halt 0xFF06, out_of_domain) on Excel's #NUM! domain: rate <= 0, pv <= 0, or fv <= 0 (all three must be strictly positive); escalates (halt 0xFF08, float_domain) on a NaN result, (halt 0xFF07, float_overflow) on a non-finite one
struct ExcelPduration { rate: f32, pv: f32, fv: f32, pduration: f32 }
impl ExcelPduration {
    fn run(&mut self) -> u16 {
        if self.rate <= 0.0f32 {
            halt(0xFF06u16);
        }
        if self.pv <= 0.0f32 {
            halt(0xFF06u16);
        }
        if self.fv <= 0.0f32 {
            halt(0xFF06u16);
        }
        let growth = (1.0f32 + self.rate).ln();
        let pd = (self.fv.ln() - self.pv.ln()) / growth;
        if pd.is_nan() {
            halt(0xFF08u16);
        }
        let fin = pd.is_finite();
        if !fin {
            halt(0xFF07u16);
        }
        self.pduration = pd;
        1u16
    }
}
