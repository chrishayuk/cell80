//! Verifies a claimed wide answer is within an absolute tolerance of the true value: returns 1 if |candidate - actual| <= tolerance, else 0 — distinct from within_percent (a percentage-based tolerance over u16); this is an absolute margin at wide u32 width.
//! tags: verify, verifier, tolerance, approximate, within, margin, wide, u32, check, plan
//! entry: AnswerWithinToleranceWide::run
struct AnswerWithinToleranceWide { candidate: u32, actual: u32, tolerance: u32, ok: u16 }
impl AnswerWithinToleranceWide {
    fn run(&mut self) -> u16 {
        let d = if self.candidate > self.actual { self.candidate - self.actual } else { self.actual - self.candidate };
        let r = (d <= self.tolerance) as u16;
        self.ok = r;
        r
    }
}
