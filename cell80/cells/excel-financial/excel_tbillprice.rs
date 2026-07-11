//! Price per $100 face value of a Treasury bill from its discount rate (Excel TBILLPRICE(settlement, maturity, discount) = 100*(1 - discount*DSM/360)), taking DSM/360 as an already-computed input from day_count_act_360 (settlement/maturity and the fixed actual/360 divide are consumed upstream and never re-derived here, and unlike DISC/PRICEMAT/AMORLINC there is no basis argument to dispatch on at all -- TBILLPRICE is always actual/360) -- distinct from TBILLEQ (this same discount rate turned into a bond-equivalent yield via a quadratic, not a dollar price) and TBILLYIELD (the inverse direction, a known price turned back into a discount yield via a single division).
//! tags: excel, tbillprice, treasury-bill, tbill, treasury, discount-rate, price, actual-360, day-count, redemption, finance
//! kernel_bank: on
//! entry: ExcelTbillprice::run
//! limits: escalates (halt 0xFF06, out_of_domain) if discount <= 0, if dsm_over_360 <= 0, or if dsm_over_360 exceeds 366/360 (mirrors Excel's "maturity more than one year after settlement" #NUM! case, the same 366-day cap excel_tbilleq.rs uses); escalates (halt 0xFF07, float_overflow) if the result is infinite; escalates (halt 0xFF08, float_domain) if the result is NaN

// Excel signature: TBILLPRICE(settlement, maturity, discount). All three arguments are
// required -- there is no optional/omittable parameter and no `basis` argument at all
// (TBILLPRICE is always actual/360, unlike DISC/PRICEMAT/AMORLINC which dispatch on a
// basis code). discount is entered as a plain positive decimal rate (e.g. 0.0914 for
// 9.14%), matching Excel's own #NUM! on discount <= 0; there is no outflow-negative
// sign convention (TBILLPRICE returns a quoted price per $100 face value, not a cash
// flow) and no type (0/1 annuity-due) argument for this function at all.
//
// settlement and maturity are consumed upstream of this cell: the caller runs
// day_count_act_360 over (settlement, maturity) to get the actual/360 year fraction
// DSM/360, and feeds that single fraction in here as dsm_over_360 -- this cell never
// re-derives a day count itself, it only multiplies discount by the fraction once and
// subtracts from 1.
struct ExcelTbillprice {
    discount: f32,
    dsm_over_360: f32,
    price: f32,
}
impl ExcelTbillprice {
    fn run(&mut self) -> u16 {
        if self.discount <= 0.0f32 { halt(0xFF06u16); }
        if self.dsm_over_360 <= 0.0f32 { halt(0xFF06u16); }
        // 366/360: the same "maturity no more than one year after settlement" cap
        // excel_tbilleq.rs enforces on raw DSM (> 366 days), expressed here as a
        // fraction since DSM itself never reaches this cell.
        let cap = 366.0f32 / 360.0f32;
        if self.dsm_over_360 > cap { halt(0xFF06u16); }

        let term = self.discount * self.dsm_over_360;
        let inner = 1.0f32 - term;
        let price = 100.0f32 * inner;

        if price.is_nan() { halt(0xFF08u16); }
        let fin = price.is_finite();
        if !fin { halt(0xFF07u16); }

        self.price = price;
        1u16
    }
}
