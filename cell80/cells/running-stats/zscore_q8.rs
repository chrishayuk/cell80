//! Q8.8 fixed-point z-score given an already-computed standard deviation: (value - mean) scaled by 256, divided by stddev — sidesteps the sqrt-of-variance problem cosine_score_approx is still blocked on by taking stddev as an input rather than deriving it. Returns 0 if stddev_q8 <= 0 (the safe_div convention, no divide-by-zero).
//! tags: zscore, z-score, standardize, anomaly, statistics, fixed-point, q8.8, signed, i16
//! limits: assumes |value_q8 - mean_q8| < 128 so the <<8 pre-shift stays in i16 range (the dialect has no i32 to widen through) — like q_mul's own "assumes the product fits" convention
fn run(value_q8: i16, mean_q8: i16, stddev_q8: i16) -> i16 {
    if stddev_q8 <= 0i16 {
        0i16
    } else {
        let diff = value_q8 - mean_q8;
        (diff << 8) / stddev_q8
    }
}
