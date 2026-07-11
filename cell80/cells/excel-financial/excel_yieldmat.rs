//! Annual yield of a security that pays its interest only at maturity (Excel YIELDMAT: yield = ((100 + (DIM/B)*rate*100) / (pr + (A/B)*rate*100) - 1) / (DSM/B)), taking all three basis-dependent day-count fractions -- DIM/B (issue to maturity), DSM/B (settlement to maturity), A/B (issue to settlement) -- as already-computed inputs from three separate calls to whichever basis-dispatched day_count_* cell the caller's basis argument selects, never re-deriving any of them here -- distinct from YIELD (a periodic-coupon bond needing Newton iteration over a whole COUPNUM-stepped coupon schedule, no closed form at all), from YIELDDISC (a pure below-redemption discount security with no issue date or rate/DIM/A term at all, just a single DSM/B fraction against redemption), and from PRICEMAT (this exact same triangular relationship solved in the opposite direction: price recovered from a known yld, rather than yld recovered from a known price).
//! tags: excel, yieldmat, yield, maturity, interest-at-maturity, security, bond, note, day-count, fraction, basis, settlement, issue, price, closed-form, finance
//! kernel_bank: on
//! entry: ExcelYieldmat::run
//! limits: escalates (halt 0xFF06, out_of_domain) if rate < 0, pr <= 0, dim_over_b <= 0, dsm_over_b <= 0, a_over_b < 0, or if the denominator (pr + a_over_b*rate*100) is <= 0; escalates (halt 0xFF07, float_overflow) if the result is infinite; escalates (halt 0xFF08, float_domain) if the result is NaN

// Excel signature: YIELDMAT(settlement, maturity, issue, rate, pr, [basis]).
// settlement, maturity, issue, rate, and pr are all required; basis is optional and
// defaults to 0 (US 30/360) when omitted. settlement/maturity/issue and the basis
// dispatch are all consumed upstream of this cell: the caller picks the day_count_*
// cell matching basis (day_count_30_360_us for 0, day_count_act_act for 1,
// day_count_act_360 for 2, day_count_act_365 for 3, day_count_30_360_eu for 4) and
// runs it three times over the same basis -- once over (issue, maturity) for DIM/B,
// once over (settlement, maturity) for DSM/B, once over (issue, settlement) for A/B --
// feeding the three resulting fractions in here as dim_over_b, dsm_over_b, and
// a_over_b. This cell only ever multiplies/divides by those three fractions once
// apiece; it never re-derives any day count itself.
// rate and pr are entered as plain positive decimals/quoted prices (e.g. 0.061 for
// 6.10%, 99.75 for a price per $100 face), matching Excel's own convention of #NUM!
// on a negative rate or non-positive price. Unlike YIELD (also takes a redemption
// argument for the face value returned at maturity), YIELDMAT assumes the security
// redeems at its own par/100 alongside its final interest payment, so there is no
// separate redemption input. There is no outflow-negative sign convention here
// either (YIELDMAT returns a yield rate, not a cash flow), and no type (0/1
// annuity-due) argument for this function at all.
struct ExcelYieldmat {
    rate: f32,
    pr: f32,
    dim_over_b: f32,
    dsm_over_b: f32,
    a_over_b: f32,
    yld: f32,
}
impl ExcelYieldmat {
    fn run(&mut self) -> u16 {
        if self.rate < 0.0f32 { halt(0xFF06u16); }
        if self.pr <= 0.0f32 { halt(0xFF06u16); }
        // DIM (issue to maturity) and DSM (settlement to maturity) must both be
        // strictly positive -- a security's maturity always lies strictly after both
        // its issue date and its settlement date.
        if self.dim_over_b <= 0.0f32 { halt(0xFF06u16); }
        if self.dsm_over_b <= 0.0f32 { halt(0xFF06u16); }
        // A (issue to settlement) may be zero (settlement falling exactly on the
        // issue date) but never negative.
        if self.a_over_b < 0.0f32 { halt(0xFF06u16); }

        let numerator = 100.0f32 + self.dim_over_b * self.rate * 100.0f32;
        let denom = self.pr + self.a_over_b * self.rate * 100.0f32;
        if denom <= 0.0f32 { halt(0xFF06u16); }

        let ratio = numerator / denom;
        let yld = (ratio - 1.0f32) / self.dsm_over_b;

        if yld.is_nan() { halt(0xFF08u16); }
        let fin = yld.is_finite();
        if !fin { halt(0xFF07u16); }

        self.yld = yld;
        1u16
    }
}
