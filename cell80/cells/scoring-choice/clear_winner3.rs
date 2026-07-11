//! Returns 1 if there is a decisive winner among three raw candidate scores (top beats the runner-up by at least margin), else 0 — computed directly from a/b/c instead of requiring the caller to pre-identify top and second like is_clear_winner/clear_winner_u32 do, by exploiting that for three values the second-highest is exactly the median.
//! tags: winner, clear, margin, decisive, tie, score, ranking, median, three, candidate, plan
//! entry: ClearWinner3::run
struct ClearWinner3 { score_a: u16, score_b: u16, score_c: u16, margin: u16 }
impl ClearWinner3 {
    fn run(&mut self) -> u16 {
        let a = self.score_a;
        let b = self.score_b;
        let c = self.score_c;
        let top = imax(imax(a, b), c);
        let lo = imin(imin(a, b), c);
        let second = a.wrapping_add(b).wrapping_add(c).wrapping_sub(lo).wrapping_sub(top);
        ((top - second) >= self.margin) as u16
    }
}
