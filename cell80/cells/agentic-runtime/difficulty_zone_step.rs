//! Difficulty-zone advance/stay/retreat decision from an accuracy tally against a target-accuracy band (target_pct +/- tolerance_pct), gated by a minimum sample count. Exact via cross-multiplication (correct*100 vs total*(target+-tolerance)) rather than dividing, so no float accuracy ratio is ever computed. Distinct from hysteresis (a raw single-value 2-state latch with no sample-size gate): this is a 3-way ratio decision over an explicit sample count.
//! tags: curriculum, difficulty, adaptive, zone, accuracy, threshold, agentic, state
//! entry: DifficultyZoneStep::run
struct DifficultyZoneStep { correct: u16, total: u16, target_pct: u16, tolerance_pct: u16, min_problems: u16, verdict: u16 }
impl DifficultyZoneStep {
    fn run(&mut self) -> u16 {
        if self.total < self.min_problems {
            self.verdict = 1u16; // hold: not enough samples yet
            return self.verdict;
        }
        let low_pct = if self.target_pct > self.tolerance_pct { self.target_pct - self.tolerance_pct } else { 0u16 };
        let high_pct = self.target_pct + self.tolerance_pct;
        let correct_100 = (self.correct as u32) * 100u32;
        let total_high = (self.total as u32) * (high_pct as u32);
        let total_low = (self.total as u32) * (low_pct as u32);
        if correct_100 > total_high {
            self.verdict = 2u16; // advance: accuracy above the band
        } else if correct_100 < total_low {
            self.verdict = 0u16; // retreat: accuracy below the band
        } else {
            self.verdict = 1u16; // hold: inside the band
        }
        self.verdict
    }
}
