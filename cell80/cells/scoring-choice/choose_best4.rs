//! Pick the value of whichever of four (value, score) candidates has the highest score (ties -> lowest index, matching choose_best3's convention) — the 4-candidate sibling of choose_best3, for the case of four options (e.g. "which of these four bids scores highest").
//! tags: choose, choice, best, selection, score, ranking, tie-break, plan, four, pick, highest, top
//! entry: ChooseBest4::run
struct ChooseBest4 { val_a: u16, score_a: u16, val_b: u16, score_b: u16, val_c: u16, score_c: u16, val_d: u16, score_d: u16 }
impl ChooseBest4 {
    fn run(&mut self) -> u16 {
        let mut best_val = self.val_a;
        let mut best_score = self.score_a;
        if self.score_b > best_score {
            best_val = self.val_b;
            best_score = self.score_b;
        }
        if self.score_c > best_score {
            best_val = self.val_c;
            best_score = self.score_c;
        }
        if self.score_d > best_score {
            best_val = self.val_d;
            best_score = self.score_d;
        }
        best_val
    }
}
