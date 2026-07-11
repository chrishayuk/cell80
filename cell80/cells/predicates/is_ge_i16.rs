//! Returns 1 if a >= b (at least) under true signed ordering, else 0 -- the non-strict sibling of is_gt_i16/is_lt_i16 and the signed counterpart of is_ge, which bit-reinterprets negative values as large positives and so orders them wrong.
//! tags: predicate, compare, greater-equal, ge, at-least, no-less-than, signed, i16, ordering, negative, boolean
fn run(a: i16, b: i16) -> u16 { (a >= b) as u16 }
