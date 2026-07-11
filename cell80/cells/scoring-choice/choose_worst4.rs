//! Pick the value of whichever of four (value, score) candidates has the lowest score (ties -> lowest index, matching choose_worst3's convention) — the 4-candidate sibling of choose_worst3, mirroring choose_best4's structure with '>' flipped to '<'.
//! tags: choose, choice, worst, lowest, cheapest, selection, score, ranking, tie-break, plan, four, pick, cost, cheaper
//! entry: ChooseWorst4::run
struct ChooseWorst4 { val_a: u16, score_a: u16, val_b: u16, score_b: u16, val_c: u16, score_c: u16, val_d: u16, score_d: u16 }
impl ChooseWorst4 {
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
        if self.score_d < worst_score {
            worst_val = self.val_d;
            worst_score = self.score_d;
        }
        worst_val
    }
}
