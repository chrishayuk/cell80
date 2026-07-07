//! The nth Catalan number (C(0)=1, C(n+1) = C(n)*2*(2n+1)/(n+2) — an exact recurrence, each step's division always lands evenly), checked: escalates on overflow rather than silently wrapping. Note the recurrence's own pre-division intermediate can overflow u32 before the true Catalan number itself would (the same class of limitation choose_u32 documents) — verified safe through C(17); beyond that, escalation is possible even though C(18)/C(19) themselves would still fit u32.
//! tags: number, catalan, combinatorics, sequence, counting, checked, wide, u32, escalate
//! entry: CatalanNumber::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the recurrence's intermediate product overflows u32 — this can trigger before the true Catalan number itself would exceed u32::MAX
struct CatalanNumber { n: u32, result: u32 }
impl CatalanNumber {
    fn run(&mut self) -> u16 {
        let mut c = 1u32;
        let mut k = 0u32;
        while k < self.n {
            let term = 2u32 * (2u32 * k + 1u32);
            let num = c.wrapping_mul(term);
            if c != 0u32 && num / c != term { halt(0xFF05u16); }
            c = num / (k + 2u32);
            k = k + 1u32;
        }
        self.result = c;
        1u16
    }
}
