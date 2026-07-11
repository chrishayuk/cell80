//! Amount received at maturity for a fully invested, non-interest-bearing (discount-basis) security -- RECEIVED = investment / (1 - discount*DIM/B), the day-count fraction DIM/B taken as an already-computed dsm_over_b input from whichever basis-dispatched day_count_* cell Excel's basis argument selects (never re-derived here) -- structurally DISC's own equation (discount = (redemption-pr)/redemption * (B/DSM)) solved for the OTHER unknown: DISC recovers a discount rate from a known purchase price and redemption value, RECEIVED recovers the redemption/maturity payout from a known purchase price (investment) and discount rate, while PRICEDISC recovers price by a plain linear subtraction (redemption - discount*redemption*DSM/B) rather than this reciprocal division.
//! tags: excel, received, amount-received, maturity, redemption, discount-rate, security, treasury-bill, investment, day-count, fraction, basis, finance
//! kernel_bank: on
//! entry: ExcelReceived::run
//! limits: escalates (halt 0xFF06, out_of_domain) if investment <= 0, discount <= 0, dsm_over_b <= 0, or the denominator (1 - discount*dsm_over_b) <= 0; escalates (halt 0xFF07, float_overflow) if the result is infinite; escalates (halt 0xFF08, float_domain) if the result is NaN

// Excel signature: RECEIVED(settlement, maturity, investment, discount, [basis]).
// settlement, maturity, investment, and discount are all required; basis is optional
// and defaults to 0 (US 30/360) when omitted. settlement/maturity and the basis
// dispatch are consumed upstream of this cell: the caller picks the day_count_* cell
// matching basis (day_count_30_360_us for 0, day_count_act_act for 1,
// day_count_act_360 for 2, day_count_act_365 for 3, day_count_30_360_eu for 4), runs
// it over (settlement, maturity) to get the day-count fraction (Excel's own docs call
// this DIM/B for RECEIVED -- the identical settlement-to-maturity quantity DISC calls
// DSM/B), and feeds it in here as dsm_over_b, kept as the same field name DISC uses
// since it is the same quantity computed the same way, never re-derived by this cell.
// No outflow-negative sign convention applies (RECEIVED returns a redemption/payout
// amount, not a signed cash flow), and there is no type (0/1 annuity-due) argument for
// this function.
struct ExcelReceived {
    investment: f32,
    discount: f32,
    dsm_over_b: f32,
    received: f32,
}
impl ExcelReceived {
    fn run(&mut self) -> u16 {
        if self.investment <= 0.0f32 { halt(0xFF06u16); }
        if self.discount <= 0.0f32 { halt(0xFF06u16); }
        if self.dsm_over_b <= 0.0f32 { halt(0xFF06u16); }

        let denom = 1.0f32 - self.discount * self.dsm_over_b;
        if denom <= 0.0f32 { halt(0xFF06u16); }

        let received = self.investment / denom;

        if received.is_nan() { halt(0xFF08u16); }
        let fin = received.is_finite();
        if !fin { halt(0xFF07u16); }

        self.received = received;
        1u16
    }
}
