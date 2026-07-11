//! Yield to maturity of a periodic-interest security (Excel YIELD(settlement, maturity, rate, pr, redemption, frequency, [basis])): bisects the annuity bond-price formula P=C*(1-(1+y/f)^-n)/(y/f)+Redemption*(1+y/f)^-n for y against a target built from pr plus accrued interest (coupon*dsm_over_b, the already-computed basis-dispatched day-count fraction elapsed since the last coupon date), with n (remaining coupon periods) taken as an already-computed COUPNUM-style input rather than re-walked here -- distinct from YIELDMAT (no periodic coupons at all, a single lump maturity payment, no iteration needed) and YIELDDISC (a discount-security division, no coupon stream and no iteration whatsoever), and from PRICE (runs this identical annuity relationship in the opposite direction, a known yield turning into a price, not a price turning into a yield).
//! tags: excel, yield, ytm, yield-to-maturity, bond, coupon, periodic-interest, price, iteration, bisection, annuity, day-count, fraction, accrued-interest, coupnum, basis, f32, finance
//! kernel_bank: on
//! entry: ExcelYield::run
//! limits: escalates (halt 0xFF06, out_of_domain) if rate < 0, pr <= 0, redemption <= 0, frequency isn't 1/2/4, num_coupons == 0, dsm_over_b is outside [0, 1), or the converged bisection result's implied price doesn't land within 0.01 of the target (the chosen [-50%, +1000%] bracket didn't actually contain a root); escalates (halt 0xFF05, needs_wider_math) if num_coupons exceeds 2000 (mirrors COUPNUM's own bounded-search ceiling); escalates (halt 0xFF07, float_overflow)/(halt 0xFF08, float_domain) on a non-finite/NaN result (unreachable in practice, kept for the same defensive convention every f32 cell in this pack follows); the 30-step bisection needs roughly 12-15 million cycles regardless of num_coupons (measured empirically; the inner (1+y/f)^n power is binary exponentiation, O(log2 n), not a flat n-count loop) -- about 6-7x the 2,000,000 default, so callers must pass a larger --cycles budget explicitly, the same cost-scaling convention is_prime_u32 already established for this library

// Excel signature: YIELD(settlement, maturity, rate, pr, redemption, frequency, [basis]).
// settlement, maturity, rate, pr, redemption, and frequency are all required (frequency
// must be 1 = annual, 2 = semiannual, or 4 = quarterly -- anything else is Excel's
// #NUM!); basis is optional and Excel defaults it to 0 (30/360 US) when omitted. rate,
// pr, and redemption are entered as plain positive numbers (rate a decimal, e.g. 0.08
// for 8%; pr and redemption per 100 face value) -- YIELD has no outflow-negative sign
// convention (it returns a rate, not a cash flow) and no [type] ordinary/due flag (a
// bond's coupons are always paid in arrears). settlement, maturity, and the optional
// basis are consumed upstream of this cell: the caller runs excel_coupnum(settlement,
// maturity, frequency) to get the remaining-coupon-period count and feeds it in here as
// num_coupons, and separately dispatches on basis to pick the matching day_count_* cell
// (day_count_30_360_us for 0, day_count_act_act for 1, day_count_act_360 for 2,
// day_count_act_365 for 3, day_count_30_360_eu for 4), runs it over (previous coupon
// date, settlement), and feeds the resulting elapsed-fraction in here as dsm_over_b
// (ACCRINT's own convention: 0.0 at the coupon date itself, up to just under 1.0 right
// before the next coupon) -- this cell only consumes those two already-derived values,
// it never re-walks the coupon schedule or re-derives a day count itself. There is no
// fractional-exponent kernel in this dialect (no pow-with-fractional-exponent), so the
// classic Excel formula's (1+y/f)^-(k-1+DSC/E) stub-period discounting is out of scope
// here; instead the elapsed fraction is folded in linearly as accrued interest added to
// pr (a dirty price), and the plain integer-exponent annuity formula is solved for y --
// exact whenever settlement lands on a coupon date (dsm_over_b == 0), a documented
// approximation otherwise.
struct ExcelYield {
    rate: f32,
    pr: f32,
    redemption: f32,
    frequency: u16,
    num_coupons: u16,
    dsm_over_b: f32,
    yld: f32,
}
impl ExcelYield {
    fn run(&mut self) -> u16 {
        if self.rate < 0.0f32 { halt(0xFF06u16); }
        if self.pr <= 0.0f32 { halt(0xFF06u16); }
        if self.redemption <= 0.0f32 { halt(0xFF06u16); }
        let freq_ok = self.frequency == 1u16 || self.frequency == 2u16 || self.frequency == 4u16;
        if !freq_ok { halt(0xFF06u16); }
        if self.num_coupons == 0u16 { halt(0xFF06u16); }
        if self.num_coupons > 2000u16 { halt(0xFF05u16); }
        if self.dsm_over_b < 0.0f32 || self.dsm_over_b >= 1.0f32 { halt(0xFF06u16); }

        let freq_f = int_to_f32(self.frequency);
        let n = self.num_coupons;
        let n_f = int_to_f32(n);
        let c_amt = 100.0f32 * self.rate / freq_f;

        // Dirty price: fold the already-elapsed fraction of the current coupon
        // period back in as accrued interest (ACCRINT's own linear formula,
        // coupon_amount * dsm_over_b), since the annuity formula below prices
        // the bond as of a coupon date, not mid-period.
        let accrued = c_amt * self.dsm_over_b;
        let target = self.pr + accrued;

        // Bisection brackets: -50% to +1000% per-annum yield, ample for any real
        // bond; bond price is monotonically decreasing in y for c_amt >= 0 and
        // redemption > 0, so the bracket always contains at most one root.
        let mut lo = -0.5f32;
        let mut hi = 10.0f32;
        let mut mid = 0.0f32;
        let mut price_mid = 0.0f32;

        let iterations = 30u16;
        let mut i = 0u16;
        while i < iterations {
            let m = (lo + hi) / 2.0f32;
            mid = m;
            let per = m / freq_f;
            let base = 1.0f32 + per;

            // base^n via binary exponentiation (O(log2 n) multiplies instead of
            // O(n)): squares b each step, folding it into pow_n whenever the
            // matching bit of n is set -- exact for integer n, and far cheaper
            // per bisection step than a flat n-count loop once n is more than a
            // handful of periods.
            let mut pow_n = 1.0f32;
            let mut b = base;
            let mut k = n;
            while k > 0u16 {
                let bit = k % 2u16;
                if bit == 1u16 {
                    pow_n = pow_n * b;
                }
                b = b * b;
                k = k / 2u16;
            }
            let inv_pow_n = 1.0f32 / pow_n;
            let is_zero = per == 0.0f32;
            let annuity_factor = if is_zero { n_f } else { (1.0f32 - inv_pow_n) / per };
            let p = c_amt * annuity_factor + self.redemption * inv_pow_n;
            price_mid = p;

            if p > target {
                lo = m;
            } else {
                hi = m;
            }
            i = i + 1u16;
        }

        let diff = price_mid - target;
        let abs_diff = if diff < 0.0f32 { 0.0f32 - diff } else { diff };
        if abs_diff > 0.01f32 { halt(0xFF06u16); }

        if mid.is_nan() { halt(0xFF08u16); }
        let fin = mid.is_finite();
        if !fin { halt(0xFF07u16); }

        self.yld = mid;
        1u16
    }
}
