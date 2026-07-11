//! Price per $100 face value of a security that pays its interest only at maturity (Excel PRICEMAT: price = (100 + (DIM/B)*rate*100) / (1 + (DSM/B)*yld) - (A/B)*rate*100), taking all three basis-dependent day-count fractions -- DIM/B (issue to maturity), DSM/B (settlement to maturity), A/B (issue to settlement) -- as already-computed inputs from three separate calls to whichever basis-dispatched day_count_* cell the caller's basis argument selects, never re-deriving any of them here -- distinct from PRICEDISC (a pure below-redemption discount security with no rate/yld coupon term at all and only a single day-count fraction, DSM/B), from PRICE (a periodic-coupon bond that must step the full COUPNUM coupon schedule via COUPNCD/COUPPCD rather than three fixed date pairs), and from YIELDMAT (this exact same triangular relationship solved in the opposite direction: yld recovered from a known price, rather than price computed from a known yld).
//! tags: excel, pricemat, price, maturity, interest-at-maturity, security, bond, note, day-count, fraction, basis, settlement, issue, yield, finance
//! kernel_bank: on
//! entry: ExcelPricemat::run
//! limits: escalates (halt 0xFF06, out_of_domain) if rate < 0, yld < 0, dim_over_b <= 0, dsm_over_b <= 0, a_over_b < 0, or if the denominator (1 + dsm_over_b*yld) is <= 0; escalates (halt 0xFF07, float_overflow) if the result is infinite; escalates (halt 0xFF08, float_domain) if the result is NaN

// Excel signature: PRICEMAT(settlement, maturity, issue, rate, yld, [basis]).
// settlement, maturity, issue, rate, and yld are all required; basis is optional and
// defaults to 0 (US 30/360) when omitted. settlement/maturity/issue and the basis
// dispatch are all consumed upstream of this cell: the caller picks the day_count_*
// cell matching basis (day_count_30_360_us for 0, day_count_act_act for 1,
// day_count_act_360 for 2, day_count_act_365 for 3, day_count_30_360_eu for 4) and
// runs it three times over the same basis -- once over (issue, maturity) for DIM/B,
// once over (settlement, maturity) for DSM/B, once over (issue, settlement) for A/B --
// feeding the three resulting fractions in here as dim_over_b, dsm_over_b, and
// a_over_b. This cell only ever multiplies/divides by those three fractions once
// apiece; it never re-derives any day count itself.
// rate and yld are entered as plain positive decimals (e.g. 0.061 for 6.10%), matching
// Excel's own convention of #NUM! on a negative rate or yld -- there is no outflow-
// negative sign convention for either (PRICEMAT returns a quoted price per $100 face
// value, not a cash flow), and there is no type (0/1 annuity-due) argument for this
// function at all.
struct ExcelPricemat {
    rate: f32,
    yld: f32,
    dim_over_b: f32,
    dsm_over_b: f32,
    a_over_b: f32,
    price: f32,
}
impl ExcelPricemat {
    fn run(&mut self) -> u16 {
        if self.rate < 0.0f32 { halt(0xFF06u16); }
        if self.yld < 0.0f32 { halt(0xFF06u16); }
        // DIM (issue to maturity) and DSM (settlement to maturity) must both be
        // strictly positive -- a security's maturity always lies strictly after both
        // its issue date and its settlement date.
        if self.dim_over_b <= 0.0f32 { halt(0xFF06u16); }
        if self.dsm_over_b <= 0.0f32 { halt(0xFF06u16); }
        // A (issue to settlement) may be zero (settlement falling exactly on the
        // issue date) but never negative.
        if self.a_over_b < 0.0f32 { halt(0xFF06u16); }

        let denom = 1.0f32 + self.dsm_over_b * self.yld;
        if denom <= 0.0f32 { halt(0xFF06u16); }

        let numerator = 100.0f32 + self.dim_over_b * self.rate * 100.0f32;
        let term1 = numerator / denom;
        let term2 = self.a_over_b * self.rate * 100.0f32;
        let price = term1 - term2;

        if price.is_nan() { halt(0xFF08u16); }
        let fin = price.is_finite();
        if !fin { halt(0xFF07u16); }

        self.price = price;
        1u16
    }
}
