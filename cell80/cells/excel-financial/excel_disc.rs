//! Simple-discount rate for a security priced below its redemption value, DISC = (redemption-pr)/redemption * (B/DSM), taking the day-count fraction DSM/B as an already-computed dsm_over_b input from whichever basis-dispatched day_count_* cell Excel's basis argument selects (inverted here with a single division, never re-derived) -- distinct from INTRATE and YIELDDISC, whose near-identical formula divides by pr (purchase price) instead of redemption (face/maturity value), and from PRICEDISC, which runs this same relationship in the opposite direction (a discount rate turning into a price, rather than a price turning into a discount rate).
//! tags: excel, disc, discount-rate, security, bond, treasury-bill, redemption, day-count, fraction, basis, price, finance
//! entry: ExcelDisc::run
//! limits: escalates (halt 0xFF06, out_of_domain) if pr <= 0, redemption <= 0, or dsm_over_b <= 0; escalates (halt 0xFF07, float_overflow) if the result is infinite; escalates (halt 0xFF08, float_domain) if the result is NaN

// Excel signature: DISC(settlement, maturity, pr, redemption, [basis]).
// settlement, maturity, pr, and redemption are all required; basis is optional and
// defaults to 0 (US 30/360) when omitted. settlement/maturity and the basis dispatch
// are consumed upstream of this cell: the caller picks the day_count_* cell matching
// basis (day_count_30_360_us for 0, day_count_act_act for 1, day_count_act_360 for 2,
// day_count_act_365 for 3, day_count_30_360_eu for 4), runs it over (settlement,
// maturity) to get DSM/B, and feeds that fraction in here as dsm_over_b -- this cell
// inverts it once (B/DSM = 1/dsm_over_b) rather than re-deriving the day count.
// No outflow-negative sign convention applies (DISC returns a discount rate, not a
// cash flow), and there is no type (0/1 annuity-due) argument for this function.
struct ExcelDisc {
    pr: f32,
    redemption: f32,
    dsm_over_b: f32,
    disc: f32,
}
impl ExcelDisc {
    fn run(&mut self) -> u16 {
        if self.pr <= 0.0f32 { halt(0xFF06u16); }
        if self.redemption <= 0.0f32 { halt(0xFF06u16); }
        if self.dsm_over_b <= 0.0f32 { halt(0xFF06u16); }

        let diff = self.redemption - self.pr;
        let ratio = diff / self.redemption;
        let disc = ratio / self.dsm_over_b;

        if disc.is_nan() { halt(0xFF08u16); }
        let fin = disc.is_finite();
        if !fin { halt(0xFF07u16); }

        self.disc = disc;
        1u16
    }
}
