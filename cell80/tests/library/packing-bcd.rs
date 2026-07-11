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
        ("bcd_encode16", &[0], 0x0000),       // all digits zero
        ("bcd_encode16", &[9999], 0x9999),    // max four-digit value, no nibble overflow
        ("bcd_encode16", &[1234], 0x1234),    // generic distinct digits
        ("bcd_encode16", &[42], 0x0042),      // leading zero digits (thousands, hundreds)
        ("bcd_encode16", &[8005], 0x8005),    // internal zero digits (hundreds, tens)
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
