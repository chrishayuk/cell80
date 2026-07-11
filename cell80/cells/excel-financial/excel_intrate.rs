//! Interest rate earned by a fully invested (non-discounted, no periodic coupon) security bought at investment and redeemed at redemption: rate = (redemption-investment)/investment * (B/DIM), where DIM/B is the settlement-to-maturity day-count fraction already computed by whichever basis-dispatched day-count cell (day_count_30_360_us/eu, day_count_act_act, day_count_act_360, day_count_act_365) Excel's basis argument selects, fed in here as dim_over_b -- distinct from DISC (an outwardly similar formula shape that divides by redemption, the face value received back, not investment, the price actually paid up front) and from ACCRINT/ACCRINTM (which accrue coupon interest already declared by a stated rate, rather than solving for the single implied rate of a zero-coupon-style security). Confirmed behaviourally identical to YIELDDISC (Excel's own well-known quirk: (redemption-pr)/pr*(B/DSM) is the exact same formula as INTRATE's (redemption-investment)/investment*(B/DIM), just renaming investment/DIM to pr/DSM) — the admission gate's fingerprint check caught this directly; YIELDDISC's vocabulary is folded into this cell's tags rather than shipping a second, functionally-identical cell (docs/library-growth.md: no behavioural duplicates).
//! tags: excel, intrate, interest-rate, fully-invested-security, non-coupon, zero-coupon, discount-security, treasury-bill, bond, day-count, redemption, investment, settlement, maturity, basis, security, yielddisc, yield, purchase-price, pr
//! entry: ExcelIntrate::run
//! limits: escalates (halt 0xFF06, out_of_domain) if investment <= 0, redemption <= 0, or dim_over_b <= 0 (dim_over_b <= 0 covers settlement >= maturity, since that would drive the upstream day-count cell's own day-count fraction to zero or negative); escalates (halt 0xFF07, float_overflow) if the result is infinite; escalates (halt 0xFF08, float_domain) if the result is NaN

// Excel signature: INTRATE(settlement, maturity, investment, redemption, [basis]).
// settlement, maturity, investment, and redemption are all required (Excel's #NUM! if
// investment <= 0 or redemption <= 0, #VALUE! if settlement >= maturity). basis is
// optional, defaulting to 0 (30/360 US) when omitted, and is consumed entirely upstream
// of this cell: the caller dispatches on basis to pick the matching day_count_* cell
// (day_count_30_360_us for 0, day_count_act_act for 1, day_count_act_360 for 2,
// day_count_act_365 for 3, day_count_30_360_eu for 4), runs it over (settlement,
// maturity), and feeds the resulting day-count fraction in here as dim_over_b (days
// between settlement and maturity, in the chosen convention, over the basis
// denominator B -- e.g. 0.25 for a 90-day/360-day quarter). investment and redemption
// are plain positive amounts (the price paid and the amount received back at
// maturity), not signed cash flows -- INTRATE has no outflow-negative convention and
// no annuity type (0/1) flag; both are irrelevant to a single lump investment/redemption
// pair.
struct ExcelIntrate {
    investment: f32,
    redemption: f32,
    dim_over_b: f32,
    rate: f32,
}
impl ExcelIntrate {
    fn run(&mut self) -> u16 {
        if self.investment <= 0.0f32 { halt(0xFF06u16); }
        if self.redemption <= 0.0f32 { halt(0xFF06u16); }
        if self.dim_over_b <= 0.0f32 { halt(0xFF06u16); }

        let diff = self.redemption - self.investment;
        let ratio = diff / self.investment;
        let rate = ratio / self.dim_over_b;

        if rate.is_nan() { halt(0xFF08u16); }
        let fin = rate.is_finite();
        if !fin { halt(0xFF07u16); }

        self.rate = rate;
        1u16
    }
}
