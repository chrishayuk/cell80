//! Fixed-declining-balance depreciation for one period (Excel's DB(cost,salvage,life,period,[month]), month omittable and defaulting to 12): the rate 1-(salvage/cost)^(1/life) is recovered by an adaptive-early-exit Newton iteration on x^life=ratio (int_to_f32 plus binary-exponentiation power, since no native fractional-exponent pow or O(life) power loop is affordable here), the first period pro-rated by month/12, then a cheap total=total*x+k recurrence compounds forward through the requested period and returns only that period's delta -- distinct from compound_decrease_by_bps's loop (money-bps pack), which applies an already-known bps rate for N periods rather than deriving the rate itself via an nth root, and distinct from Excel's own DDB sibling, whose rate is a fixed 2/life factor with no root-finding at all.
//! tags: excel, financial, depreciation, declining-balance, fixed-declining-balance, db, rate, newton, nth-root, period, month, prorate, f32, softfloat
//! kernel_bank: on
//! entry: ExcelDb::run
//! limits: escalates (halt 0xFF06, out_of_domain) if cost <= 0, salvage < 0, life == 0, month is outside 1..12, or period is outside 1..=life+1; escalates (halt 0xFF08, float_domain) if the Newton root or the period recurrence produces NaN, or (halt 0xFF07, float_overflow) if either produces a non-finite result; large life *and* period together (roughly both past the high-20s) can exhaust the runner's own cycle budget before returning -- a soft ceiling from the O(life) Newton work plus the O(period) compounding walk, not a designed escalation
struct ExcelDb {
    cost: f32,
    salvage: f32,
    life: u16,
    period: u16,
    month: u16,
    depreciation: f32,
}
impl ExcelDb {
    fn run(&mut self) -> u16 {
        if self.cost <= 0.0f32 { halt(0xFF06u16); }
        if self.salvage < 0.0f32 { halt(0xFF06u16); }
        if self.life == 0u16 { halt(0xFF06u16); }
        if self.month < 1u16 || self.month > 12u16 { halt(0xFF06u16); }
        let life_plus_1 = self.life.saturating_add(1u16);
        if self.period < 1u16 || self.period > life_plus_1 { halt(0xFF06u16); }

        let ratio = self.salvage / self.cost;
        if ratio.is_nan() { halt(0xFF08u16); }

        // Newton-iterate x^life = ratio for x (the (1/life)-th root of ratio), via
        // x_{k+1} = ((life-1)*x_k + ratio/x_k^(life-1)) / life. Start at x0 = 1.0
        // rather than at `ratio` itself -- starting from `ratio` badly overshoots
        // for life > 2 (x_k^(life-1) shrinks fast, blowing up the correction term).
        // life == 1 converges in a single step regardless of the start since x^0
        // == 1 makes the update collapse to exactly `ratio`. x_k^(life-1) is built
        // via binary exponentiation (bit-test-and-square, O(log2(life)) multiplies)
        // rather than a plain O(life) loop (frac_pow's technique) -- life can run
        // into the tens, and repeating an O(life) power inside every outer Newton
        // step would multiply that cost, not just add it. The loop itself exits
        // early once the step stops moving (`converged`), so cheap/typical ratios
        // (most of them) don't pay for iterations they don't need; the fixed cap
        // of 10 exists only to bound the pathological ones.
        let life_f = int_to_f32(self.life);
        let life_recip = 1.0f32 / life_f;
        let life_minus_1 = life_f - 1.0f32;
        let exponent = self.life - 1u16;
        let mut x = 1.0f32;
        let mut converged = 0u16;
        let mut ni = 0u16;
        while ni < 10u16 {
            if converged == 0u16 {
                let mut x_pow = 1.0f32;
                let mut base = x;
                let mut e = exponent;
                while e > 0u16 {
                    let bit_set = e & 1u16;
                    if bit_set == 1u16 {
                        x_pow = x_pow * base;
                    }
                    base = base * base;
                    e = e >> 1u16;
                }
                let term = life_minus_1 * x + ratio / x_pow;
                let x_new = term * life_recip;
                let step_delta = x_new - x;
                let step_mag = step_delta.abs();
                if step_mag < 0.00001f32 {
                    converged = 1u16;
                }
                x = x_new;
            }
            ni = ni + 1u16;
        }

        if x.is_nan() { halt(0xFF08u16); }
        let x_fin = x.is_finite();
        if !x_fin { halt(0xFF07u16); }

        let rate = 1.0f32 - x;
        let k = self.cost * rate;

        // First period, pro-rated by month/12.
        let month_f = int_to_f32(self.month);
        let first = k * month_f / 12.0f32;

        let mut total = first;
        let mut delta = first;
        if self.period >= 2u16 {
            // Every period strictly between 1 and the requested one is a full,
            // un-prorated declining-balance step -- period == life_plus_1 (the
            // only *other* prorated period, the trailing stub) can only ever be
            // the requested period itself, since the domain check above caps
            // period at life_plus_1. So this recurrence is always safe for the
            // walk-up: total_new = total*x + k, because
            // total + (cost-total)*rate == total*(1-rate) + cost*rate == total*x + k
            // -- one multiply-add instead of a subtract-multiply-add each step.
            let mut p = 2u16;
            while p < self.period {
                total = total * x + k;
                p = p + 1u16;
            }
            let remaining = self.cost - total;
            let mut step = remaining * rate;
            if self.period == life_plus_1 {
                let stub_months_u = 12u16 - self.month;
                let stub_months = int_to_f32(stub_months_u);
                step = remaining * rate * stub_months / 12.0f32;
            }
            delta = step;
        }

        if delta.is_nan() { halt(0xFF08u16); }
        let d_fin = delta.is_finite();
        if !d_fin { halt(0xFF07u16); }

        self.depreciation = delta;
        1u16
    }
}
