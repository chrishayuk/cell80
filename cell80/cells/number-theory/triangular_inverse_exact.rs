//! Solve n*(n+1)/2 = x for n, the exact inverse of triangular: given a triangular number x, return which n produced it. Escalates if x isn't triangular (a wrong-plan signal, e.g. GSM8K's "how many rows" problems). Domain matches triangular's own (n <= 361, x <= 65341).
//! tags: number, triangular, inverse, sequence, sum, series, math, escalate
//! limits: escalates (halt 0xFF06, out_of_domain) if x is not a triangular number
fn run(x: u16) -> u16 {
    let mut n = 0u16;
    let mut t = 0u16;
    while t < x {
        n = n + 1u16;
        t = t + n;
    }
    if t != x { halt(0xFF06u16); }
    n
}
