//! Treasury bill discount yield from settlement/maturity dates and price per 100 of face value: yield = ((100-pr)/pr)*(360/DSM), DSM the actual days settlement to maturity -- all three Excel arguments (settlement, maturity, pr) are required with no optional/defaulted params, and pr is always a positive quoted price (never a signed cash-flow outflow); distinct from TBILLPRICE (the reverse direction: price derived from a known yield) and TBILLEQ (which further converts this same discount yield onto a bond-equivalent basis).
//! tags: finance, excel, treasury, tbill, t-bill, bill, yield, discount-yield, price, settlement, maturity, dsm, actual-360, bps, checked, wide, u32
//! entry: ExcelTbillYield::run
//! limits: escalates (halt 0xFF06, out_of_domain) if pr <= 0 (pr_cents == 0), if pr >= 100 (pr_cents >= 10000 -- this pack has no signed-yield field, so a non-positive yield is out of domain rather than encoded negative), if settlement >= maturity, or if DSM (actual days settlement to maturity) exceeds 365 (matches Excel's own "#NUM! if maturity is more than one year after settlement" rule); escalates (halt 0xFF05, needs_wider_math) if the discount*3,600,000 numerator or the pr_cents*DSM denominator multiply overflows u32
fn serial_day(y: u16, m: u16, d: u16) -> u32 {
    let y32 = y as u32;
    let m32 = m as u32;
    let d32 = d as u32;
    let a = (14u32 - m32) / 12u32;
    let yy = y32 + 4800u32 - a;
    let mm = m32 + 12u32 * a - 3u32;
    d32 + (153u32 * mm + 2u32) / 5u32 + 365u32 * yy + yy / 4u32 - yy / 100u32 + yy / 400u32 - 32045u32
}

struct ExcelTbillYield {
    y1: u16,
    m1: u16,
    d1: u16,
    y2: u16,
    m2: u16,
    d2: u16,
    pr_cents: u32,
    yield_bps: u32,
}
impl ExcelTbillYield {
    fn run(&mut self) -> u16 {
        if self.pr_cents == 0u32 { halt(0xFF06u16); }
        if self.pr_cents >= 10000u32 { halt(0xFF06u16); }
        let s1 = serial_day(self.y1, self.m1, self.d1);
        let s2 = serial_day(self.y2, self.m2, self.d2);
        if s2 <= s1 { halt(0xFF06u16); }
        let dsm = s2 - s1;
        if dsm > 365u32 { halt(0xFF06u16); }
        let discount = 10000u32 - self.pr_cents;
        let numerator = mul_checked_u32(discount, 3600000u32);
        let denominator = mul_checked_u32(self.pr_cents, dsm);
        self.yield_bps = numerator / denominator;
        1u16
    }
}
