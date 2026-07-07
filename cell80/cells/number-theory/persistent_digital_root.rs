//! Additive persistence: the number of digit-summing passes needed to reduce n to a single digit (0 if n is already a single digit) -- the step count, not the resulting digit itself.
//! tags: number, persistent, additive-persistence, digit, sum, iterate, passes, steps, count, math
fn run(n: u16) -> u16 {
    let mut v = n;
    let mut steps = 0u16;
    while v >= 10u16 {
        let mut s = 0u16;
        let mut t = v;
        while t != 0u16 {
            s = s + t % 10u16;
            t = t / 10u16;
        }
        v = s;
        steps = steps + 1u16;
    }
    steps
}
