//! Returns 1 if a and b fall in the same sign bucket (both >= 0 or both < 0, zero counted nonnegative), else 0 -- the direct callable form of the "neg_a == neg_b" test that is the load-bearing branch inside every sign-magnitude combine in the library (q_mul_i16, q_div_i16, lerp_i16), distinct from sign_i16 (single-value, three-way -1/0/1) and from smag_cmp/smag_eq (state cells comparing pre-decomposed magnitude/sign pairs, not raw i16 inputs).
//! tags: predicate, sign, same-sign, bucket, boolean, signed, i16, sign-magnitude, compare, zero
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
fn run(a: i16, b: i16) -> u16 {
    (i16_neg(a) == i16_neg(b)) as u16
}
