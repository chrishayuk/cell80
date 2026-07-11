//! Periodic payment for an ordinary annuity or annuity-due (Excel's PMT): rate, nper, pv are required; fv defaults to 0 (no balloon/future value) and type defaults to 0 (0=ordinary annuity, payment at period end; 1=annuity-due, payment at period start, found by dividing the ordinary-annuity payment by (1+rate)); follows Excel's outflow-negative sign convention (a pv received now yields a negative pmt paid out each period) -- distinct from IPMT/PPMT (which split this exact payment into interest-only or principal-only portions) and from FV/PV (which solve the same annuity equation for a different unknown).
//! tags: excel, finance, pmt, payment, annuity, loan, amortization, installment, rate, nper, present-value, future-value, type, annuity-due, ordinary-annuity, tvm, time-value-of-money, f32, float, softfloat
//! kernel_bank: on
//! entry: Pmt::run
//! limits: escalates (halt 0xFF06, out_of_domain) if nper == 0, or if rate != 0 and (1+rate)^nper rounds to exactly 1.0 (degenerate zero-growth denominator); escalates (halt 0xFF08, float_domain) if the result is NaN, or (halt 0xFF07, float_overflow) if the result is non-finite
struct Pmt {
    rate: f32,
    nper: u16,
    pv: f32,
    fv: f32,
    typ: u16,
    pmt: f32,
}
impl Pmt {
    fn run(&mut self) -> u16 {
        if self.nper == 0u16 {
            halt(0xFF06u16);
        }

        // (1+rate)^nper via repeated multiplication over the integer period count --
        // frac_pow's/geometric_nth_checked_u32's checked-repeated-multiply idiom,
        // carried over to f32 since no fractional-exponent pow kernel exists.
        let mut growth = 1.0f32;
        let mut i = 0u16;
        while i < self.nper {
            growth = growth * (1.0f32 + self.rate);
            i = i + 1u16;
        }

        let denom = growth - 1.0f32;
        if self.rate != 0.0f32 && denom == 0.0f32 {
            halt(0xFF06u16);
        }

        // int_to_f32 is the dialect's one sanctioned int->float crossing (a typed
        // builtin, not `as`) -- used only for the rate==0 fallback divisor, Excel's
        // own special case since the general formula is 0/0 there.
        let nper_f = int_to_f32(self.nper as u32);

        let pmt_ordinary = if self.rate == 0.0f32 {
            -(self.pv + self.fv) / nper_f
        } else {
            -(self.pv * growth + self.fv) * self.rate / denom
        };

        let adj = if self.typ == 1u16 { 1.0f32 + self.rate } else { 1.0f32 };
        let pmt = pmt_ordinary / adj;

        if pmt.is_nan() {
            halt(0xFF08u16);
        }
        let fin = pmt.is_finite();
        if !fin {
            halt(0xFF07u16);
        }
        self.pmt = pmt;
        1u16
    }
}