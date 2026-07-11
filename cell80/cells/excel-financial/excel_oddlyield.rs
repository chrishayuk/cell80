//! ODDLYIELD(settlement, maturity, last_interest, rate, pr, redemption, frequency, [basis]): yield of a security whose LAST coupon period is odd (short or long), solved in closed form because settlement always falls inside that single remaining period (one terminal cash flow, no quasi-coupon-period summing) -- takes the caller-computed accrued fraction (a_over_e, last_interest-to-settlement over the basis-dispatched period length E) and remaining fraction (dsc_over_e, settlement-to-maturity over that same E) directly rather than raw dates plus a basis code, algebraically inverting ODDLPRICE's single-remaining-period pricing relationship for yield instead of re-deriving it by iteration -- distinct from ODDFYIELD (odd FIRST period, accrual runs from issue rather than from a known last_interest date) and from plain YIELD (a regular, non-odd coupon schedule, no odd-period fraction at all).
//! tags: excel, oddlyield, yield, odd-coupon, last-coupon, bond, security, day-count, fraction, basis, closed-form, redemption, price, finance
//! kernel_bank: on
//! entry: ExcelOddlyield::run
//! limits: escalates (halt 0xFF06, out_of_domain) if pr <= 0, redemption <= 0, rate < 0, frequency isn't 1/2/4, a_over_e < 0, dsc_over_e <= 0, or a_over_e + dsc_over_e > 1.0 (the odd period would span more than one normal coupon period, outside this closed-form single-period scope); escalates (halt 0xFF07, float_overflow) if the result is infinite; escalates (halt 0xFF08, float_domain) if the result is NaN

// Excel signature: ODDLYIELD(settlement, maturity, last_interest, rate, pr, redemption,
// frequency, [basis]). All arguments except basis are required; basis is optional and
// Excel defaults it to 0 (US 30/360) when omitted. frequency must be 1 (annual), 2
// (semiannual), or 4 (quarterly) -- anything else is Excel's #NUM!. rate is the security's
// annual coupon rate (0 is allowed, e.g. a zero-coupon-like odd tail; negative is Excel's
// #NUM!). pr and redemption are both quoted per $100 of face value (Excel's convention
// for every bond-pricing function in this family), never per $1, and never in the
// outflow-negative sign convention PV/FV use elsewhere -- ODDLYIELD returns a plain
// annualized yield (a rate, e.g. 0.06 for 6%), not a cash flow, so no sign convention and
// no annuity `type` (0/1) flag applies here.
//
// settlement, maturity, last_interest, and the optional basis are consumed upstream of
// this cell: the caller dispatches on basis to pick day_count_30_360_us (0),
// day_count_act_act (1), day_count_act_360 (2), day_count_act_365 (3), or
// day_count_30_360_eu (4), uses date_add_months to find the coupon period bracketing
// settlement and that period's length E, then runs the chosen day-count cell twice --
// once over (last_interest, settlement) and once over (settlement, maturity), each
// divided by E -- and feeds the two resulting fractions in here as a_over_e and
// dsc_over_e. This cell never re-derives E, a coupon date, or a raw day count itself; it
// only consumes the two already-computed fractions and inverts ODDLPRICE's
// single-remaining-period pricing formula algebraically for yield:
//
//   pr = [redemption + C*(a_over_e+dsc_over_e)] / [1 + (yield/frequency)*dsc_over_e] - C*a_over_e
//
// where C = 100*rate/frequency. Solved directly for yield (no iteration needed, since
// there is exactly one discount period left):
//
//   yield = frequency * [(redemption-pr) + C*dsc_over_e] / [dsc_over_e * (pr + C*a_over_e)]
//
// This cell scopes to the odd period being no longer than one normal coupon period
// (a_over_e+dsc_over_e <= 1); the "long odd last period spanning several quasi-coupon
// periods" case needs a compounding sum this cell does not attempt, matching
// excel_accrint.rs's precedent of scoping out its own analogous multi-period case.
struct ExcelOddlyield {
    rate: f32,
    pr: f32,
    redemption: f32,
    frequency: u16,
    a_over_e: f32,
    dsc_over_e: f32,
    oddlyield: f32,
}
impl ExcelOddlyield {
    fn run(&mut self) -> u16 {
        if self.pr <= 0.0f32 { halt(0xFF06u16); }
        if self.redemption <= 0.0f32 { halt(0xFF06u16); }
        if self.rate < 0.0f32 { halt(0xFF06u16); }
        let freq_ok = self.frequency == 1u16 || self.frequency == 2u16 || self.frequency == 4u16;
        if !freq_ok { halt(0xFF06u16); }
        if self.a_over_e < 0.0f32 { halt(0xFF06u16); }
        if self.dsc_over_e <= 0.0f32 { halt(0xFF06u16); }

        let dci_over_e = self.a_over_e + self.dsc_over_e;
        if dci_over_e > 1.0f32 { halt(0xFF06u16); }

        let freq_f = int_to_f32(self.frequency);
        let c = (100.0f32 * self.rate) / freq_f;

        let numerator_term = (self.redemption - self.pr) + c * self.dsc_over_e;
        let denom_price = self.pr + c * self.a_over_e;
        let denom_term = self.dsc_over_e * denom_price;

        let result = freq_f * numerator_term / denom_term;

        if result.is_nan() { halt(0xFF08u16); }
        let fin = result.is_finite();
        if !fin { halt(0xFF07u16); }

        self.oddlyield = result;
        1u16
    }
}
