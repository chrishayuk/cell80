//! Unit-compatibility check for adding/subtracting two typed quantities: returns their shared dimension code if the units match, else escalates on a units mismatch (dimension codes documented in docs/library-growth.md, now including 8=rate_money_per_time and 9=rate_count_per_time).
//! tags: unit, units, dimension, money, time, distance, add, subtract, compatible, match, mismatch, checked
//! limits: escalates (halt 0xFF06, out_of_domain) if a and b differ, or either code is unrecognized (> 9)
fn run(a: u16, b: u16) -> u16 {
    if a > 9u16 || b > 9u16 { halt(0xFF06u16); }
    if a != b { halt(0xFF06u16); }
    a
}
