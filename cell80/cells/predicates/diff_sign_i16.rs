//! Returns 1 if a and b fall in different sign buckets (one >=0, the other <0), else 0 -- the direct complement of same_sign_i16, rounding out the pack's eq/neq-style complement pair for the signed-sign-bucket case.
//! tags: predicate, sign, compare, different, differs, boolean, signed, i16, ordering, bucket, complement
fn run(a: i16, b: i16) -> u16 { ((a >= 0i16) != (b >= 0i16)) as u16 }
