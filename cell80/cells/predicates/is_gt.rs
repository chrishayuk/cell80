//! Returns 1 if a > b (strictly greater than), else 0.
// Also known as `argmin2` (index 0/1 of the smaller of two) — the admission gate found
// them behaviourally identical for every input; kept as an alias here, not a second cell
// (docs/library-growth.md: no behavioural duplicates).
//! tags: predicate, compare, greater, greater-than, larger, boolean, argmin2, argmin, index, which, smaller, choose, select
fn run(a: u16, b: u16) -> u16 { (a > b) as u16 }
