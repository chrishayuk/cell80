//! Host-oracle tests for the packing-bcd pack (`cell80/cells/packing-bcd/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::{cell_src, run_cell};
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn first_wave_packing_bcd_cells_match_defined_behaviour() {
    let cases: &[(&str, &[u16], u16)] = &[
        ("pack_u8", &[0x12, 0x34], 0x1234),
        ("pack_u8", &[0x1FF, 0x2FF], 0xFFFF), // out-of-range inputs mask cleanly
        ("pack_nibbles", &[0xA, 0x5], 0xA5),
        ("bcd_encode", &[42], 0x42),
        ("bcd_encode", &[0], 0),
        ("bcd_decode", &[0x42], 42),
        ("bcd_decode", &[0], 0),
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
fn nibble_hi_matches_defined_behaviour() {
    // (x >> 4) & 0xF: the high nibble of x's low byte. Hand-computed cases:
    //  - 0x0000 -> low byte 0x00 -> high nibble 0
    //  - 0x00A5 -> low byte 0xA5 = 1010_0101 -> high nibble 0xA
    //  - 0x1234 -> low byte 0x34 = 0011_0100 -> high nibble 0x3 (upper byte of x is irrelevant)
    //  - 0x12FF -> low byte 0xFF -> high nibble 0xF
    //  - 0x000F -> low byte 0x0F = 0000_1111 -> high nibble 0
    let cases: &[(u16, u16)] = &[
        (0x0000, 0x0),
        (0x00A5, 0xA),
        (0x1234, 0x3),
        (0x12FF, 0xF),
        (0x000F, 0x0),
    ];
    for (x, exp) in cases {
        let got = run_cell("nibble_hi", &[*x]);
        assert_eq!(
            got, *exp,
            "nibble_hi({x:#06x}) = {got:#x}, expected {exp:#x}"
        );
    }
}

#[test]
fn nibble_lo_extracts_the_low_4_bits() {
    // nibble_lo(x) = x & 0xF -- the low-nibble counterpart to nibble_hi, distinct from
    // low_byte's byte-level (x & 0xFF) mask. Cases hand-computed:
    //   0x0000 & 0xF = 0x0
    //   0x1234 & 0xF = 0x4
    //   0x00FF & 0xF = 0xF (15)
    //   0x0009 & 0xF = 0x9
    //   0xABCD & 0xF = 0xD (13)
    let cases: &[(&str, &[u16], u16)] = &[
        ("nibble_lo", &[0x0000], 0x0),
        ("nibble_lo", &[0x1234], 0x4),
        ("nibble_lo", &[0x00FF], 0xF),
        ("nibble_lo", &[0x0009], 0x9),
        ("nibble_lo", &[0xABCD], 0xD),
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

// Verifies pack_u16_pair concatenates two u16 halves into one u32 as (hi << 16) | lo,
// the u32-width rung above pack_u8's (hi << 8) | lo. Cases hand-computed: a generic
// mixed-nibble pair, both-zero, both-max (checks no wraparound/overflow), and the two
// "one half is zero" edges that isolate hi's placement from lo's.
#[test]
fn pack_u16_pair_state_cell_matches_defined_behaviour() {
    fn pack(hi: u16, lo: u16) -> u32 {
        let mut cell = StateCell::bind(&cell_src("pack_u16_pair"), "PackU16Pair", None)
            .unwrap_or_else(|e| panic!("bind pack_u16_pair: {e}"));
        cell.set("hi", hi as u64).unwrap();
        cell.set("lo", lo as u64).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap();
        cell.get("out").unwrap_or_else(|| panic!("no out field")) as u32
    }

    assert_eq!(pack(0x1234, 0x5678), 0x1234_5678); // generic mixed halves
    assert_eq!(pack(0, 0), 0); // both zero
    assert_eq!(pack(0xFFFF, 0xFFFF), 0xFFFF_FFFF); // both max, no overflow/wrap
    assert_eq!(pack(1, 0), 65536); // hi=1 alone -> 0x00010000
    assert_eq!(pack(0, 0xFFFF), 65535); // lo=0xFFFF alone -> 0x0000FFFF
}

// unpack_u16_pair: the inverse of pack_u16_pair — splits a u32 back into (hi, lo) u16 halves
// via hi = in_val >> 16, lo = in_val & 0xFFFF. Requires `use cell80::{StateCell, DEFAULT_CYCLES};`
// and `crate::common::cell_src` at the top of this pack test file (not yet imported there since
// packing-bcd previously had only free-fn cells).
#[test]
fn unpack_u16_pair_matches_defined_behaviour() {
    fn step(in_val: u64) -> (u64, u64) {
        let mut cell = StateCell::bind(&cell_src("unpack_u16_pair"), "UnpackU16Pair", None)
            .unwrap_or_else(|e| panic!("bind unpack_u16_pair: {e}"));
        cell.set("in_val", in_val).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.result, 1, "status flag should be 1");
        (cell.get("hi").unwrap(), cell.get("lo").unwrap())
    }

    // Round-trip corners plus a mixed value, mirroring the morton_encode/morton_decode
    // corner-value tests already in tests/library/spatial-grid.rs.
    assert_eq!(step(0), (0, 0));
    assert_eq!(step(0x12345678), (0x1234, 0x5678));
    assert_eq!(step(0xFFFFFFFF), (0xFFFF, 0xFFFF));
    assert_eq!(step(0x0000FFFF), (0, 0xFFFF));
    assert_eq!(step(0xFFFF0000), (0xFFFF, 0));
}

// Verifies bcd_encode16 packs a four-digit decimal value into a full u16 as one BCD digit
// per nibble (thousands<<12 | hundreds<<8 | tens<<4 | units) -- the 4-nibble extension of
// bcd_encode's 2-digit/1-byte form. Cases hand-computed: zero, all-nines max, a generic
// mixed-digit value, a small value with leading zero digits, and a value with an internal
// zero digit (tests that the zero digit's nibble stays cleanly zero, not skipped).
#[test]
fn bcd_encode16_matches_defined_behaviour() {
    let cases: &[(&str, &[u16], u16)] = &[
        ("bcd_encode16", &[0], 0x0000),    // all digits zero
        ("bcd_encode16", &[9999], 0x9999), // max four-digit value, no nibble overflow
        ("bcd_encode16", &[1234], 0x1234), // generic distinct digits
        ("bcd_encode16", &[42], 0x0042),   // leading zero digits (thousands, hundreds)
        ("bcd_encode16", &[8005], 0x8005), // internal zero digits (hundreds, tens)
    ];

    let mut failures = Vec::new();
    for (id, args, exp) in cases {
        let got = run_cell(id, args);
        if got != *exp {
            failures.push(format!("{id}({args:?}) = {got:#06x}, expected {exp:#06x}"));
        }
    }
    assert!(
        failures.is_empty(),
        "cell mismatches:\n{}",
        failures.join("\n")
    );
}

// Verifies bcd_decode16 unpacks a 4-nibble packed-BCD u16 back to its binary value:
// each nibble is a decimal digit weighted by its place (1000/100/10/1), summed.
// This is the inverse of bcd_encode16, the 4-digit extension of the existing
// bcd_encode/bcd_decode 2-digit pair. Cases hand-computed: all-zero, all-nine (max
// valid 4-digit BCD 9999), a generic mixed-digit value, an internal leading-zero
// digit (isolates each nibble's place value), and a single low digit.
#[test]
fn bcd_decode16_matches_defined_behaviour() {
    let cases: &[(&str, &[u16], u16)] = &[
        ("bcd_decode16", &[0x0000], 0),
        ("bcd_decode16", &[0x9999], 9999),
        ("bcd_decode16", &[0x1234], 1234),
        ("bcd_decode16", &[0x0507], 507),
        ("bcd_decode16", &[0x0001], 1),
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

// Verifies bcd16_is_valid checks that all four nibbles of a packed 4-digit BCD u16 are
// valid decimal digits (0-9) -- the 4-nibble extension of bcd_is_valid's 2-nibble check,
// mirroring the bcd_encode/bcd_encode16 2-digit/4-digit ladder. Cases hand-computed:
// all-zero (valid), all-nine max (valid, boundary), a generic valid value, and one
// invalid case per nibble position (thousands, hundreds, units) to isolate that every
// nibble is actually checked, not just the first or last.
#[test]
fn bcd16_is_valid_matches_defined_behaviour() {
    let cases: &[(&str, &[u16], u16)] = &[
        ("bcd16_is_valid", &[0x0000], 1), // all digits zero
        ("bcd16_is_valid", &[0x9999], 1), // max valid four-digit BCD
        ("bcd16_is_valid", &[0x1234], 1), // generic valid value
        ("bcd16_is_valid", &[0xF123], 0), // thousands nibble 0xF > 9
        ("bcd16_is_valid", &[0x1A23], 0), // hundreds nibble 0xA > 9
        ("bcd16_is_valid", &[0x123A], 0), // units nibble 0xA > 9
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

// Verifies bcd_add sums two packed 2-digit BCD bytes via per-nibble decimal-carry
// correction (the Z80 ADD+DAA idiom): each nibble sum over 9 gets +6 corrected, and a
// carry propagates from the low nibble into the high-nibble sum. Cases hand-computed:
// a plain add with an internal nibble carry but no overall carry, max+max (overflow of
// both nibbles), zero+zero, an exact hundred boundary (tests the carry-out flag fires
// right at 100), and a zero addend (identity-like case).
#[test]
fn bcd_add_matches_hand_computed_cases() {
    fn add(a: u16, b: u16) -> (u16, u16) {
        let mut cell = StateCell::bind(&cell_src("bcd_add"), "BcdAdd", None)
            .unwrap_or_else(|e| panic!("bind bcd_add: {e}"));
        cell.set("a", a as u64).unwrap();
        cell.set("b", b as u64).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.result, 1, "status flag should be 1");
        (
            cell.get("sum").unwrap() as u16,
            cell.get("carry").unwrap() as u16,
        )
    }

    // 45 + 38 = 83 (nibble carry in the low digit, no overall carry)
    assert_eq!(add(0x45, 0x38), (0x83, 0));
    // 99 + 99 = 198 -> mod 100 = 98, carry = 1 (max + max)
    assert_eq!(add(0x99, 0x99), (0x98, 1));
    // 0 + 0 = 0, no carry
    assert_eq!(add(0x00, 0x00), (0x00, 0));
    // 59 + 41 = 100 -> mod 100 = 0, carry = 1 (exact boundary)
    assert_eq!(add(0x59, 0x41), (0x00, 1));
    // 12 + 0 = 12, no carry (zero addend)
    assert_eq!(add(0x12, 0x00), (0x12, 0));
}

// bcd_sub: subtracts two packed 2-digit BCD bytes (tens in high nibble, units in low
// nibble) via per-nibble decimal-borrow correction, producing the packed-BCD difference
// plus a borrow-out flag. Requires `use cell80::{StateCell, DEFAULT_CYCLES};` and
// `crate::common::cell_src` at the top of this pack test file.
#[test]
fn bcd_sub_matches_hand_computed_values() {
    fn bcd_sub(a: u16, b: u16) -> (u16, u16) {
        let mut cell = StateCell::bind(&cell_src("bcd_sub"), "BcdSub", None)
            .unwrap_or_else(|e| panic!("bind bcd_sub: {e}"));
        cell.set("a", a as u64).unwrap();
        cell.set("b", b as u64).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.result, 1, "status flag should be 1");
        let diff = cell.get("diff").unwrap_or_else(|| panic!("no diff field")) as u16;
        let borrow = cell
            .get("borrow")
            .unwrap_or_else(|| panic!("no borrow field")) as u16;
        (diff, borrow)
    }

    // 59 - 27 = 32, no borrow anywhere (9>=7, 5>=2).
    assert_eq!(bcd_sub(0x59, 0x27), (0x32, 0));
    // 50 - 27 = 23: low nibble borrows (0<7 -> 0+10-7=3, carry 1), high absorbs it (5-2-1=2).
    assert_eq!(bcd_sub(0x50, 0x27), (0x23, 0));
    // 12 - 34 = -22 -> decimal-borrow wraps to 100-22=78, borrow-out=1.
    assert_eq!(bcd_sub(0x12, 0x34), (0x78, 1));
    // 0 - 0 = 0, no borrow.
    assert_eq!(bcd_sub(0x00, 0x00), (0x00, 0));
    // 99 - 99 = 0, equal digit-by-digit, no borrow.
    assert_eq!(bcd_sub(0x99, 0x99), (0x00, 0));
    // 0 - 1 = -1 -> wraps to 100-1=99, borrow-out=1 (both nibbles borrow through zero).
    assert_eq!(bcd_sub(0x00, 0x01), (0x99, 1));
}

// Verifies pack_bytes4 concatenates four byte values into one u32 as
// (b3 << 24) | (b2 << 16) | (b1 << 8) | b0 -- the 4x8-bit rung above pack_u16_pair's
// 2x16-bit form, needed because four inputs exceed a free fn's 3-param cap. Cases
// hand-computed: a generic mixed-byte value, all-zero, all-max (checks no
// overflow/wrap across the full u32), each single-byte-alone edge that isolates b3's
// and b1's placement, and out-of-range inputs that must mask cleanly to their low byte.
#[test]
fn pack_bytes4_state_cell_matches_defined_behaviour() {
    fn pack(b3: u16, b2: u16, b1: u16, b0: u16) -> u32 {
        let mut cell = StateCell::bind(&cell_src("pack_bytes4"), "PackBytes4", None)
            .unwrap_or_else(|e| panic!("bind pack_bytes4: {e}"));
        cell.set("b3", b3 as u64).unwrap();
        cell.set("b2", b2 as u64).unwrap();
        cell.set("b1", b1 as u64).unwrap();
        cell.set("b0", b0 as u64).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.result, 1, "status flag should be 1");
        cell.get("out").unwrap_or_else(|| panic!("no out field")) as u32
    }

    assert_eq!(pack(0x12, 0x34, 0x56, 0x78), 0x1234_5678); // generic mixed bytes
    assert_eq!(pack(0, 0, 0, 0), 0); // all zero
    assert_eq!(pack(0xFF, 0xFF, 0xFF, 0xFF), 0xFFFF_FFFF); // all max, no overflow/wrap
    assert_eq!(pack(1, 0, 0, 0), 16_777_216); // b3 alone -> 0x01000000
    assert_eq!(pack(0, 0, 1, 0), 256); // b1 alone -> 0x00000100
    assert_eq!(pack(0x1FF, 0, 0, 0x2FF), 0xFF00_00FF); // out-of-range inputs mask cleanly
}

// unpack_bytes4: the inverse of pack_bytes4 — splits a u32 back into its four constituent
// bytes (b3 highest .. b0 lowest) via b3 = (in_val >> 24) & 0xFF, b2 = (in_val >> 16) & 0xFF,
// b1 = (in_val >> 8) & 0xFF, b0 = in_val & 0xFF. Uses `crate::common::cell_src` and
// `cell80::{StateCell, DEFAULT_CYCLES}`, both already imported at the top of this pack test file.
#[test]
fn unpack_bytes4_matches_defined_behaviour() {
    fn step(in_val: u64) -> (u64, u64, u64, u64) {
        let mut cell = StateCell::bind(&cell_src("unpack_bytes4"), "UnpackBytes4", None)
            .unwrap_or_else(|e| panic!("bind unpack_bytes4: {e}"));
        cell.set("in_val", in_val).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.result, 1, "status flag should be 1");
        (
            cell.get("b3").unwrap(),
            cell.get("b2").unwrap(),
            cell.get("b1").unwrap(),
            cell.get("b0").unwrap(),
        )
    }

    // Round-trip corners plus a mixed value, mirroring unpack_u16_pair's own corner-value
    // test cases (all-zero, generic mixed, all-ones, single-byte-set at each extreme).
    assert_eq!(step(0), (0, 0, 0, 0));
    assert_eq!(step(0x12345678), (0x12, 0x34, 0x56, 0x78));
    assert_eq!(step(0xFFFFFFFF), (0xFF, 0xFF, 0xFF, 0xFF));
    assert_eq!(step(0x000000FF), (0, 0, 0, 0xFF));
    assert_eq!(step(0xFF000000), (0xFF, 0, 0, 0));
}

// Verifies bcd_add16 sums two packed 4-digit BCD u16 values via a 4-nibble decimal-carry
// chain -- the width sibling of bcd_add's 2-nibble byte form, following the
// bcd_encode/bcd_encode16 2-digit/4-digit ladder. Cases hand-computed: a generic sum with
// two internal nibble carries but no carry-out, a full ripple-through-all-four-nibbles
// wraparound (9999+1=10000 mod 10000=0, carry=1), the zero baseline, a different
// wraparound combination that also carries out through the top nibble, and a partial
// ripple that stops cleanly at the hundreds digit (no carry-out).
#[test]
fn bcd_add16_matches_defined_behaviour() {
    fn add16(a: u16, b: u16) -> (u16, u16) {
        let mut cell = StateCell::bind(&cell_src("bcd_add16"), "BcdAdd16", None)
            .unwrap_or_else(|e| panic!("bind bcd_add16: {e}"));
        cell.set("a", a as u64).unwrap();
        cell.set("b", b as u64).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.result, 1, "status flag should be 1");
        (
            cell.get("sum").unwrap() as u16,
            cell.get("carry").unwrap() as u16,
        )
    }

    // 1234 + 5678 = 6912: units 4+8=12>9 -> digit 2 carry 1; tens 3+7+1=11>9 -> digit 1
    // carry 1; hundreds 2+6+1=9 -> digit 9 carry 0; thousands 1+5+0=6 -> digit 6. No carry-out.
    assert_eq!(add16(0x1234, 0x5678), (0x6912, 0));

    // 9999 + 1 = 10000 -> wraps mod 10000 to 0, carry out 1: every nibble sum is 9+1(+carry)=10,
    // rippling all the way through the thousands digit.
    assert_eq!(add16(0x9999, 0x0001), (0x0000, 1));

    // 0 + 0 = 0, no carry: trivial baseline.
    assert_eq!(add16(0x0000, 0x0000), (0x0000, 0));

    // 4999 + 5001 = 10000 -> wraps to 0, carry out 1: carry ripples through three nibbles,
    // then the top nibble sum 4+5+1=10 also decimal-corrects to 0 with carry out.
    assert_eq!(add16(0x4999, 0x5001), (0x0000, 1));

    // 1099 + 1 = 1100, no carry out: carry ripples through units and tens, then stops
    // cleanly at the hundreds digit (0+0+1=1, no overflow) -- isolates a partial ripple.
    assert_eq!(add16(0x1099, 0x0001), (0x1100, 0));
}

// Verifies bcd_sub16 subtracts two packed 4-digit BCD u16 values via a 4-nibble decimal-borrow
// chain, mirroring bcd_sub's 2-nibble form one rung wider on the pack's 2-digit/4-digit ladder.
// Cases hand-computed: a generic no-borrow subtraction, a full borrow-chain propagation across
// all four digits, the zero-minus-zero identity, a max-minus-small edge with no borrow, and a
// zero-minus-one wraparound (mod 10000) that forces borrow-out through every nibble.
#[test]
fn bcd_sub16_matches_defined_behaviour() {
    fn sub16(a: u16, b: u16) -> (u16, u16) {
        let mut cell = StateCell::bind(&cell_src("bcd_sub16"), "BcdSub16", None)
            .unwrap_or_else(|e| panic!("bind bcd_sub16: {e}"));
        cell.set("a", a as u64).unwrap();
        cell.set("b", b as u64).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.result, 1, "status flag should be 1");
        (
            cell.get("diff").unwrap() as u16,
            cell.get("borrow").unwrap() as u16,
        )
    }

    assert_eq!(sub16(0x5000, 0x3000), (0x2000, 0)); // 5000 - 3000 = 2000, no borrow
    assert_eq!(sub16(0x1234, 0x5678), (0x5556, 1)); // 1234 - 5678 = -4444 mod 10000 = 5556, borrow
    assert_eq!(sub16(0x0000, 0x0000), (0x0000, 0)); // 0 - 0 = 0
    assert_eq!(sub16(0x9999, 0x0001), (0x9998, 0)); // 9999 - 1 = 9998, no borrow
    assert_eq!(sub16(0x0000, 0x0001), (0x9999, 1)); // 0 - 1 wraps to 9999, borrow-out
}
