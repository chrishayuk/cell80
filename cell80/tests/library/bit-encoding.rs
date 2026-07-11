//! Host-oracle tests for the bit-encoding pack (`cell80/cells/bit-encoding/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::run_cell;

#[test]
fn first_wave_bit_encoding_cells_match_defined_behaviour() {
    let cases: &[(&str, &[u16], u16)] = &[
        ("low_byte", &[4660], 52),
        ("high_byte", &[4660], 18),
        ("swap_bytes", &[4660], 13330),
        ("rotl16", &[1, 1], 2),
        ("rotl16", &[32768, 1], 1),
        ("rotl16", &[1, 16], 1),
        ("rotr16", &[1, 1], 32768),
        ("rotr16", &[2, 1], 1),
        ("reverse_bits", &[1], 32768),
        ("reverse_bits", &[65535], 65535),
        ("leading_zeros", &[0], 16),
        ("leading_zeros", &[32768], 0),
        ("leading_zeros", &[255], 8),
        ("trailing_zeros", &[0], 16),
        ("trailing_zeros", &[8], 3),
        ("bit_length", &[0], 0),
        ("bit_length", &[256], 9),
        ("bit_length", &[32768], 16),
    ];

    let mut failures = Vec::new();
    for (id, args, exp) in cases {
        let got = run_cell(id, args);
        if got != *exp {
            failures.push(format!("{id}({args:?}) = {got}, expected {exp}"));
        }
    }
    assert!(
        failures.is_empty(),
        "cell mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn verify_leading_ones() {
    fn leading_ones(x: u16) -> u16 {
        run_cell("leading_ones", &[x])
    }

    // 1) Zero has no set bits at all -> 0 leading ones.
    assert_eq!(leading_ones(0x0000), 0);

    // 2) All bits set -> the full 16-bit run counts as 16 (mirrors leading_zeros(0) == 16).
    assert_eq!(leading_ones(0xFFFF), 16);

    // 3) Only the top bit set (0x8000 = 1000 0000 0000 0000): one leading one, then a clear bit.
    assert_eq!(leading_ones(0x8000), 1);

    // 4) Top three bits set (0xE000 = 1110 0000 0000 0000): three leading ones, then clear.
    assert_eq!(leading_ones(0xE000), 3);

    // 5) Top bit clear despite everything else set (0x7FFF = 0111 1111 1111 1111): 0 leading ones,
    //    since the run is measured from the MSB inward.
    assert_eq!(leading_ones(0x7FFF), 0);
}

#[test]
fn trailing_ones_matches_hand_computed_cases() {
    // trailing_ones mirrors trailing_zeros' loop shape but tests (v & 1) != 0 instead of
    // == 0, counting how many low bits are set to 1 before the first 0 bit (or all 16 for
    // x == 0xFFFF). Cases hand-computed against the binary expansion of each input.
    let cases: &[(&str, &[u16], u16)] = &[
        ("trailing_ones", &[0], 0),       // no bits set at all -> 0 trailing ones
        ("trailing_ones", &[0xFFFF], 16), // all 16 bits set -> saturates at 16
        ("trailing_ones", &[7], 3),       // 0b0000000000000111 -> 3 trailing ones
        ("trailing_ones", &[11], 2),      // 0b0000000000001011 -> bit2 is 0, so 2 trailing ones
        ("trailing_ones", &[0xFFFE], 0),  // 0b1111111111111110 -> low bit is 0, so 0 trailing ones
    ];

    let mut failures = Vec::new();
    for (id, args, exp) in cases {
        let got = run_cell(id, args);
        if got != *exp {
            failures.push(format!("{id}({args:?}) = {got}, expected {exp}"));
        }
    }
    assert!(
        failures.is_empty(),
        "cell mismatches:\n{}",
        failures.join("\n")
    );
}
