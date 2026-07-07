//! Returns 1 if a < b (strictly less than), else 0.
// Also known as `argmax2` (index 0/1 of the larger of two) — the admission gate found
// them behaviourally identical for every input; kept as an alias here, not a second cell
// (docs/library-growth.md: no behavioural duplicates).
//! tags: predicate, compare, less, less-than, smaller, boolean, argmax2, argmax, index, which, larger, choose, select
fn run(a: u16, b: u16) -> u16 { (a < b) as u16 }
