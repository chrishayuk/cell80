//! Host-oracle tests for the bit-mask pack (`cell80/cells/bit-mask/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::run_cell;

#[test]
fn first_wave_bit_mask_cells_match_defined_behaviour() {
    let cases: &[(&str, &[u16], u16)] = &[
        ("popcount", &[255], 8),
        ("popcount", &[65535], 16),
        ("popcount", &[0], 0),
        ("parity", &[7], 1),
        ("parity", &[255], 0),
        ("bit_is_set", &[8, 3], 1),
        ("bit_is_set", &[8, 2], 0),
        ("bit_is_set", &[32768, 15], 1),
        ("set_bit", &[0, 3], 8),
        ("set_bit", &[0, 15], 32768),
        ("clear_bit", &[15, 1], 13),
        ("clear_bit", &[8, 3], 0),
        ("toggle_bit", &[0, 3], 8),
        ("toggle_bit", &[8, 3], 0),
        ("mask_has_all", &[7, 5], 1),
        ("mask_has_all", &[5, 7], 0),
        ("mask_has_any", &[7, 4], 1),
        ("mask_has_any", &[7, 8], 0),
        ("mask_union", &[12, 10], 14),
        ("mask_intersection", &[12, 10], 8),
        ("mask_xor", &[12, 10], 6),
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
fn mask_has_none_matches_defined_behaviour() {
    // mask_has_none(x, mask): 1 iff (x & mask) == 0, i.e. x and mask share no set bits
    // (the exact logical complement of mask_has_any). Cases hand-computed below.
    let cases: &[(&str, &[u16], u16)] = &[
        ("mask_has_none", &[15, 5], 0), // 0b1111 & 0b0101 = 0b0101 != 0 -> overlap -> 0
        ("mask_has_none", &[8, 3], 1),  // 0b1000 & 0b0011 = 0 -> disjoint -> 1
        ("mask_has_none", &[0, 65535], 1), // x has no bits at all -> always disjoint -> 1
        ("mask_has_none", &[65535, 65535], 0), // full overlap -> 0
        ("mask_has_none", &[255, 256], 1), // low byte vs bit 8 -> disjoint bit ranges -> 1
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
fn mask_clear_matches_defined_behaviour() {
    // mask_clear(x, mask) clears every bit set in `mask` from x, i.e. x & !mask (the
    // mask-level generalization of clear_bit — the classic AND-NOT / "andn" op).
    let cases: &[(&str, &[u16], u16)] = &[
        // 15 (0b1111) with mask 5 (0b0101): clears bits 0 and 2 -> 0b1010 = 10
        ("mask_clear", &[15, 5], 10),
        // 8 (0b1000) with mask 3 (0b0011): no overlapping bits -> x unchanged = 8
        ("mask_clear", &[8, 3], 8),
        // 255 (0b11111111) with mask 15 (0b00001111): clears the low nibble -> 0b11110000 = 240
        ("mask_clear", &[255, 15], 240),
        // 0 with mask 65535: nothing was set to begin with -> stays 0
        ("mask_clear", &[0, 65535], 0),
        // 65535 with mask 0: clearing nothing -> x unchanged = 65535
        ("mask_clear", &[65535, 0], 65535),
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
fn verify_bit_not() {
    // Self-contained: compiles the cell fresh per call via the pack's run_cell helper.
    fn bit_not(x: u16) -> u16 {
        run_cell("bit_not", &[x])
    }

    // 1) All zero bits -> complement is all ones: 0x0000 ^ 0xFFFF = 0xFFFF = 65535.
    assert_eq!(bit_not(0x0000), 0xFFFF);

    // 2) All one bits -> complement is all zero: 0xFFFF ^ 0xFFFF = 0x0000 = 0.
    assert_eq!(bit_not(0xFFFF), 0x0000);

    // 3) Low byte set: 0x00FF ^ 0xFFFF = 0xFF00 = 65280.
    assert_eq!(bit_not(0x00FF), 0xFF00);

    // 4) Arbitrary value: 0x1234 ^ 0xFFFF = 0xEDCB.
    //    0xFFFF - 0x1234 = 65535 - 4660 = 60875 = 0xEDCB (XOR against all-ones is
    //    equivalent to subtraction from 0xFFFF since there's no borrow/carry).
    assert_eq!(bit_not(0x1234), 0xEDCB);

    // 5) Alternating pattern flips to its own complement: 0xAAAA ^ 0xFFFF = 0x5555.
    assert_eq!(bit_not(0xAAAA), 0x5555);
}
