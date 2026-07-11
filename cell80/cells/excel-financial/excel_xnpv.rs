//! Excel XNPV(rate, values, dates): net present value of cash flows at IRREGULAR dates -- each flow discounts by (1+rate)^(days_i/365) where days_i is the flow's day offset from the first date, computed as exp(-(days_i/365) * ln(1+rate)) through the owned F2 fexp/fln (a fractional-year exponent per flow is exactly what plain NPV's whole-period walk cannot express, and why XNPV could never ship before the transcendentals landed). Flows arrive in a u32[4] state field carrying f32 bit patterns (host writes f32::to_bits; the cell reinterprets with f32_from_bits) with day offsets in a parallel u32[4] of plain integers (raw dates are consumed upstream: the caller runs excel_days/days_between per flow against the first date, the same feed-in convention the day-count family uses everywhere in this pack) -- the envelope is 4, the smallest in the pack, because every flow pays a full fexp (~300K T-states each against the 2M budget). Distinct from XIRR (would iterate THIS entire evaluation per secant step -- priced out of the default cycle budget entirely, see docs/excel-financial-map.md).
//! tags: excel, xnpv, net-present-value, irregular, dates, day-count, cash-flow, discount, transcendental, exp, ln, array, finance, f32
//! kernel_bank: on
//! entry: ExcelXnpv::run
//! accuracy: <= ~8 ulp per discounted term (one fln at <= 1 ulp amortized across the stream + one fexp at <= 1 ulp per flow, error scaled by |t*ln(1+rate)| which stays small at realistic rates/horizons; rustz80's F2 harness pins the kernels)
//! limits: fixed 4-slot envelope, the pack's smallest, not caller-configurable (each flow costs a full fexp -- the cycle-budget wall, measured); escalates (halt 0xFF06, out_of_domain) if count is 0 or exceeds 4, or if rate <= -1 (ln domain, Excel's #NUM!); escalates (halt 0xFF08, float_domain) on a NaN result, (halt 0xFF07, float_overflow) on a non-finite one
struct ExcelXnpv {
    rate: f32,
    values: [u32; 4],
    days: [u32; 4],
    count: u16,
    xnpv: f32,
}
impl ExcelXnpv {
    fn run(&mut self) -> u16 {
        if self.count == 0u16 { halt(0xFF06u16); }
        if self.count > 4u16 { halt(0xFF06u16); }
        let base = 1.0f32 + self.rate;
        if base <= 0.0f32 { halt(0xFF06u16); }
        let neg_k = (0.0f32 - base.ln()) / 365.0f32;
        let mut acc = 0.0f32;
        let mut i = 0u16;
        while i < self.count {
            let t = int_to_f32(self.days[i as usize]);
            let d = (t * neg_k).exp();
            acc = acc + f32_from_bits(self.values[i as usize]) * d;
            i = i + 1u16;
        }
        if acc.is_nan() { halt(0xFF08u16); }
        let fin = acc.is_finite();
        if !fin { halt(0xFF07u16); }
        self.xnpv = acc;
        1u16
    }
}
