//! Maximum of three signed 16-bit values under true signed ordering (-1 > -32768) -- the three-operand sibling of max_i16, chained the same way min3_i16 chains imin, using plain i16 comparison since ordering never needs to combine magnitudes the way sign-magnitude add/subtract does.
//! tags: max, maximum, largest, signed, i16, compare, ordering, three, extremum
fn run(a: i16, b: i16, c: i16) -> i16 {
    let ab = if a > b { a } else { b };
    if ab > c {
        ab
    } else {
        c
    }
}
