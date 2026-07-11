//! Present value of an ordinary annuity/annuity-due plus a lump-sum future value, composed from compound_original_before_increase's reverse-compounding loop (a discount factor built by dividing by (1+rate) each period rather than a closed-form power) and geometric_series_sum's running-sum idiom (the annuity term accumulated alongside that same loop, avoiding any division by rate) -- covers Excel's rate/nper/pmt/[fv=0]/[type=0] signature (type=1 multiplies the annuity term by (1+rate) for beginning-of-period payments instead of end-of-period), the outflow-negative sign convention (pv returns negated relative to pmt/fv, per Excel's pv*(1+rate)^nper + pmt*(1+rate*type)*annuity + fv = 0 identity), and restricts nper to whole periods since a fractional exponent would need an unavailable transcendental pow -- distinct from FV (the same identity solved for the future term via forward compounding) and PMT (the same identity solved for the payment).
//! tags: finance, excel, tvm, time-value-of-money, pv, present-value, annuity, annuity-due, discount, compounding, reverse-compounding, lump-sum, f32, float, softfloat
//! kernel_bank: on
//! entry: ExcelPv::run
//! limits: escalates (halt 0xFF06, out_of_domain) if 1+rate == 0 (undefined discount factor, e.g. rate = -100%); escalates (halt 0xFF08, float_domain) on a NaN result or (halt 0xFF07, float_overflow) on a non-finite result (e.g. an extreme negative rate blowing up the discount factor); nper is a whole number of periods, not Excel's fractional-nper generality
struct ExcelPv { rate: f32, nper: u16, pmt: f32, fv: f32, pmt_type: u16, pv: f32 }
impl ExcelPv {
    fn run(&mut self) -> u16 {
        let onerate = 1.0f32 + self.rate;
        if onerate == 0.0f32 { halt(0xFF06u16); }
        let mut disc = 1.0f32;
        let mut annuity = 0.0f32;
        let mut i = 0u16;
        while i < self.nper {
            disc = disc / onerate;
            annuity = annuity + disc;
            i = i + 1u16;
        }
        let type_f = if self.pmt_type == 1u16 { 1.0f32 } else { 0.0f32 };
        let due_factor = 1.0f32 + self.rate * type_f;
        let raw = -(self.pmt * due_factor * annuity + self.fv * disc);
        if raw.is_nan() { halt(0xFF08u16); }
        let fin = raw.is_finite();
        if !fin { halt(0xFF07u16); }
        self.pv = raw;
        1u16
    }
}
