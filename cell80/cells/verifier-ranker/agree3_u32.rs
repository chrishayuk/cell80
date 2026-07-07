//! Multi-plan agreement check at wide u32 width: returns 1 if at least two of three candidate answers are equal, else 0 — the wide sibling of majority3 (which works over u16 and can't represent answers beyond 65535, e.g. money totals in cents).
//! tags: verify, verifier, agreement, majority, multi-plan, consensus, wide, u32, check, plan
//! entry: Agree3Wide::run
struct Agree3Wide { a: u32, b: u32, c: u32, ok: u16 }
impl Agree3Wide {
    fn run(&mut self) -> u16 {
        let r = (self.a == self.b || self.b == self.c || self.a == self.c) as u16;
        self.ok = r;
        r
    }
}
