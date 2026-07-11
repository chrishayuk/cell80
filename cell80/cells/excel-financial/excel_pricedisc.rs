//! Price per $100 face value of a security sold at a simple discount (no periodic coupon), PRICEDISC = redemption*(1 - discount*DSM/B), taking the day-count fraction DSM/B as an already-computed dsm_over_b input from whichever basis-dispatched day_count_* cell Excel's basis argument selects (never re-derived here) -- distinct from DISC, which runs this exact same relationship in the opposite direction (a price turning into a discount rate, rather than a discount rate turning into a price), and from ACCRINT, whose per-period-rate*par*fraction formula has no leading (1 - ...) term or redemption scaling at all.
//! tags: excel, pricedisc, price, discount-rate, security, bond, treasury-bill, redemption, day-count, fraction, basis, finance
//! entry: ExcelPricedisc::run
//! limits: escalates (halt 0xFF06, out_of_domain) if discount <= 0, redemption <= 0, or dsm_over_b < 0; escalates (halt 0xFF07, float_overflow) if the result is infinite; escalates (halt 0xFF08, float_domain) if the result is NaN

// Excel signature: PRICEDISC(settlement, maturity, discount, redemption, [basis]).
// settlement, maturity, discount, and redemption are all required; basis is optional
// and defaults to 0 (US 30/360) when omitted. settlement/maturity and the basis
// dispatch are consumed upstream of this cell: the caller picks the day_count_* cell
// matching basis (day_count_30_360_us for 0, day_count_act_act for 1, day_count_act_360
// for 2, day_count_act_365 for 3, day_count_30_360_eu for 4), runs it over (settlement,
// maturity) to get DSM/B, and feeds that fraction in here as dsm_over_b -- this cell
// applies the resulting fraction once (discount*dsm_over_b), it never re-derives the
// day count itself.
// No outflow-negative sign convention applies (PRICEDISC returns a price per $100 face
// value, not a cash flow), and there is no type (0/1 annuity-due) argument for this
// function.
struct ExcelPricedisc {
    discount: f32,
    redemption: f32,
    dsm_over_b: f32,
    price: f32,
}
impl ExcelPricedisc {
    fn run(&mut self) -> u16 {
        if self.discount <= 0.0f32 { halt(0xFF06u16); }
        if self.redemption <= 0.0f32 { halt(0xFF06u16); }
        if self.dsm_over_b < 0.0f32 { halt(0xFF06u16); }

        let dxf = self.discount * self.dsm_over_b;
        let one_minus = 1.0f32 - dxf;
        let price = self.redemption * one_minus;

        if price.is_nan() { halt(0xFF08u16); }
        let fin = price.is_finite();
        if !fin { halt(0xFF07u16); }

        self.price = price;
        1u16
    }
}
