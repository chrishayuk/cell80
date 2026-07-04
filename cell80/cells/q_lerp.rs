//! Linear interpolation from a to b by t (Q0.8 fraction, 0..256 = 0.0..1.0): a + (b-a)*t/256. Also an EMA step: q_lerp(prev, sample, alpha).
//! tags: fixed-point, q8.8, lerp, interpolate, blend, ema, moving-average, mix
fn run(a: u16, b: u16, t: u16) -> u16 {
    if b >= a {
        let diff = (b - a) as u32;
        a + ((diff * t as u32) >> 8u32) as u16
    } else {
        let diff = (a - b) as u32;
        a - ((diff * t as u32) >> 8u32) as u16
    }
}
