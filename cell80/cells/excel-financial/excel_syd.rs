//! Sum-of-years-digits depreciation for one period: SYD(cost,salvage,life,per) = (cost-salvage)*(life-per+1)/(life*(life+1)/2); all four arguments are required (Excel defines no optional params for SYD), and cost/salvage are positive asset magnitudes with no outflow-negative sign convention here (unlike PMT/FV/PV's signed cash flows) -- distinct from SLN's constant per-period charge and DB/DDB's declining-balance-rate shape, this weights the depreciable base by remaining useful life over the summed year count, front-loading larger charges into early periods.
//! tags: excel, depreciation, syd, sum-of-years-digits, sum-of-the-years-digits, accelerated, asset, cost, salvage, book-value, remaining-life, checked, wide, u32
//! entry: ExcelSyd::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if salvage > cost or if the depreciable-base * remaining-life multiply overflows u32; escalates (halt 0xFF06, out_of_domain) if life == 0, per == 0, or per > life; the final divide floors (truncates) toward zero rather than rounding to the nearest cent, the same convention frac_of_whole_floor already established for this pack -- it never escalates on an inexact split
struct ExcelSyd {
    cost: u32,
    salvage: u32,
    life: u16,
    per: u16,
    result: u32,
}
impl ExcelSyd {
    fn run(&mut self) -> u16 {
        if self.life == 0u16 { halt(0xFF06u16); }
        if self.per == 0u16 { halt(0xFF06u16); }
        if self.per > self.life { halt(0xFF06u16); }
        let depreciable = sub_checked_u32(self.cost, self.salvage);
        let remaining = self.life - self.per + 1u16;
        let life32 = self.life as u32;
        let remaining32 = remaining as u32;
        let sum_years = mul_checked_u32(life32, life32 + 1u32);
        let denom = sum_years / 2u32;
        let numerator = mul_checked_u32(depreciable, remaining32);
        let result = numerator / denom;
        self.result = result;
        1u16
    }
}
