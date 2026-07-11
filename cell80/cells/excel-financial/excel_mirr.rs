//! Excel MIRR(values, finance_rate, reinvest_rate): modified internal rate of return -- negative flows discounted to present value at the finance rate, positive flows compounded forward to the horizon at the reinvestment rate, then mirr = (FV_pos / -PV_neg)^(1/(n-1)) - 1, the closed form that fixes IRR's reinvest-at-IRR assumption. The fractional root runs through the owned F2 fpow (this cell is why MIRR could never ship before the transcendentals landed). Cash flows arrive in a u32[12] state field carrying f32 bit patterns (host writes f32::to_bits; the cell reinterprets with f32_from_bits) -- envelope 12 because the cell walks the array twice AND pays one fpow. Distinct from excel_irr (iterative root-find, single rate, reinvest-at-IRR baked in) and NPV (no rate solving at all).
//! tags: excel, mirr, modified-irr, internal-rate-of-return, reinvestment, finance-rate, cash-flow, array, transcendental, pow, finance, f32
//! kernel_bank: on
//! entry: ExcelMirr::run
//! accuracy: <= ~45 ulp worst case (one fpow at <= 41 ulp over its declared domain + two discounting walks of correctly-rounded ops; rustz80's F2 harness pins the kernel)
//! limits: fixed 12-slot cash-flow envelope, not caller-configurable (two array walks + an fpow against the cycle budget); escalates (halt 0xFF06, out_of_domain) if count < 2 or count > 12, if finance_rate == -1 or reinvest_rate == -1, or if the stream lacks at least one negative AND one positive flow (Excel's #DIV/0!); escalates (halt 0xFF08, float_domain) on a NaN result, (halt 0xFF07, float_overflow) on a non-finite one
struct ExcelMirr {
    values: [u32; 12],
    count: u16,
    finance_rate: f32,
    reinvest_rate: f32,
    mirr: f32,
}
impl ExcelMirr {
    fn run(&mut self) -> u16 {
        if self.count < 2u16 { halt(0xFF06u16); }
        if self.count > 12u16 { halt(0xFF06u16); }
        let fbase = 1.0f32 + self.finance_rate;
        let rbase = 1.0f32 + self.reinvest_rate;
        if fbase == 0.0f32 { halt(0xFF06u16); }
        if rbase == 0.0f32 { halt(0xFF06u16); }

        // PV of the negatives at the finance rate (values[0] at time zero).
        let finv = 1.0f32 / fbase;
        let mut df = 1.0f32;
        let mut pv_neg = 0.0f32;
        let mut any_neg = 0u16;
        let mut i = 0u16;
        while i < self.count {
            let cf = f32_from_bits(self.values[i as usize]);
            if cf < 0.0f32 {
                pv_neg = pv_neg + cf * df;
                any_neg = 1u16;
            }
            df = df * finv;
            i = i + 1u16;
        }

        // FV of the positives at the reinvest rate, compounded to period n-1:
        // walk backward so the growth factor is a running multiply.
        let mut fv_pos = 0.0f32;
        let mut any_pos = 0u16;
        let mut g = 1.0f32;
        let mut k = self.count;
        while k > 0u16 {
            let idx = k - 1u16;
            let cf = f32_from_bits(self.values[idx as usize]);
            if cf > 0.0f32 {
                fv_pos = fv_pos + cf * g;
                any_pos = 1u16;
            }
            g = g * rbase;
            k = k - 1u16;
        }

        if any_neg == 0u16 { halt(0xFF06u16); }
        if any_pos == 0u16 { halt(0xFF06u16); }

        let ratio = fv_pos / (0.0f32 - pv_neg);
        let n1 = int_to_f32(self.count - 1u16);
        let root = ratio.powf(1.0f32 / n1);
        let m = root - 1.0f32;

        if m.is_nan() { halt(0xFF08u16); }
        let fin = m.is_finite();
        if !fin { halt(0xFF07u16); }
        self.mirr = m;
        1u16
    }
}
