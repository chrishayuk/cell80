//! Happy-number predicate: repeatedly replace n with the sum of the squares of its decimal digits, returning 1 if this reaches 1, else 0 -- detects the non-happy case by a bounded return to 4, the known entry point of the only other cycle, rather than an unbounded search. Distinct from every other digit predicate in the library (is_repdigit, is_automorphic_number, is_palindromic_number each check an unrelated digit property).
//! tags: number, happy-number, digit, square, sum, iterate, cycle, predicate, sequence, bounded
fn run(n: u16) -> u16 {
    if n == 0u16 { return 0u16; }
    let mut v = n;
    while v != 1u16 && v != 4u16 {
        let mut s = 0u16;
        let mut t = v;
        while t != 0u16 {
            let d = t % 10u16;
            s = s + d * d;
            t = t / 10u16;
        }
        v = s;
    }
    (v == 1u16) as u16
}
