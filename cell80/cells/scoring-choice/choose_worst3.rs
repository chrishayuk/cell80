//! Pick the value of whichever of three (value, score) candidates has the lowest score (ties -> lowest index, matching choose_worst2's convention) — the 3-candidate sibling of choose_worst2, and the lowest-score counterpart of choose_best3.
//! tags: choose, choice, worst, lowest, cheapest, selection, score, ranking, tie-break, plan, three, pick, cost, cheaper
//! entry: ChooseWorst3::run
struct ChooseWorst3 { val_a: u16, score_a: u16, val_b: u16, score_b: u16, val_c: u16, score_c: u16 }
impl ChooseWorst3 {
    fn run(&mut self) -> u16 {
        let mut worst_val = self.val_a;
        let mut worst_score = self.score_a;
        if self.score_b < worst_score {
            worst_val = self.val_b;
            worst_score = self.score_b;
        }
        if self.score_c < worst_score {
            worst_val = self.val_c;
            worst_score = self.score_c;
        }
        worst_val
    }
}
