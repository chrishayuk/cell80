//! Excel VDB: variable-declining-balance depreciation summed over an arbitrary [start_period,end_period] sub-range (periods here are *1000 permille fields so fractional boundaries like 0.875 stay exact), switching each period to straight-line-of-remaining-book-value once that exceeds the declining-balance step unless no_switch suppresses it -- cost/salvage/life/start_period/end_period are all required, factor is Excel's one defaultable numeric arg (omit it -> factor_x100=200, i.e. 2.0 double-declining) and no_switch is its one defaultable flag (omit it -> 0, switching allowed), output is always non-negative (no outflow-negative sign convention here, unlike PV/FV/PMT) -- distinct from DB/DDB (one fixed period, no caller-chosen range) and SYD (a different formula with no switch-over case analysis at all).
//! tags: excel, depreciation, vdb, declining-balance, variable-declining-balance, double-declining, straight-line, switch-over, prorate, period-range, sub-range, factor, no-switch, permille, checked, wide, u32
//! entry: ExcelVdb::run
//! limits: escalates (halt 0xFF06, out_of_domain) if life == 0, factor_x100 == 0, start_permille > end_permille, or end_permille exceeds life*1000; escalates (halt 0xFF05, needs_wider_math) the moment any per-period multiply or the running total would overflow u32
struct ExcelVdb {
    cost: u32,
    salvage: u32,
    life: u16,
    start_permille: u32,
    end_permille: u32,
    factor_x100: u16,
    no_switch: u16,
    depreciation: u32,
}
impl ExcelVdb {
    fn run(&mut self) -> u16 {
        if self.life == 0u16 { halt(0xFF06u16); }
        if self.factor_x100 == 0u16 { halt(0xFF06u16); }
        if self.start_permille > self.end_permille { halt(0xFF06u16); }
        let life32 = self.life as u32;
        let life_permille = life32 * 1000u32;
        if self.end_permille > life_permille { halt(0xFF06u16); }
        let life_x100 = life32 * 100u32;

        let whole_end = self.end_permille / 1000u32;
        let rem_end = self.end_permille % 1000u32;
        let num_periods = if rem_end != 0u32 { whole_end + 1u32 } else { whole_end };

        let mut bv = self.cost;
        let mut total = 0u32;
        let mut t = 1u32;
        while t <= num_periods {
            let t_lo = (t - 1u32) * 1000u32;
            let t_hi = t * 1000u32;
            let ov_start = if self.start_permille > t_lo { self.start_permille } else { t_lo };
            let ov_end = if self.end_permille < t_hi { self.end_permille } else { t_hi };
            let overlap = if ov_end > ov_start { ov_end - ov_start } else { 0u32 };

            let remaining = life32 - t + 1u32;
            let remroom = if bv > self.salvage { bv - self.salvage } else { 0u32 };

            let numerator = mul_checked_u32(bv, self.factor_x100 as u32);
            let db_raw = numerator / life_x100;
            let db_amount = if db_raw > remroom { remroom } else { db_raw };
            let sl_amount = remroom / remaining;

            let switch_allowed = self.no_switch == 0u16;
            let use_sl = switch_allowed && (sl_amount > db_amount);
            let chosen = if use_sl { sl_amount } else { db_amount };

            bv = bv - chosen;

            let contrib_raw = mul_checked_u32(chosen, overlap);
            let contrib = contrib_raw / 1000u32;
            total = add_checked_u32(total, contrib);

            t = t + 1u32;
        }

        self.depreciation = total;
        1u16
    }
}
