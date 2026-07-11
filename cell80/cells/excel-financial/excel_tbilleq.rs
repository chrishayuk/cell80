//! Bond-equivalent (coupon-equivalent) yield for a Treasury bill from its discount rate (Excel TBILLEQ(settlement, maturity, discount)): DSM=raw actual-day count between settlement and maturity (no basis parameter, Gregorian serial-day subtraction), DSM<=182 is one division (365*discount/(360-discount*DSM)), DSM>182 solves the genuine quadratic in x=DSM/365 (one fsqrt) that longer bills require -- distinct from TBILLYIELD (price->yield, a single division, never branches) and TBILLPRICE (discount->dollar price, pure actual/360 division), TBILLEQ is the only one of the three treasury functions with a quadratic branch at all.
//! tags: excel, tbilleq, treasury-bill, tbill, treasury, bond-equivalent-yield, coupon-equivalent-yield, discount-rate, quadratic, sqrt, dsm, days-between, finance
//! kernel_bank: on
//! entry: ExcelTbilleq::run
//! limits: escalates (halt 0xFF06, out_of_domain) if maturity isn't strictly after settlement, if DSM (actual days between) exceeds 366 (Excel's own "maturity more than one year after settlement" #NUM! case), or if discount <= 0 (Excel's own third #NUM! condition); escalates (halt 0xFF07, float_overflow) if the result is infinite, (halt 0xFF08, float_domain) if it's NaN (an adversarially large discount can flip the quadratic's discriminant negative)

// Excel signature: TBILLEQ(settlement, maturity, discount). All three arguments are
// required -- there is no optional/omittable parameter and no outflow-negative sign
// convention (discount is always entered as a positive decimal rate, e.g. 0.0914 for
// 9.14%). No `basis` parameter exists for this function at all: DSM is always the raw
// actual-day count, independent of any day-count convention.
//
// A calendar-month-range guard (1<=month<=12, the excel_coupncd.rs convention) was
// deliberately dropped here, not forgotten: this cell's straight-line body already
// sits right at the compiled-code-window ceiling once the quadratic branch's fsqrt
// (plus fdiv/fmul/fsub and the comparison kernels) are linked in -- a 4th sequential
// domain-guard `if` on top of the 3 Excel-documented ones (below) reproducibly blew
// the 0xB000 code+scratch ceiling during authoring, while the NaN/Infinity safety net
// (which every f32 cell in this dialect carries) did not, and is the more important of
// the two to keep since discount is otherwise unbounded above. Month validity is the
// caller's responsibility here.
fn serial_day(y: u16, m: u16, d: u16) -> u32 {
    let y32 = y as u32;
    let m32 = m as u32;
    let d32 = d as u32;
    let a = (14u32 - m32) / 12u32;
    let yy = y32 + 4800u32 - a;
    let mm = m32 + 12u32 * a - 3u32;
    d32 + (153u32 * mm + 2u32) / 5u32 + 365u32 * yy + yy / 4u32 - yy / 100u32 + yy / 400u32 - 32045u32
}

struct ExcelTbilleq {
    sy: u16,
    sm: u16,
    sd: u16,
    my: u16,
    mm: u16,
    md: u16,
    discount: f32,
    bey: f32,
}
impl ExcelTbilleq {
    fn run(&mut self) -> u16 {
        let s1 = serial_day(self.sy, self.sm, self.sd);
        let s2 = serial_day(self.my, self.mm, self.md);
        if s2 <= s1 { halt(0xFF06u16); }
        let dsm32 = s2 - s1;
        if dsm32 > 366u32 { halt(0xFF06u16); }
        if self.discount <= 0.0f32 { halt(0xFF06u16); }
        let dsm_f = int_to_f32(dsm32);

        let mut bey = 0.0f32;
        if dsm32 <= 182u32 {
            // DSM<=182: yield = 365*discount / (360 - discount*DSM)
            bey = (365.0f32 * self.discount) / (360.0f32 - self.discount * dsm_f);
        } else {
            // DSM>182: the published Treasury quadratic in x=DSM/365 (year fixed at
            // 365, matching Excel's own known behaviour of never adjusting to 366
            // even when a leap day falls inside DSM -- this is not our bug, it's
            // Excel's, and TBILLEQ has no basis/day-count-convention input to fix it
            // with). P is the discount-implied price per 100 face value:
            //   b = DSM/365
            //   P = 100*(1 - discount*DSM/360)
            //   c = (P - 100)/P
            //   a = (DSM/730) - 0.25
            //   yield = (-b + sqrt(b*b - 4*a*c)) / (2*a)
            let b = dsm_f / 365.0f32;
            let price = 100.0f32 * (1.0f32 - self.discount * dsm_f / 360.0f32);
            let c = (price - 100.0f32) / price;
            let a = (dsm_f / 730.0f32) - 0.25f32;
            bey = (0.0f32 - b + (b * b - 4.0f32 * a * c).sqrt()) / (2.0f32 * a);
        }

        if bey.is_nan() { halt(0xFF08u16); }
        if !bey.is_finite() { halt(0xFF07u16); }

        self.bey = bey;
        1u16
    }
}
