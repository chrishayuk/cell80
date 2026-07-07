//! Generalized Lucas sequence pair U_n/V_n for parameters p, q (both non-negative): U(0)=0, U(1)=1, U(n)=p*U(n-1)+q*U(n-2); V(0)=2, V(1)=p, V(n)=p*V(n-1)+q*V(n-2) -- both share one recurrence structure, so one cell computes them together. p=2,q=1 gives the Pell numbers (U) and companion Pell / Pell-Lucas numbers (V) -- pell_number and pell_lucas_number are not shipped as separate cells for exactly that reason. p=1,q=1 reproduces fibonacci_checked_u32 (U) and the classic Lucas numbers (V); fibonacci_checked_u32 stays its own cell for its own retrieval identity, not folded away, the same precedent triangular/polygonal_number(3,n) already set.
//! tags: number, lucas, sequence, pell, fibonacci, recurrence, checked, wide, u32, escalate
//! entry: LucasUV::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if U(n) or V(n) would exceed u32::MAX
struct LucasUV { p: u32, q: u32, n: u32, u: u32, v: u32 }
impl LucasUV {
    fn run(&mut self) -> u16 {
        let mut ua = 0u32;
        let mut ub = 1u32;
        let mut va = 2u32;
        let mut vb = self.p;
        let mut i = 1u32;
        while i < self.n {
            let pu = mul_checked_u32(self.p, ub);
            let qu = mul_checked_u32(self.q, ua);
            let un = add_checked_u32(pu, qu);
            let pv = mul_checked_u32(self.p, vb);
            let qv = mul_checked_u32(self.q, va);
            let vn = add_checked_u32(pv, qv);
            ua = ub;
            ub = un;
            va = vb;
            vb = vn;
            i = i + 1u32;
        }
        if self.n == 0u32 {
            self.u = ua;
            self.v = va;
        } else {
            self.u = ub;
            self.v = vb;
        }
        1u16
    }
}
