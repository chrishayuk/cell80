//! Width-bridge stress (A2, `docs/13-multi-target-spec.md` §2.2): the explicit
//! truncate / zero-extend / sign-extend family under the rustc oracle, across the
//! sign boundaries the dialect used to dodge (`i16 as u32` was a deliberate
//! rejection until the `SignExtend` bridge landed). Every case runs on both Z80
//! targets and the IR interpreter via `check!`.

#[test]
fn sign_extend_i16_to_u32() {
    // The high word takes the sign fill — probe both halves of the result.
    check!({
        let x = 0i16 - 3;
        let w = x as u32;
        (w >> 16) as u16
    });
    check!({
        let x = 0i16 - 3;
        let w = x as u32;
        w as u16
    });
    check!({
        let x = 7i16;
        let w = x as u32;
        ((w >> 16) as u16) ^ (w as u16)
    });
}

#[test]
fn sign_extend_boundaries() {
    // i16::MIN, -1, 0, i16::MAX — the four corners of the sign fill.
    check!({
        let x = -32768i16;
        let w = x as u32;
        ((w >> 16) as u16).wrapping_add(w as u16)
    });
    check!({
        let x = 0i16 - 1;
        let w = x as u32;
        (w >> 16) as u16
    });
    check!({
        let x = 0i16;
        let w = x as u32;
        ((w >> 16) as u16) | (w as u16)
    });
    check!({
        let x = 32767i16;
        let w = x as u32;
        (w >> 16) as u16
    });
}

#[test]
fn sign_extend_feeds_wide_arithmetic() {
    // The bridge exists to let signed values enter u32 lanes (the Q16.16 shape).
    check!({
        let x = 0i16 - 200;
        let scaled = (x as u32).wrapping_mul(256);
        (scaled >> 8) as u16
    });
    check!({
        let a = 0i16 - 5;
        let b = 3i16;
        (((a as u32) ^ (b as u32)) >> 16) as u16
    });
}

#[test]
fn take_the_bits_spelling_still_zero_extends() {
    // `x as u16 as u32` stays the bit-pattern route — high word zero.
    check!({
        let x = 0i16 - 3;
        let w = (x as u16) as u32;
        (w >> 16) as u16
    });
    check!({
        let x = 0i16 - 3;
        let w = (x as u16) as u32;
        w as u16
    });
}

#[test]
fn conversion_matrix_pins() {
    // The pre-existing bridges, pinned as a matrix alongside the new one:
    // u8 → u16/u32 zero-extend; u32 → u16/u8 truncate; round trips.
    check!({
        let b = 200u8;
        let w = b as u32;
        ((w >> 16) as u16) | (w as u16)
    });
    check!({
        let x = 0x1234_ABCDu32;
        (x as u16) ^ ((x >> 16) as u16)
    });
    check!({
        let x = 0x1234_ABCDu32;
        (x as u8) as u16
    });
    check!({
        let x = 40000u16;
        let w = x as u32;
        ((w as u16) == 40000u16) as u16
    });
}
