//! Returns 1 if there is a decisive loser among three raw candidate scores (the bottom is beaten by the second-lowest by at least margin), else 0 — the low-end mirror of clear_winner3, exploiting that for three values the second-lowest is exactly the median.
//! tags: loser, clear, margin, decisive, tie, score, ranking, median, three, candidate, plan
//! entry: ClearLoser3::run
struct ClearLoser3 { score_a: u16, score_b: u16, score_c: u16, margin: u16 }
impl ClearLoser3 {
    fn run(&mut self) -> u16 {
        let a = self.score_a;
        let b = self.score_b;
        let c = self.score_c;
        let bot = imin(imin(a, b), c);
        let top = imax(imax(a, b), c);
        let second_lowest = a.wrapping_add(b).wrapping_add(c).wrapping_sub(bot).wrapping_sub(top);
        ((second_lowest - bot) >= self.margin) as u16
    }
}
