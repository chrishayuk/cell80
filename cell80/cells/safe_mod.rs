//! Remainder a % b, returning 0 when b == 0.
// Also known as `wrap` (wrap x into the cycle [0, m)) — the admission gate found them
// behaviourally identical for every input; kept as an alias here, not a second cell
// (docs/library-growth.md: no behavioural duplicates).
//! tags: math, arithmetic, modulo, remainder, mod, safe, wrap, cycle, around, period, bounds
fn run(a: u16, b: u16) -> u16 { if b != 0u16 { a % b } else { 0u16 } }
