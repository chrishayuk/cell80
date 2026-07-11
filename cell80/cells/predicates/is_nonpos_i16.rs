//! Returns 1 if x <= 0 under signed i16 ordering, else 0 -- the non-strict complement of is_positive_i16 (which returns 1 only for x > 0), completing the sign-vs-zero family alongside is_gt_i16/is_ge_i16/is_lt_i16/is_le_i16.
//! tags: predicate, nonpositive, sign, compare, less-equal, boolean, signed, i16, ordering, zero
fn run(x: i16) -> u16 { (x <= 0i16) as u16 }
