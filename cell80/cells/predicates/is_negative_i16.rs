//! Returns 1 if x < 0 (strictly negative) under signed i16 ordering, else 0 -- the direct complement of is_positive_i16, rounding out the pack's zero/nonzero-style complement pairs (is_zero/nonzero, is_even/is_odd) for the signed-sign case.
//! tags: predicate, negative, sign, compare, less-than, boolean, signed, i16, ordering, zero
fn run(x: i16) -> u16 { (x < 0i16) as u16 }
