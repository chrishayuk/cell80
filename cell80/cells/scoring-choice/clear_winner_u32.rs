//! Returns 1 if the top score beats the second-best by at least margin at wide u32 width, else 0 — including when top < second (a malformed call, treated as no clear winner) — the wide sibling of is_clear_winner (which works over u16 and can't compare scores beyond 65535, e.g. money totals in cents).
//! tags: winner, clear, margin, decisive, tie, ambiguous, score, ranking, plan, wide, u32, large
//! entry: ClearWinnerWide::run
struct ClearWinnerWide { top: u32, second: u32, margin: u32 }
impl ClearWinnerWide {
    fn run(&mut self) -> u16 {
        if self.top < self.second {
            0u16
        } else {
            ((self.top - self.second) >= self.margin) as u16
        }
    }
}
