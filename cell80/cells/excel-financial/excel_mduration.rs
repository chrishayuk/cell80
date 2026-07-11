//! Modified duration of a coupon-paying bond (Excel MDURATION = Macaulay duration / (1+yld/frequency)), built as a bounded weighted discounted-cash-flow sum over num_periods whole coupon periods (N, a COUPNUM-style whole-period count derived upstream from settlement/maturity/frequency and fed in directly, never re-stepped here) where each term is (k/frequency)*CF_k/(1+yld/frequency)^k via a frac_pow-style iterative f32 compounding loop -- distinct from plain DURATION (the undivided Macaulay value, itself host_only/array-state and not built in this pack) and from COUPNUM (which only counts N, never discounts a cash flow against it).
//! tags: excel, mduration, modified-duration, macaulay-duration, bond, duration, coupon, yield, frequency, weighted-average-life, present-value, discounted-cash-flow, f32
//! kernel_bank: on
//! entry: ExcelMduration::run
//! limits: escalates (halt 0xFF06, out_of_domain) if frequency isn't 1, 2, or 4; if num_periods is 0 or exceeds 2400 (ample for any realistic bond term at any supported frequency, mirroring COUPNCD/COUPDAYS' own loop bounds); if coupon < 0; or if yld < 0 (matching Excel's own #NUM! conditions exactly). Escalates (halt 0xFF07, float_overflow) if the result is infinite, (halt 0xFF08, float_domain) if it's NaN.

// Excel signature: MDURATION(settlement, maturity, coupon, yld, frequency, [basis]).
// coupon, yld, and frequency are required (frequency must be 1/annual, 2/semiannual,
// or 4/quarterly -- anything else is Excel's #NUM!); coupon and yld are entered as
// decimal annual rates (e.g. 0.08 for 8%), coupon == 0.0 is a valid zero-coupon case.
// basis is optional in real Excel, defaulting to 0 (US 30/360) when omitted -- it is
// NOT a field on this cell at all, because MDURATION's whole-period count N is pure
// calendar arithmetic (how many coupon periods fit between settlement and maturity),
// independent of which day-count convention prices a partial period; COUPNCD's own
// doc comment makes the same point about basis never changing which date IS a coupon
// date. There is no par/redemption argument in MDURATION's real signature at all
// (unlike PRICE/YIELD): par is implicitly 100 always, baked into the redemption cash
// flow at the final period below. No outflow-negative sign convention applies here --
// MDURATION returns a positive duration/risk measure, not a signed cash flow -- and no
// annuity type (0/1) flag exists for this function either.
//
// settlement and maturity themselves are consumed upstream of this cell: the caller
// derives num_periods (N), the whole number of coupon periods from settlement to
// maturity, via a COUPNUM-style backward date-stepping walk (the same technique
// COUPNCD/COUPDAYS already inline) and feeds the resulting count in here as a plain
// scalar, exactly the way ACCRINT consumes an already-computed dsm_over_b instead of
// re-deriving it from raw dates -- finding N is COUPNUM's job, not MDURATION's, so the
// date-stepping loop itself is not duplicated in this cell.
//
// Scope simplification (matching ACCRINT's own single-accrual-period precedent): this
// implements the whole-period Macaulay/modified duration formula, i.e. settlement is
// assumed to fall exactly on a coupon date, so every one of the N periods is a full
// period and no fractional first-period (DSC/E) adjustment is applied. Excel's real
// MDURATION folds in that fractional first period when settlement falls between
// coupon dates; that finer adjustment is out of scope here.
struct ExcelMduration {
    num_periods: u16,
    coupon: f32,
    yld: f32,
    frequency: u16,
    mduration: f32,
}
impl ExcelMduration {
    fn run(&mut self) -> u16 {
        let freq_ok = self.frequency == 1u16 || self.frequency == 2u16 || self.frequency == 4u16;
        if !freq_ok { halt(0xFF06u16); }
        if self.num_periods == 0u16 { halt(0xFF06u16); }
        if self.num_periods > 2400u16 { halt(0xFF06u16); }
        if self.coupon < 0.0f32 { halt(0xFF06u16); }
        if self.yld < 0.0f32 { halt(0xFF06u16); }

        let freq_f = int_to_f32(self.frequency);
        let per_period_coupon = 100.0f32 * self.coupon / freq_f;
        let r = self.yld / freq_f;
        let one_plus_r = 1.0f32 + r;

        // frac_pow-style idiom: no native pow-with-fractional-or-large-integer-exponent
        // kernel exists, so (1+r)^k is built the same way frac_pow builds n^k -- one
        // more multiply per period, chained onto the previous period's running power
        // rather than recomputed from scratch each time.
        let mut pow_val = 1.0f32;
        let mut price = 0.0f32;
        let mut weighted = 0.0f32;
        let mut k = 1u16;
        while k <= self.num_periods {
            pow_val = pow_val * one_plus_r;
            let is_last = k == self.num_periods;
            let redemption = if is_last { 100.0f32 } else { 0.0f32 };
            let cf = per_period_coupon + redemption;
            let df_cf = cf / pow_val;
            price = price + df_cf;
            let k_f = int_to_f32(k);
            let time_years = k_f / freq_f;
            weighted = weighted + time_years * df_cf;
            k = k + 1u16;
        }

        let mac_duration = weighted / price;
        let mdur = mac_duration / one_plus_r;

        if mdur.is_nan() { halt(0xFF08u16); }
        let fin = mdur.is_finite();
        if !fin { halt(0xFF07u16); }

        self.mduration = mdur;
        1u16
    }
}
