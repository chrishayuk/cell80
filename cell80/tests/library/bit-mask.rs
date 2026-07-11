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

#[test]
fn bit_is_clear_matches_defined_behaviour() {
    // bit_is_clear(x, bit): 1 iff bit `bit` of x is NOT set, i.e. ((x >> bit) & 1) == 0
    // (the exact logical complement of bit_is_set). Cases hand-computed below.
    let cases: &[(&str, &[u16], u16)] = &[
        ("bit_is_clear", &[8, 3], 0), // 8 = 0b1000, bit 3 is set -> not clear -> 0
        ("bit_is_clear", &[8, 2], 1), // 8 = 0b1000, bit 2 is 0 -> clear -> 1
        ("bit_is_clear", &[0, 0], 1), // 0 has no bits set at all -> bit 0 clear -> 1
        ("bit_is_clear", &[65535, 15], 0), // all bits set -> bit 15 set -> not clear -> 0
        ("bit_is_clear", &[32767, 15], 1), // 0b0111...1 -> bit 15 is the only unset bit -> clear -> 1
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
fn mask_missing_any_matches_defined_behaviour() {
    // mask_missing_any(x, mask): 1 iff (x & mask) != mask, i.e. x is missing at least
    // one bit that mask requires -- the exact logical complement of mask_has_all.
    // Cases hand-computed below (crate::common::run_cell is already in scope in
    // cell80/tests/library/bit-mask.rs via `use crate::common::run_cell;`).
    let cases: &[(&str, &[u16], u16)] = &[
        // 7=0b0111, 5=0b0101: x&mask=5==mask -> x has all of mask -> missing_any=0
        ("mask_missing_any", &[7, 5], 0),
        // 5=0b0101, 7=0b0111: x&mask=5 != 7 -> x is missing bit 1 of mask -> 1
        ("mask_missing_any", &[5, 7], 1),
        // 15,15: x&mask=15==mask -> has all -> 0
        ("mask_missing_any", &[15, 15], 0),
        // 0, 65535: x&mask=0 != 65535 -> missing every requested bit -> 1
        ("mask_missing_any", &[0, 65535], 1),
        // 65535, 65535: x&mask=65535==mask -> has all -> 0
        ("mask_missing_any", &[65535, 65535], 0),
        // 8=0b1000, 3=0b0011: x&mask=0 != 3 -> missing both mask bits -> 1
        ("mask_missing_any", &[8, 3], 1),
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
fn verify_hamming_distance16() {
    // hamming_distance16(a, b): count of bit positions where a and b differ, i.e.
    // popcount(a ^ b). Cases hand-computed below.
    fn hamming_distance16(a: u16, b: u16) -> u16 {
        run_cell("hamming_distance16", &[a, b])
    }

    // 1) Identical values -> xor is 0 -> no differing bits.
    assert_eq!(hamming_distance16(0, 0), 0);

    // 2) Full bitwise complement -> xor is 0xFFFF -> all 16 bits differ.
    assert_eq!(hamming_distance16(0xFFFF, 0x0000), 16);

    // 3) 5 (0b101) vs 3 (0b011): xor = 0b110 = 6 -> 2 bits set.
    assert_eq!(hamming_distance16(5, 3), 2);

    // 4) 0x1234 ^ 0x5678 = 0x444C (0100 0100 0100 1100) -> 5 bits set.
    assert_eq!(hamming_distance16(0x1234, 0x5678), 5);

    // 5) 0xABCD ^ 0x1234 = 0xB9F9 (1011 1001 1111 1001) -> 11 bits set.
    assert_eq!(hamming_distance16(0xABCD, 0x1234), 11);
}

// popcount_u32: the u32-width sibling of popcount -- counts set bits across a full 32-bit
// value via a u32 state field, since popcount's fn run(x: u16) cannot represent inputs with
// bits above position 15 at all. Cases hand-computed: zero, all-32-bits-set, a lone high bit
// (bit 31) to prove wide bits count, a lone bit above the u16 domain (bit 16) which is exactly
// the case popcount cannot even express as input, the all-u16-domain-bits-set case cross-checked
// against popcount(65535)==16, and a mixed nibble pattern.
#[test]
fn popcount_u32_matches_hand_computed_expectations() {
    fn step(x: u64) -> u64 {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("popcount_u32"),
            "PopcountU32",
            None,
        )
        .unwrap_or_else(|e| panic!("bind popcount_u32: {e}"));
        cell.set("x", x).unwrap();
        cell.run(cell80::DEFAULT_CYCLES).unwrap();
        cell.get("out").unwrap_or_else(|| panic!("no out field"))
    }

    // 1) x=0 -> no bits set -> 0.
    assert_eq!(step(0), 0);

    // 2) x=0xFFFFFFFF -> all 32 bits set -> 32.
    assert_eq!(step(0xFFFF_FFFF), 32);

    // 3) x=0x80000000 (bit 31 only) -> 1. Confirms high bits above the u16 domain count.
    assert_eq!(step(0x8000_0000), 1);

    // 4) x=0x00010000 (bit 16 only, beyond popcount's 16-bit domain) -> 1. This is exactly
    //    the case popcount (u16-only) cannot represent as input at all.
    assert_eq!(step(0x0001_0000), 1);

    // 5) x=0xFFFF (fits in u16 domain) -> 16, matching popcount(65535)=16's existing behaviour.
    assert_eq!(step(0xFFFF), 16);

    // 6) x=0x12345678 -> nibble popcounts 1,1,2,1,2,2,3,1 -> sum = 13.
    assert_eq!(step(0x1234_5678), 13);
}

#[test]
fn bit_is_set_u32_wide_sibling_matches_hand_computed_cases() {
    // bit_is_set_u32: the wide (u32-domain) sibling of bit_is_set, returns 1 if bit
    // number `bit` (0-31) of x is set, else 0. Needs a u32 state field (state cell)
    // since bit_is_set's fn run(x: u16, bit: u16) can't accept a 32-bit x under the
    // 16-bit calling convention. Cases hand-computed via (x >> bit) & 1.
    fn bit_is_set_u32(x: u32, bit: u16) -> u16 {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("bit_is_set_u32"),
            "BitIsSetU32",
            None,
        )
        .unwrap_or_else(|e| panic!("bind bit_is_set_u32: {e}"));
        cell.set("x", x as u64).unwrap();
        cell.set("bit", bit as u64).unwrap();
        cell.run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run bit_is_set_u32: {e}"));
        cell.get("out").unwrap() as u16
    }

    // low word, bit set: 8 = 0b1000, bit 3 -> 1
    assert_eq!(bit_is_set_u32(8, 3), 1);
    // low word, bit clear: 8 = 0b1000, bit 2 -> 0
    assert_eq!(bit_is_set_u32(8, 2), 0);
    // high word, top bit set: 0x80000000, bit 31 -> 1
    assert_eq!(bit_is_set_u32(0x80000000, 31), 1);
    // high word, bit clear: 0x80000000, bit 30 -> 0
    assert_eq!(bit_is_set_u32(0x80000000, 30), 0);
    // low/high boundary: 0x00010000, bit 16 (first bit of high word) -> 1
    assert_eq!(bit_is_set_u32(0x00010000, 16), 1);
    // all bits set, low bit: 0xFFFFFFFF, bit 0 -> 1
    assert_eq!(bit_is_set_u32(0xFFFFFFFF, 0), 1);
}
