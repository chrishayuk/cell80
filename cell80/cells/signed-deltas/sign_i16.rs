//! Sign of a signed value: 1 if positive, -1 if negative, 0 if zero.
//! tags: sign, signed, i16, delta, positive, negative, direction
fn run(x: i16) -> i16 {
    if x > 0i16 {
        1i16
    } else if x < 0i16 {
        -1i16
    } else {
        0i16
    }
}
