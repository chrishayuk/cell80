//! Excel NPV(rate, value1..valueN): net present value of a cash-flow stream at a fixed per-period discount rate, sum of values[i]/(1+rate)^(i+1) with Excel's own convention that the FIRST value is discounted one full period (payments land at period ends -- an at-time-zero outlay is added outside NPV, exactly as Excel's docs say) -- the pack's first array-input cell (`.cell` v11): the cash flows arrive in a u32[16] state field carrying f32 bit patterns (the dialect has no [f32; N] fields; the host writes f32::to_bits per element, the cell reinterprets each with f32_from_bits), with `count` naming how many of the 16 envelope slots are live. Distinct from XNPV (irregular dates, per-flow fractional-year exponents through fexp) and from PV (a single closed-form annuity, no per-flow array at all).
//! tags: excel, npv, net-present-value, cash-flow, discount, rate, present-value, dcf, array, finance, f32
//! kernel_bank: on
//! entry: ExcelNpv::run
//! limits: fixed 16-slot cash-flow envelope, not caller-configurable (the array-state envelope wall); escalates (halt 0xFF06, out_of_domain) if count is 0 or exceeds 16, or if rate == -1 (Excel's #DIV/0!); escalates (halt 0xFF08, float_domain) on a NaN result, (halt 0xFF07, float_overflow) on a non-finite one
struct ExcelNpv {
    rate: f32,
    values: [u32; 16],
    count: u16,
    npv: f32,
}
impl ExcelNpv {
    fn run(&mut self) -> u16 {
        if self.count == 0u16 { halt(0xFF06u16); }
        if self.count > 16u16 { halt(0xFF06u16); }
        let base = 1.0f32 + self.rate;
        if base == 0.0f32 { halt(0xFF06u16); }
        let inv = 1.0f32 / base;
        let mut df = 1.0f32;
        let mut acc = 0.0f32;
        let mut i = 0u16;
        while i < self.count {
            df = df * inv;
            let cf = f32_from_bits(self.values[i as usize]);
            acc = acc + cf * df;
            i = i + 1u16;
        }
        if acc.is_nan() { halt(0xFF08u16); }
        let fin = acc.is_finite();
        if !fin { halt(0xFF07u16); }
        self.npv = acc;
        1u16
    }
}
