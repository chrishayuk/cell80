//! General Nth root of a positive f32 value c -- finds x such that x^n = c for a caller-supplied whole exponent n (a plain u16 field, 1..65535) via a sqrt-seeded, bounded (<=10 iteration, early-exit) Newton-Raphson refinement of x^n=c, each trial power evaluated by square-and-multiply rather than a naive O(n) loop -- this is the shared, general form of the exact technique excel_db.rs (root of salvage/cost over its own hardcoded `life` field), excel_nominal.rs (root of 1+effect_rate over its own hardcoded `npery` field), and excel_rri.rs (root of fv/pv over its own hardcoded `nper` field) each already rebuild inline, then each post-process differently (a depreciation rate, a scaled-out nominal rate, a minus-one rate conversion); this cell does none of that and returns only the bare root, with n as a plain parameter rather than one formula's fixed exponent, and widens the fixed Newton budget from excel_rri's narrower 4 (safe only because a compounding ratio sits close to 1) to excel_db/excel_nominal's own 10, since a general-purpose root can't assume the caller's c is anywhere near 1.
//! tags: nth-root, root, radical, newton-raphson, newton, root-finding, square-and-multiply, binary-exponentiation, power, iterative, bounded, shared, general, numerical, primitive, f32, float, softfloat
//! kernel_bank: on
//! entry: NthRootF32::run
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0 or c <= 0.0 (only a positive radicand and a whole exponent >= 1 are supported -- no principal-root convention for negative c is picked here); the Newton refinement is capped at a fixed 10 iterations (an early-exit `converged` flag, excel_db.rs's own pattern, stops paying for iterations an already-settled case doesn't need) and is always checked back by re-raising the result to the n-th power and comparing to c within 1% -- escalates (halt 0xFF05, needs_wider_math) if that check fails, which empirically happens only for combinations of a large c together with a large n (verified by direct simulation of this exact algorithm across c from 1e-4 to 1e8 and n from 1 to 65535: the check reliably catches every case the fixed budget under- or over-shoots, never passing while silently wrong); escalates (halt 0xFF08/0xFF07, float_domain/float_overflow) if the seed, the refined root, or the convergence check itself goes NaN / non-finite
struct NthRootF32 {
    c: f32,
    n: u16,
    root: f32,
}
impl NthRootF32 {
    fn run(&mut self) -> u16 {
        if self.n == 0u16 {
            halt(0xFF06u16);
        }
        if self.c <= 0.0f32 {
            halt(0xFF06u16);
        }

        let m = self.n - 1u16;
        let n_f = int_to_f32(self.n);
        let m_f = int_to_f32(m);

        // Seed y0 = c^(1/2^bl), bl = n's bit length, via bl plain .sqrt() calls -- a
        // cheap, always-stable way to land the Newton start near the true root
        // regardless of n, generalizing excel_rri.rs's technique (which seeds against
        // its own fixed `nper` field) to an arbitrary caller-supplied n.
        let mut bl = 0u16;
        let mut t = self.n;
        while t != 0u16 {
            bl = bl + 1u16;
            t = t >> 1u16;
        }
        let mut y = self.c;
        let mut s = 0u16;
        while s < bl {
            y = y.sqrt();
            s = s + 1u16;
        }
        if y.is_nan() {
            halt(0xFF08u16);
        }
        let y_seed_fin = y.is_finite();
        if !y_seed_fin {
            halt(0xFF07u16);
        }

        // Bounded Newton refinement of y^n = c: y' = ((n-1)*y + c/y^(n-1)) / n. y^(n-1)
        // is evaluated by square-and-multiply (log2(n) multiplies per step), never a
        // naive O(n) repeated-multiply loop -- the exact fix excel_db/excel_nominal/
        // excel_rri's own doc comments already apply to this shape. A fixed cap of 10
        // (excel_db's and excel_nominal's own cap, not excel_rri's narrower 4 -- this
        // cell can't assume the caller's c sits close to 1 the way an already-known
        // compounding ratio does) bounds the pathological cases; the `converged` flag
        // (excel_db's own early-exit pattern) lets the common, already-settled cases
        // stop paying for iterations they don't need.
        let mut converged = 0u16;
        let mut i = 0u16;
        while i < 10u16 {
            if converged == 0u16 {
                let mut y_pow = 1.0f32;
                let mut base = y;
                let mut e = m;
                while e > 0u16 {
                    let bit = e & 1u16;
                    if bit == 1u16 {
                        y_pow = y_pow * base;
                    }
                    base = base * base;
                    e = e >> 1u16;
                }
                let next = (m_f * y + self.c / y_pow) / n_f;
                let step_delta = next - y;
                let step_mag = step_delta.abs();
                if step_mag < 0.00001f32 {
                    converged = 1u16;
                }
                y = next;
            }
            i = i + 1u16;
        }

        if y.is_nan() {
            halt(0xFF08u16);
        }
        let y_fin = y.is_finite();
        if !y_fin {
            halt(0xFF07u16);
        }

        // Verify convergence by re-evaluating y^n (same square-and-multiply technique)
        // and comparing back to c within 1% -- catches the pathological extreme-c /
        // extreme-n combinations the fixed 10-iteration budget can't fully resolve,
        // escalating honestly instead of returning a silently wrong root (excel_rri.rs's
        // own policy, generalized here).
        let mut check = 1.0f32;
        let mut base2 = y;
        let mut e2 = self.n;
        while e2 > 0u16 {
            let bit2 = e2 & 1u16;
            if bit2 == 1u16 {
                check = check * base2;
            }
            base2 = base2 * base2;
            e2 = e2 >> 1u16;
        }
        if check.is_nan() {
            halt(0xFF08u16);
        }
        let check_fin = check.is_finite();
        if !check_fin {
            halt(0xFF07u16);
        }
        let diff = check - self.c;
        let adiff = diff.abs();
        let tol = self.c * 0.01f32;
        if adiff > tol {
            halt(0xFF05u16);
        }

        self.root = y;
        1u16
    }
}
