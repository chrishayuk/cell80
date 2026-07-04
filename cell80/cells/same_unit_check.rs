//! Unit-compatibility check for adding/subtracting two typed quantities: returns their shared dimension code if the units match, else escalates on a units mismatch (dimension codes documented in docs/library-growth.md).
//! tags: unit, units, dimension, money, time, distance, add, subtract, compatible, match, mismatch, checked
//! limits: escalates (halt 0xFF06, out_of_domain) if a and b differ, or either code is unrecognized (> 7)
fn run(a: u16, b: u16) -> u16 {
    if a > 7u16 || b > 7u16 { halt(0xFF06u16); }
    if a != b { halt(0xFF06u16); }
    a
}
