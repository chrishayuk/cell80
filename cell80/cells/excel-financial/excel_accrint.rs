//! Accrued interest on a periodic-interest (coupon-paying) security since its last coupon date -- par*rate/frequency times an already-computed day-count fraction (the caller runs whichever basis-dispatched day-count cell, day_count_30_360_us/eu, day_count_act_act, day_count_act_360, or day_count_act_365, that Excel's basis argument selects, then feeds its output in here as dsm_over_b), distinct from ACCRINTM (one lump accrual spanning the security's whole issue-to-maturity life, no frequency divisor at all) and from the COUPDAYS*/COUPPCD/COUPNCD family (which derive the coupon date and day-count fraction themselves; this cell only consumes an already-derived fraction, it never re-steps the coupon schedule).
//! tags: excel, accrint, accrued-interest, bond, coupon, periodic-interest, day-count, fraction, par, rate, frequency, security, finance
//! entry: ExcelAccrint::run
//! limits: escalates (halt 0xFF06, out_of_domain) if rate <= 0, par <= 0, frequency isn't 1/2/4, or dsm_over_b < 0; escalates (halt 0xFF07, float_overflow) if the result is infinite; escalates (halt 0xFF08, float_domain) if the result is NaN

// Excel signature: ACCRINT(issue, first_interest, settlement, rate, par, frequency, [basis], [calc_method]).
// rate and frequency are required; frequency must be 1 (annual), 2 (semiannual), or 4
// (quarterly) -- anything else is Excel's #NUM!. par is optional and Excel defaults it
// to 1000 when omitted; callers of this cell pass 1000.0 explicitly for that case.
// issue/first_interest/settlement and the optional basis (default 0, 30/360 US) are
// consumed upstream of this cell: the caller dispatches on basis to pick the matching
// day_count_* cell, runs it over (last-coupon-or-issue, settlement), and feeds the
// resulting accrued-day-count fraction in here as dsm_over_b (days-since-last-coupon
// over the basis denominator, i.e. 0.0 at the coupon date itself up to just under 1.0
// right before the next coupon). This cell implements the common single-accrual-period
// case (settlement within one coupon period of issue), where Excel's optional
// calc_method (TRUE/FALSE, default TRUE = accrue from issue date) collapses to the same
// fraction; summing several quasi-coupon periods for calc_method's multi-period case is
// out of scope here.
struct ExcelAccrint {
    rate: f32,
    par: f32,
    frequency: u16,
    dsm_over_b: f32,
    accrued_interest: f32,
}
impl ExcelAccrint {
    fn run(&mut self) -> u16 {
        if self.rate <= 0.0f32 { halt(0xFF06u16); }
        if self.par <= 0.0f32 { halt(0xFF06u16); }
        let freq_ok = self.frequency == 1u16 || self.frequency == 2u16 || self.frequency == 4u16;
        if !freq_ok { halt(0xFF06u16); }
        if self.dsm_over_b < 0.0f32 { halt(0xFF06u16); }

        let freq_f = int_to_f32(self.frequency);
        let per_period_rate = self.rate / freq_f;
        let coupon_amount = self.par * per_period_rate;
        let accrued = coupon_amount * self.dsm_over_b;

        if accrued.is_nan() { halt(0xFF08u16); }
        let fin = accrued.is_finite();
        if !fin { halt(0xFF07u16); }

        self.accrued_interest = accrued;
        1u16
    }
}
