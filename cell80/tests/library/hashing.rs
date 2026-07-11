//! Host-oracle tests for the hashing pack (`cell80/cells/hashing/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::run_cell;

#[test]
fn first_wave_hashing_cells_match_defined_behaviour() {
    let cases: &[(&str, &[u16], u16)] = &[
        ("hash_pair", &[1, 2], 49696),
        ("hash_pair", &[0, 0], 0),
        ("fnv1a_step", &[0, 65], 26195),
        ("fnv1a_step", &[0, 256], 0), // byte masked to 0xFF, so == (0, 0)
        ("crc8_step", &[0, 0], 0),
        ("crc8_step", &[0, 1], 94),
        ("mix16", &[0], 0),
        ("mix16", &[1], 10688),
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
fn hash3_matches_defined_behaviour() {
    // hash3 extends hash_pair's multiply-xor-multiply chain by one more term/prime
    // (0xEB2F after 0x9E37 and 0x85EB), then applies the same final avalanche shift-xor.
    // Values below were hand-derived from the exact wrapping-mul/xor chain and
    // cross-checked against hash_pair's own known vectors (hash_pair(1,2) == 49696)
    // before being locked in as the oracle.
    let cases: &[(&str, &[u16], u16)] = &[
        ("hash3", &[0, 0, 0], 0),
        ("hash3", &[1, 2, 3], 30706),
        ("hash3", &[5, 5, 5], 47465),
        ("hash3", &[1, 0, 0], 55134),
        ("hash3", &[0, 0, 1], 60153),
        ("hash3", &[65535, 65535, 65535], 35865),
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
fn crc16_step_matches_hand_computed_crc16_arc() {
    // CRC-16/ARC (poly 0xA001 reflected) single-step values, hand-computed by tracing
    // the shift-xor loop bit-by-bit, plus the classic "123456789" -> 0xBB3D full-message
    // vector fed byte-by-byte through the step function starting from crc=0.
    let cases: &[(&str, &[u16], u16)] = &[
        ("crc16_step", &[0, 0], 0x0000),
        ("crc16_step", &[0, 1], 0xC0C1),
        ("crc16_step", &[0xFFFF, 0], 0x40BF),
        ("crc16_step", &[0x1234, 0x56], 0xE993),
        ("crc16_step", &[0, 0xFF], 0x4040),
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

    // Full-message check: "123456789" through crc16_step from crc=0 reproduces the
    // standard CRC-16/ARC test vector 0xBB3D.
    let mut crc = 0u16;
    for b in b"123456789" {
        crc = run_cell("crc16_step", &[crc, *b as u16]);
    }
    assert_eq!(crc, 0xBB3D, "\"123456789\" -> {crc:#06x}, expected 0xbb3d");
}
