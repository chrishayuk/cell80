//! Returns 1 if x >= 0 under true signed ordering, else 0 -- the non-strict complement of is_positive_i16 (mirrors the is_gt_i16/is_ge_i16 strict/non-strict pairing) and distinct from verifier-ranker's smag_is_nonneg, which tests a (magnitude, sign) pair rather than a raw i16.
//! tags: predicate, nonneg, non-negative, sign, compare, greater-equal, boolean, signed, i16, ordering, zero
fn run(x: i16) -> u16 { (x >= 0i16) as u16 }
