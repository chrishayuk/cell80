//! Excel IRR(values, [guess]): internal rate of return of a cash-flow stream -- the rate where NPV crosses zero, with Excel's IRR convention that values[0] sits at time zero (undiscounted) and each later flow discounts one more period. No closed form exists, so this iterates: a bounded SECANT search (two seeded NPV evaluations at guess and guess+0.1, then at most 5 secant walks at up to 6 flows / 4 walks at 7-8, convergence checked on the iterate before paying the next walk) -- secant over Newton because each derivative walk would cost a second full array pass per step, and the cycle budget is the binding wall here, the same pricing discipline excel_oddfyield's fixed 4-round bisection documents. Cash flows arrive in a u32[8] state field carrying f32 bit patterns (host writes f32::to_bits; the cell reinterprets with f32_from_bits) -- the envelope is 8, HALF the pack's usual 16, because each secant step walks the whole array through real softfloat ops. guess == 0.0 is treated as omitted and defaults to Excel's own 0.1. Distinct from excel_rate (constant-payment annuity, closed-form-per-round bisection) and from XIRR (irregular dates -- priced out of the default cycle budget entirely, see docs/excel-financial-map.md).
//! tags: excel, irr, internal-rate-of-return, cash-flow, rate, yield, secant, root-finding, iterative, array, finance, f32
//! kernel_bank: on
//! entry: ExcelIrr::run
//! accuracy: rate accurate to ~1e-5 typical on well-conditioned streams (secant accepts an iterate when the step falls below 2e-4 -- the accepted value sits ~step^2 closer; Excel's own 1e-7 target would cost walks the cycle budget cannot buy); ill-conditioned streams escalate rather than answer loosely
//! limits: fixed 8-slot cash-flow envelope (the cycle-budget wall: each secant step re-walks the array through real softfloat ops), not caller-configurable; escalates (halt 0xFF06, out_of_domain) if count < 2 or count > 8, if an iterate leaves the rate domain (1+r <= 0), if the secant degenerates (flat NPV between iterates), or if the walk allowance (5 at count <= 6, 4 above) doesn't converge (Excel's #NUM! after its own 20 tries); escalates (halt 0xFF08, float_domain) on a NaN result, (halt 0xFF07, float_overflow) on a non-finite one
struct ExcelIrr {
    values: [u32; 8],
    count: u16,
    guess: f32,
    irr: f32,
}
impl ExcelIrr {
    fn run(&mut self) -> u16 {
        if self.count < 2u16 { halt(0xFF06u16); }
        if self.count > 8u16 { halt(0xFF06u16); }
        let mut r0 = self.guess;
        if r0 == 0.0f32 { r0 = 0.1f32; }
        let mut r1 = r0 + 0.1f32;
        if 1.0f32 + r0 <= 0.0f32 { halt(0xFF06u16); }

        // npv(r) inlined per evaluation: values[0] undiscounted, each later flow
        // one more 1/(1+r) factor (Excel's IRR timing, unlike NPV's shift-by-one).
        let mut f0 = 0.0f32;
        let inv0 = 1.0f32 / (1.0f32 + r0);
        let mut df = 1.0f32;
        let mut i = 0u16;
        while i < self.count {
            f0 = f0 + f32_from_bits(self.values[i as usize]) * df;
            df = df * inv0;
            i = i + 1u16;
        }

        // Bounded secant: at most 4 steps after the two seeds, and convergence is
        // checked on the ITERATE (|r2 - r1| < 2e-4; the returned r2 sits ~dr^2 closer,
        // once the next secant correction would land ~dr² away) BEFORE paying the
        // next array walk — each walk is the budget item here, so a converged
        // solve never buys an evaluation it won't use.
        // Step allowance is count-dependent: 5 walks fit the budget up to 6
        // flows, only 4 at the full 8-slot envelope (each walk scales with count).
        let max_steps = if self.count <= 6u16 { 5u16 } else { 4u16 };
        let mut result = 0.0f32;
        let mut converged = 0u16;
        let mut step = 0u16;
        while step < max_steps {
            if 1.0f32 + r1 <= 0.0f32 { halt(0xFF06u16); }
            let mut f1 = 0.0f32;
            let inv1 = 1.0f32 / (1.0f32 + r1);
            let mut df1 = 1.0f32;
            let mut j = 0u16;
            while j < self.count {
                f1 = f1 + f32_from_bits(self.values[j as usize]) * df1;
                df1 = df1 * inv1;
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
        self.irr = result;
        1u16
    }
}
