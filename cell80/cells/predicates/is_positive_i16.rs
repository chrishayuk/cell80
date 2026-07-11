//! Returns 1 if x > 0 (strictly positive) under signed i16 ordering, else 0 -- orders against the implicit zero the pack's two-argument is_gt_i16/is_ge_i16/is_lt_i16/is_le_i16 never test alone, and unlike sign_i16 (which returns -1/0/1) this stays on the 0/1 predicate convention.
//! tags: predicate, positive, sign, compare, greater-than, boolean, signed, i16, ordering, zero
fn run(x: i16) -> u16 { (x > 0i16) as u16 }
