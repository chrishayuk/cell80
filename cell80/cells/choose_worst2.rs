//! Pick the value of whichever of two (value, score) candidates has the lowest score (ties -> lowest index, matching choose_best2's convention) — the inverse-comparison sibling of choose_best2, for the common "which of these two costs less" shape.
//! tags: choose, choice, worst, lowest, cheapest, selection, score, ranking, tie-break, plan, two, pick, cost, cheaper
//! entry: ChooseWorst2::run
struct ChooseWorst2 { val_a: u16, score_a: u16, val_b: u16, score_b: u16 }
impl ChooseWorst2 {
    fn run(&mut self) -> u16 {
        if self.score_b < self.score_a { self.val_b } else { self.val_a }
    }
}
