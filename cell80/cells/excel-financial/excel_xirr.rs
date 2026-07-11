//! Excel XIRR(values, dates, [guess]): internal rate of return for a cash-flow stream on IRREGULAR (non-periodic) dates -- the rate where XNPV crosses zero, composing exactly two techniques this pack already owns rather than inventing a third: XNPV's own per-flow discounting (each flow divides by (1+rate)^(days_i/365), computed as exp(-(days_i/365)*ln(1+rate)) through the owned F2 fexp/fln, days_i the flow's day offset from the first date with days[0] == 0 by convention -- the caller runs excel_days/days_between per flow against the first date upstream, exactly as excel_xnpv.rs's own feed-in convention) and IRR's bounded SECANT root-find (two seeded XNPV evaluations at guess and guess+0.1, then a fixed allowance of further walks, convergence checked on the iterate -- |r2-r1| < 2e-4 -- before paying the next evaluation). Priced, not killed: excel_xnpv.rs already flagged this cell as the one that "would iterate THIS entire evaluation per secant step" (docs/excel-financial-map.md), since every XIRR evaluation IS a full XNPV pass and every flow in that pass pays a real fexp (~330K T-states). The envelope is capped at 4 flows -- XNPV's own smallest-in-the-pack envelope, for the identical reason -- and the walk allowance is capped at 4 secant steps after the 2 seed evaluations (6 full XNPV passes total), rather than IRR's own 8-flow/5-walk allowance, because each pass here costs roughly 5x what a plain NPV pass costs (a fln plus N fexp calls versus N plain divisions). Distinct from XNPV (a single evaluation at a caller-supplied rate, no root-find) and from IRR (whole-period indices, no per-flow transcendental, so it can afford a bigger envelope and more walks).
//! tags: excel, xirr, internal-rate-of-return, irregular, dates, day-count, cash-flow, discount, rate, secant, root-finding, iterative, transcendental, exp, ln, array, finance, f32
//! kernel_bank: on
//! entry: ExcelXirr::run
//! limits: fixed 4-slot cash-flow envelope (XNPV's own envelope -- every secant step pays a full XNPV pass, and every flow in that pass pays a real fexp), not caller-configurable; escalates (halt 0xFF06, out_of_domain) if count < 2 or count > 4, if an iterate leaves the rate domain (1+r <= 0, ln's domain), if the secant degenerates (flat XNPV between iterates), or if the fixed 4-walk allowance (after the 2 seed evaluations) doesn't converge; escalates (halt 0xFF08, float_domain) on a NaN result, (halt 0xFF07, float_overflow) on a non-finite one; guess == 0.0 is treated as omitted and defaults to Excel's own 0.1, the same convention excel_irr.rs already established. Cycle cost, worked explicitly rather than pretending this fits the default budget (the same "priced, not killed" discipline excel_yield.rs's own manifest documents for its 12-15M-cycle need): each full XNPV evaluation costs 1 fln (~330K T, ln(1+rate) computed once per evaluation) plus up to 4 fexp calls (~330K T each, one per flow) = ~1.65M T-states per evaluation; the bounded secant pays up to 6 such evaluations (2 seeds + up to 4 walks) = ~9.9M T-states worst case, roughly 5x the 2,000,000 default -- measured directly on the emulator, a 4-flow stream needing 3 walks (5 evaluations) already costs ~6.9M T-states, so callers must pass a larger --cycles budget explicitly (12,000,000 verified sufficient; the same cost-scaling convention excel_yield/is_prime_u32 already established for this library).
struct ExcelXirr {
    values: [u32; 4],
    days: [u32; 4],
    count: u16,
    guess: f32,
    xirr: f32,
}
impl ExcelXirr {
    fn run(&mut self) -> u16 {
        if self.count < 2u16 { halt(0xFF06u16); }
        if self.count > 4u16 { halt(0xFF06u16); }

        let mut r0 = self.guess;
        if r0 == 0.0f32 { r0 = 0.1f32; }
        let mut r1 = r0 + 0.1f32;
        if 1.0f32 + r0 <= 0.0f32 { halt(0xFF06u16); }

        // Seed evaluation at r0: a full XNPV pass (XNPV's own formula, inlined) --
        // one fln to get ln(1+r0), reused across every flow's fexp.
        let base0 = 1.0f32 + r0;
        if base0 <= 0.0f32 { halt(0xFF06u16); }
        let neg_k0 = (0.0f32 - base0.ln()) / 365.0f32;
        let mut f0 = 0.0f32;
        let mut i = 0u16;
        while i < self.count {
            let t0 = int_to_f32(self.days[i as usize]);
            let d0 = (t0 * neg_k0).exp();
            f0 = f0 + f32_from_bits(self.values[i as usize]) * d0;
            i = i + 1u16;
        }

        // Bounded secant: 2 seed evaluations already paid above and at the first
        // loop iteration below, then at most 4 further full XNPV passes (6 total,
        // half IRR's own 8-flow/5-walk allowance, since every pass here costs
        // roughly 5x what IRR's plain-division NPV pass costs). Convergence is
        // checked on the ITERATE (|r2-r1| < 2e-4) before paying the next pass,
        // the same walk-priced discipline excel_irr.rs already established.
        let max_steps = 4u16;
        let mut result = 0.0f32;
        let mut converged = 0u16;
        let mut step = 0u16;
        while step < max_steps {
            if 1.0f32 + r1 <= 0.0f32 { halt(0xFF06u16); }
            let base1 = 1.0f32 + r1;
            let neg_k1 = (0.0f32 - base1.ln()) / 365.0f32;
            let mut f1 = 0.0f32;
            let mut j = 0u16;
            while j < self.count {
                let t1 = int_to_f32(self.days[j as usize]);
                let d1 = (t1 * neg_k1).exp();
                f1 = f1 + f32_from_bits(self.values[j as usize]) * d1;
                j = j + 1u16;
            }
            let denom = f1 - f0;
            if denom == 0.0f32 { halt(0xFF06u16); }
            let r2 = r1 - f1 * (r1 - r0) / denom;
            let dr = r2 - r1;
            let small_dr = dr < 0.0002f32 && dr > -0.0002f32;
            if small_dr {
                result = r2;
                converged = 1u16;
                step = max_steps;
            } else {
                r0 = r1;
                f0 = f1;
                r1 = r2;
                step = step + 1u16;
            }
        }
        if converged == 0u16 { halt(0xFF06u16); }
        if result.is_nan() { halt(0xFF08u16); }
        let fin = result.is_finite();
        if !fin { halt(0xFF07u16); }
        self.xirr = result;
        1u16
    }
}
