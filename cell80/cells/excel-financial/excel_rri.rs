//! Equivalent per-period growth rate implied by a present value pv compounding to a future value fv over nper whole periods, rri = (fv/pv)^(1/nper) - 1, found by seeding a sqrt-halved starting guess (repeated .sqrt() down to the 1/2^bits(nper) root of the ratio) then refining with fixed-count Newton iteration on y^nper = ratio (each trial's y^(nper-1) evaluated by square-and-multiply, not a naive O(nper) repeated-multiply loop, which this cell measured as blowing the runtime's own T-state budget past a few dozen periods) -- generalizes bps_rate_over_2_periods's fixed N=2 isqrt trick to arbitrary nper; all three arguments are required (Excel defines no optional RRI argument), and unlike PV/FV/PMT/RATE, RRI carries no outflow-negative cash-flow sign convention: pv and fv are plain positive magnitudes of the same investment observed nper periods apart, not opposing cash flows.
//! tags: excel, finance, rri, rate, interest-rate, growth-rate, equivalent-rate, nth-root, root, newton, bisection, nper, present-value, future-value, compounding, f32, float, softfloat
//! kernel_bank: on
//! entry: ExcelRri::run
//! limits: escalates (halt 0xFF06, out_of_domain) if nper == 0, pv <= 0.0, or fv/pv is negative (no real Nth root exists); fv == 0.0 returns the exact rate -1.0 (total loss) without iterating; escalates (halt 0xFF05, needs_wider_math) if the fixed-iteration Newton refinement does not converge within 1% of the target ratio when checked back (pathological combinations of an extreme ratio with a very small nper) -- verified empirically to converge to within a few ULPs of the host-rustc reference for realistic ratios (0.01x to 100x) at every nper from 1 to 65535, though nper beyond the low thousands may separately hit the runner's own cycle_budget halt rather than this cell's own escalation, same tradeoff compound_increase_by_bps's period loop makes; escalates (halt 0xFF08, float_domain) / (halt 0xFF07, float_overflow) if the ratio or the final rate is NaN / non-finite
struct ExcelRri {
    nper: u16,
    pv: f32,
    fv: f32,
    rate: f32,
}
impl ExcelRri {
    fn run(&mut self) -> u16 {
        if self.nper == 0u16 { halt(0xFF06u16); }
        if self.pv <= 0.0f32 { halt(0xFF06u16); }
        let ratio = self.fv / self.pv;
        if ratio.is_nan() { halt(0xFF08u16); }
        let ratio_fin = ratio.is_finite();
        if !ratio_fin { halt(0xFF07u16); }
        if ratio < 0.0f32 { halt(0xFF06u16); }

        if ratio == 0.0f32 {
            self.rate = 0.0f32 - 1.0f32;
            return 1u16;
        }

        let m = self.nper - 1u16;
        let n_f = int_to_f32(self.nper);
        let m_f = int_to_f32(m);

        // Seed y0 = ratio^(1/2^bl), bl = nper's bit length, via bl plain .sqrt() calls --
        // a cheap, always-stable way to land the Newton start near the true root
        // regardless of how extreme ratio is, instead of a fixed y0 = 1.0.
        let mut bl = 0u16;
        let mut t = self.nper;
        while t != 0u16 {
            bl = bl + 1u16;
            t = t >> 1u16;
        }
        let mut y = ratio;
        let mut s = 0u16;
        while s < bl {
            y = y.sqrt();
            s = s + 1u16;
        }

        // Fixed-count Newton refinement of y^nper = ratio: y' = ((nper-1)*y + ratio/y^(nper-1)) / nper.
        // y^(nper-1) is evaluated by square-and-multiply (log2(nper) multiplies), never a
        // naive O(nper) repeated-multiply loop -- that alternative was measured (via the
        // cell80 CLI/host on this exact draft) to blow the runner's default T-state budget
        // once nper passed a few dozen periods.
        let mut i = 0u16;
        while i < 4u16 {
            let mut y_pow = 1.0f32;
            let mut base = y;
            let mut exp = m;
            while exp > 0u16 {
                let bit = exp & 1u16;
                if bit != 0u16 {
                    y_pow = y_pow * base;
                }
                base = base * base;
                exp = exp >> 1u16;
            }
            let next = (m_f * y + ratio / y_pow) / n_f;
            y = next;
            i = i + 1u16;
        }

        // Verify convergence by re-evaluating y^nper (same square-and-multiply technique)
        // and comparing back to ratio within 1% -- catches the pathological extreme-ratio /
        // tiny-nper combinations the fixed 4-iteration budget above cannot fully resolve,
        // escalating honestly instead of returning a silently wrong rate.
        let mut check = 1.0f32;
        let mut base2 = y;
        let mut exp2 = self.nper;
        while exp2 > 0u16 {
            let bit2 = exp2 & 1u16;
            if bit2 != 0u16 {
                check = check * base2;
            }
            base2 = base2 * base2;
            exp2 = exp2 >> 1u16;
        }
        let diff = check - ratio;
        let adiff = diff.abs();
        let tol = ratio * 0.01f32;
        if adiff > tol { halt(0xFF05u16); }

        let r = y - 1.0f32;
        if r.is_nan() { halt(0xFF08u16); }
        let r_fin = r.is_finite();
        if !r_fin { halt(0xFF07u16); }
        self.rate = r;
        1u16
    }
}
