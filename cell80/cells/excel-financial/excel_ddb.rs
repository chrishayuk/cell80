//! Double-declining-balance depreciation for one period (Excel's DDB): rate = factor/life (factor is omittable, Excel defaults it to 2 for true double-declining-balance; pass 2 explicitly here), then loop from period 1 up to the target period tracking a declining book_value, taking depreciation = min(book_value*rate, book_value-salvage) each step so it floors to zero once book_value reaches salvage -- distinct from a flat per-period split (straight-line, SLN) or a years-digits-weighted split (SYD), DDB is the only method whose rate is re-applied against the *declining* book value rather than the original cost.
//! tags: excel, finance, depreciation, ddb, double-declining-balance, declining-balance, book-value, factor, salvage, asset, cost, checked, wide, u32, money, cents
//! entry: ExcelDdb::run
//! limits: escalates (halt 0xFF06, out_of_domain) if life == 0, period == 0, factor == 0, or salvage_cents > cost_cents; escalates (halt 0xFF05, needs_wider_math) if book_value*factor overflows u32 during the loop
struct ExcelDdb {
    cost_cents: u32,
    salvage_cents: u32,
    life: u16,
    period: u16,
    factor: u16,
    depreciation_cents: u32,
}
impl ExcelDdb {
    fn run(&mut self) -> u16 {
        if self.life == 0u16 { halt(0xFF06u16); }
        if self.period == 0u16 { halt(0xFF06u16); }
        if self.factor == 0u16 { halt(0xFF06u16); }
        if self.salvage_cents > self.cost_cents { halt(0xFF06u16); }

        let mut book_value = self.cost_cents;
        let mut depr = 0u32;
        let mut p = 0u16;
        let life32 = self.life as u32;
        let factor32 = self.factor as u32;
        while p < self.period {
            let scaled = mul_checked_u32(book_value, factor32);
            let by_rate = scaled / life32;
            let remaining = book_value - self.salvage_cents;
            let d = if by_rate < remaining { by_rate } else { remaining };
            book_value = book_value - d;
            depr = d;
            p = p + 1u16;
        }
        self.depreciation_cents = depr;
        1u16
    }
}
