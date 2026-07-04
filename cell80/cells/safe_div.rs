//! Integer divide a / b, returning 0 when b == 0 (no divide-by-zero).
// Also known as `quantize` (bucket index by step: x/step) — the admission gate found
// them behaviourally identical for every input; kept as an alias here, not a second cell
// (docs/library-growth.md: no behavioural duplicates).
//! tags: math, arithmetic, divide, division, quotient, safe, quantize, bucket, step, index, discretize, bin
fn run(a: u16, b: u16) -> u16 { if b != 0u16 { a / b } else { 0u16 } }
