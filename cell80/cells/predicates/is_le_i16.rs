//! Returns 1 if a <= b (at most) under true signed ordering, else 0 -- the non-strict sibling of is_lt_i16/is_gt_i16 and the signed counterpart of is_le, which bit-reinterprets negative values as large positives and so orders them wrong.
//! tags: predicate, compare, less-equal, le, at-most, no-more-than, signed, i16, ordering, negative, boolean
fn run(a: i16, b: i16) -> u16 { (a <= b) as u16 }
