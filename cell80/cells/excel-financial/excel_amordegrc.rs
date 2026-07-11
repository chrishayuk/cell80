//! French declining-balance depreciation for one named period via a rate-to-coefficient lookup (life = 1/rate: <3yr -> 1.0, <5 -> 1.5, <=6 -> 2.0, >6 -> 2.5) applied to a shrinking book value across a bounded per-period loop, with the first period prorated by an already-computed day-count-basis fraction (dsm_over_b, produced upstream by whichever day_count_30_360_us/day_count_30_360_eu/day_count_act_act/day_count_act_360/day_count_act_365 cell Excel's basis argument selects) and a final-period 50%-of-remaining-book-value catch-up rule -- distinct from DB/DDB/VDB's plain month-based declining balance (no coefficient table, no day-count proration, no catch-up rule) and from AMORLINC (the straight-line French sibling, no declining coefficient or per-period loop at all).
//! tags: excel, amordegrc, depreciation, declining-balance, french-depreciation, coefficient-lookup, rate-to-coefficient, day-count, basis, prorated-period, book-value, catch-up, asset-life, finance
//! kernel_bank: on
//! entry: ExcelAmordegrc::run
//! limits: escalates (halt 0xFF06, out_of_domain) if cost <= 0, salvage < 0, rate <= 0, or dsm_over_b < 0; escalates (halt 0xFF07, float_overflow) if the result is infinite; escalates (halt 0xFF08, float_domain) if the result is NaN

// Excel signature: AMORDEGRC(cost, date_purchased, first_period, salvage, period, rate, [basis]).
// cost, date_purchased, first_period, salvage, period, and rate are all required --
// only basis is optional and defaults to 0 (US 30/360, NASD) when omitted, per Excel's
// own documented default. There is no outflow-negative sign convention here (unlike
// PV/FV/PMT): cost, salvage and the returned depreciation are always plain positive
// magnitudes. There is no annuity [type] flag either -- that convention is specific to
// the time-value-of-money family, not depreciation. date_purchased, first_period and
// basis are consumed upstream of this cell: the caller dispatches on basis to pick the
// matching day_count_* cell, runs it over (date_purchased, first_period), and feeds the
// resulting year-fraction in here as dsm_over_b (0.0 if the asset was purchased exactly
// at the first period boundary, up to just under 1.0 for a purchase date close to a full
// period before it). period is Excel's own period index: period == 0 asks for the
// prorated first (stub) period's depreciation directly; period == 1, 2, 3, ... ask for
// each subsequent whole period, which this cell derives by replaying every period from
// 0 up to the requested one (the same running-remainder idiom compound_decrease_by_bps
// uses, since AMORDEGRC has no closed form -- each period's depreciation depends on the
// book value left over from every period before it).
struct ExcelAmordegrc {
    cost: f32,
    salvage: f32,
    dsm_over_b: f32,
    rate: f32,
    period: u16,
    depreciation: f32,
}
impl ExcelAmordegrc {
    fn run(&mut self) -> u16 {
        if self.cost <= 0.0f32 { halt(0xFF06u16); }
        if self.salvage < 0.0f32 { halt(0xFF06u16); }
        if self.rate <= 0.0f32 { halt(0xFF06u16); }
        if self.dsm_over_b < 0.0f32 { halt(0xFF06u16); }

        // Depreciation coefficient from the asset's implied life (1/rate) -- the
        // French "amortissement dégressif" acceleration table: short-life assets
        // (already depreciating fast under straight-line) get no boost, long-life
        // assets get progressively more acceleration.
        let life = 1.0f32 / self.rate;
        let coeff = if life < 3.0f32 {
            1.0f32
        } else if life < 5.0f32 {
            1.5f32
        } else if life <= 6.0f32 {
            2.0f32
        } else {
            2.5f32
        };
        let dep_rate = self.rate * coeff;

        // Period 0: the prorated stub period between date_purchased and first_period,
        // scaled by the caller-supplied day-count-basis fraction.
        let first_raw = (self.dsm_over_b * dep_rate) * self.cost;
        let first_dep = first_raw.round();

        let mut cost_remaining = self.cost - first_dep;
        let mut rest = cost_remaining - self.salvage;
        let mut n_rate = first_dep;

        // Replay every whole period from 1 up to the requested one. `rest` tracks the
        // book value still above salvage under the plain declining-balance step (even
        // past zero, as a trigger signal only); once it would go negative the last
        // period(s) switch to a straight 50%-of-remaining-book-value catch-up instead
        // of a further declining-balance step, per AMORDEGRC's own documented rule.
        let mut per = 0u16;
        while per < self.period {
            let step_raw = dep_rate * cost_remaining;
            let step_rounded = step_raw.round();
            let rest_after = rest - step_rounded;
            let mut applied = step_rounded;
            if rest_after < 0.0f32 {
                let remaining_periods = self.period - per;
                if remaining_periods <= 1u16 {
                    applied = (cost_remaining * 0.5f32).round();
                } else {
                    applied = 0.0f32;
                }
            }
            cost_remaining = cost_remaining - applied;
            rest = rest_after;
            n_rate = applied;
            per = per + 1u16;
        }

        if n_rate.is_nan() { halt(0xFF08u16); }
        let fin = n_rate.is_finite();
        if !fin { halt(0xFF07u16); }

        self.depreciation = n_rate;
        1u16
    }
}
