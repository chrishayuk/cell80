//! Excel EFFECT: effective annual rate from a nominal annual rate compounded npery times per year, EFFECT = (1+nominal/npery)^npery - 1 -- both nominal_rate_bps and npery are required with no omittable default (nominal_rate_bps must be > 0 and npery >= 1, else out-of-domain), no outflow-sign or annuity-due type applies (this is a pure rate conversion, not a cash-flow schedule), and the bps-scaled per-period rate is floored by integer division BEFORE compounding npery times (compound_increase_by_bps's own technique) then 10000 is subtracted once at the end for the "-1" -- this divide-then-compound-then-subtract ordering is the well-known fragile part: flooring the per-period rate early compounds its own rounding error npery times rather than rounding once at the end.
//! tags: excel, finance, effect, nominal, effective, annual, rate, compounding, periods, npery, bps, basis-points, compound, checked, wide, u32
//! entry: ExcelEffect::run
//! limits: escalates (halt 0xFF06, out_of_domain) if nominal_rate_bps == 0 or npery < 1; escalates (halt 0xFF05, needs_wider_math) the moment any compounding step's multiply or add overflows u32
struct ExcelEffect { nominal_rate_bps: u32, npery: u16, effective_rate_bps: u32 }
impl ExcelEffect {
    fn run(&mut self) -> u16 {
        if self.nominal_rate_bps == 0u32 { halt(0xFF06u16); }
        if self.npery < 1u16 { halt(0xFF06u16); }
        let per_period_bps = self.nominal_rate_bps / (self.npery as u32);
        let mut v = 10000u32;
        let mut i = 0u16;
        while i < self.npery {
            let product = mul_checked_u32(v, per_period_bps);
            let delta = product / 10000u32;
            let r = add_checked_u32(v, delta);
            v = r;
            i = i + 1u16;
        }
        self.effective_rate_bps = v - 10000u32;
        1u16
    }
}
