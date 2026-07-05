//! Pick the value of whichever of two (value, score) candidates has the highest score (ties -> lowest index, matching choose_best3's convention) — the 2-candidate sibling of choose_best3, for the common case of only two options (e.g. "which of these two candidates has the highest profit").
//! tags: choose, choice, best, selection, score, ranking, tie-break, plan, two, pick, highest, profit, most
//! entry: ChooseBest2::run
struct ChooseBest2 { val_a: u16, score_a: u16, val_b: u16, score_b: u16 }
impl ChooseBest2 {
    fn run(&mut self) -> u16 {
        if self.score_b > self.score_a { self.val_b } else { self.val_a }
    }
}
