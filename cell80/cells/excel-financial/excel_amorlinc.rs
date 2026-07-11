//! French straight-line depreciation for a single accounting period, taking the day-count fraction between date_purchased and first_period as an already-computed input (the caller runs whichever basis-dispatched day-count cell -- day_count_30_360_us/eu, day_count_act_act, day_count_act_360, or day_count_act_365 -- that Excel's basis argument selects, then feeds its output in here as dp_fp_over_b), prorating only period 0's depreciation by that fraction, then paying a flat cost*rate amount per full period until exactly one final period is capped to whatever remains of cost-salvage, after which every later period returns zero -- distinct from AMORDEGRC (same French-depreciation family but a declining-balance amount driven by a life-dependent coefficient table, not a flat straight-line rate) and from SYD/SLN (neither of which prorates by a purchase-date day-count fraction or caps a specific final period against a running cumulative total).
//! tags: excel, amorlinc, depreciation, straight-line, french, prorated, first-period, day-count, fraction, cost, salvage, asset, capped
//! kernel_bank: on
//! entry: ExcelAmorlinc::run
//! limits: escalates (halt 0xFF06, out_of_domain) if cost <= 0, rate <= 0, salvage < 0, or dp_fp_over_b < 0; escalates (halt 0xFF07, float_overflow) if the result is infinite; escalates (halt 0xFF08, float_domain) if the result is NaN

// Excel signature: AMORLINC(cost, date_purchased, first_period, salvage, period, rate, [basis]).
// cost, date_purchased, first_period, salvage, period, and rate are all required;
// basis is optional and Excel defaults it to 0 (30/360 US) when omitted. Depreciation
// amounts are always non-negative here -- there is no outflow-negative sign convention
// and no [type] ordinary/due branch (that's an annuity concept, this is depreciation).
// date_purchased, first_period, and the optional basis are consumed upstream of this
// cell: the caller dispatches on basis to pick the matching day_count_* cell, runs it
// over (date_purchased, first_period), and feeds the resulting day-count fraction in
// here as dp_fp_over_b (the fraction of a full accounting year between the purchase
// date and the end of the first period, under whichever basis convention Excel's basis
// code selects -- 0 = day_count_30_360_us, 1 = day_count_act_act, 2 = day_count_act_360,
// 3 = day_count_act_365, 4 = day_count_30_360_eu). period is Excel's zero-based period
// index (period 0 is the prorated first period); callers must pass INT(period) since
// this cell takes it as an already-integral u16.
struct ExcelAmorlinc {
    cost: f32,
    salvage: f32,
    rate: f32,
    dp_fp_over_b: f32,
    period: u16,
    depreciation: f32,
}
impl ExcelAmorlinc {
    fn run(&mut self) -> u16 {
        if self.cost <= 0.0f32 { halt(0xFF06u16); }
        if self.rate <= 0.0f32 { halt(0xFF06u16); }
        if self.salvage < 0.0f32 { halt(0xFF06u16); }
        if self.dp_fp_over_b < 0.0f32 { halt(0xFF06u16); }

        let cost_delta = self.cost * self.rate;
        let avg_amount = cost_delta * self.dp_fp_over_b;

        if self.period == 0u16 {
            if avg_amount.is_nan() { halt(0xFF08u16); }
            let fin0 = avg_amount.is_finite();
            if !fin0 { halt(0xFF07u16); }
            self.depreciation = avg_amount;
            return 1u16;
        }

        // rest_value is the remaining depreciable base after the prorated first
        // period; dividing by cost_delta and flooring finds how many more *full*
        // periods fit before a final, smaller, capped period is needed.
        let rest_value = self.cost - avg_amount - self.salvage;
        let ratio = rest_value / cost_delta;
        let num_full_periods = ratio.floor();
        let period_f = int_to_f32(self.period);
        let boundary = num_full_periods + 1.0f32;

        let result = if period_f <= num_full_periods {
            cost_delta
        } else if period_f == boundary {
            rest_value - cost_delta * num_full_periods
        } else {
            0.0f32
        };

        if result.is_nan() { halt(0xFF08u16); }
        let fin = result.is_finite();
        if !fin { halt(0xFF07u16); }

        self.depreciation = result;
        1u16
    }
}
