//! Excel-compatible EVEN(number): rounds a signed integer UP, away from zero, to the nearest even integer (EVEN(3)=4, EVEN(2)=2, EVEN(-1)=-2, EVEN(0)=0) -- composed exactly as the catalog maps it: abs_i16's own magnitude split (mag = 0u16.wrapping_sub(number as u16) when negative, never native i16 subtraction) feeds div_ceil_u32's own q+1-if-remainder technique doubled back (ceil_mag = (mag/2, +1 if mag is odd) * 2, landing on the next even integer at or above mag with no separate parity-bump step), then the sign is reapplied the way negate_i16 does (negate the magnitude only when number was negative); this checked-int-tier, fixed-step-2 composition is what distinguishes EVEN from its own pack sibling ODD (an f32-tier cell rounding to the nearest ODD integer instead, since Excel's ODD also accepts fractional inputs), from MROUND (rounds to the NEAREST multiple of an arbitrary caller-supplied significance, not always away from zero, and errors on a sign mismatch between number and multiple), and from CEILING.MATH/CEILING.PRECISE (round toward an arbitrary significance with a mode flag or an unconditional +infinity direction, never fixed at the constant step 2 on both sides of zero).
//! tags: excel, even, round, round-up, ceiling, away-from-zero, parity, nearest-even, magnitude, signed, i16, sign-magnitude, math-trig
//! limits: escalates (halt 0xFF05, needs_wider_math) if number == 32767i16 (i16::MAX, odd), since its rounded-up magnitude 32768 has no representation as a *positive* i16 -- the negative side never hits this ceiling, since abs_i16's own magnitude of i16::MIN is already exactly 32768 (even, no rounding needed) and negating 32768 lands back on exactly i16::MIN.
fn run(number: i16) -> i16 {
    // mag = abs_i16(number): strip the sign via wrapping_sub, never native i16 subtraction.
    let mag = if number < 0i16 { 0u16.wrapping_sub(number as u16) } else { number as u16 };

    // ceil_mag = div_ceil_u32(mag, 2) * 2: div_ceil_u32's own q+1-if-remainder technique
    // (checked-arithmetic), doubled back to land on the next even integer at or above mag.
    let q = mag / 2u16;
    let r = mag % 2u16;
    let rounded = if r != 0u16 { q + 1u16 } else { q };
    let ceil_mag = rounded * 2u16;

    // Reapply the original sign (negate_i16's own technique): negate the magnitude only
    // when number was negative. On the positive side, escalate if the rounded-up magnitude
    // no longer fits a positive i16 (only reachable at number == i16::MAX).
    if number < 0i16 {
        (0u16.wrapping_sub(ceil_mag)) as i16
    } else {
        if ceil_mag > 32767u16 { halt(0xFF05u16); }
        ceil_mag as i16
    }
}
