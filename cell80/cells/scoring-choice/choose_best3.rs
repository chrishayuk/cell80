//! Pick the value of whichever of three (value, score) candidates has the highest score (ties → lowest index, matching argmax3's convention) — distinct from argmax3, which assumes the value and the score are the same number.
//! tags: choose, choice, best, selection, score, ranking, tie-break, plan
//! entry: ChooseBest3::run
struct ChooseBest3 { val_a: u16, score_a: u16, val_b: u16, score_b: u16, val_c: u16, score_c: u16 }
impl ChooseBest3 {
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
        best_val
    }
}
