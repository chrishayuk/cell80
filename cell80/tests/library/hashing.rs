//! Host-oracle tests for the hashing pack (`cell80/cells/hashing/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::{cell_src, run_cell};
use cell80::{StateCell, DEFAULT_CYCLES};

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

// crc32_step: one CRC-32 (CRC-32/ISO-HDLC, poly 0xEDB88320 reflected) shift-xor step over a
// byte on a full u32 accumulator -- crc8_step/crc16_step's loop widened one more rung, needing
// a state cell (crc/out are u32) since u32 cannot be a free-fn parameter under the 16-bit
// calling convention. Requires `use cell80::{StateCell, DEFAULT_CYCLES};` and
// `crate::common::cell_src` at the top of this pack test file (not yet imported there since
// hashing previously had only free-fn cells), mirroring packing-bcd.rs's unpack_u16_pair note.
#[test]
fn crc32_step_matches_hand_computed_crc32_iso_hdlc() {
    fn step(crc: u32, byte: u16) -> u32 {
        let mut cell = StateCell::bind(&cell_src("crc32_step"), "Crc32Step", None)
            .unwrap_or_else(|e| panic!("bind crc32_step: {e}"));
        cell.set("crc", crc as u64).unwrap();
        cell.set("byte", byte as u64).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.halt, cell80::Halt::Returned);
        cell.get("out").unwrap_or_else(|| panic!("no out field")) as u32
    }

    // Hand-computed by tracing the shift-xor loop bit-by-bit against the standard
    // CRC-32/ISO-HDLC polynomial 0xEDB88320 (reflected):
    //   c = crc ^ (byte & 0xFF); repeat 8x: c = (c>>1)^0xEDB88320 if (c&1)!=0 else c>>1
    let cases: &[(u32, u16, u32)] = &[
        (0, 0, 0x00000000),             // zero crc, zero byte -> stays zero
        (0, 1, 0x77073096),             // classic single-bit-input CRC-32 constant
        (0xFFFFFFFF, 0, 0x2DFD1072),    // all-ones crc, zero byte
        (0x12345678, 0x56, 0xDCC43999), // generic mixed crc + byte
        (0, 0xFF, 0x2D02EF8D),          // zero crc, all-ones byte
    ];
    for (crc, byte, exp) in cases {
        let got = step(*crc, *byte);
        assert_eq!(
            got, *exp,
            "crc32_step({crc:#010x}, {byte:#04x}) = {got:#010x}, expected {exp:#010x}"
        );
    }

    // Full-message check: "123456789" through crc32_step from crc=0xFFFFFFFF, with the
    // conventional final crc ^ 0xFFFFFFFF, reproduces the standard CRC-32/ISO-HDLC (zlib/
    // Ethernet/PNG/zip "CRC-32") check value 0xCBF43926 -- the same convention crc16_step's
    // own test uses for its "123456789" -> 0xBB3D vector, one rung down.
    let mut crc = 0xFFFFFFFFu32;
    for b in b"123456789" {
        crc = step(crc, *b as u16);
    }
    crc ^= 0xFFFFFFFF;
    assert_eq!(
        crc, 0xCBF43926,
        "\"123456789\" -> {crc:#010x}, expected 0xcbf43926"
    );
}

// mix32: full-width avalanche finalizer over a u32 (murmur3-style fmix32 chain, then
// fold hi/lo halves into u16). Values hand-derived by tracing h ^= h>>16; h *= 0x85EBCA6B;
// h ^= h>>13; h *= 0xC2B2AE35; h ^= h>>16; out = (h & 0xFFFF) ^ (h >> 16), in 32-bit
// modular arithmetic. Case 4 (0x10000) is the load-bearing one: its low 16 bits are all
// zero, so a cell that (incorrectly) truncated the input to u16 before mixing would
// collapse it to the same output as x=0 -- this proves the full 32 bits are used.
#[test]
fn mix32_matches_hand_computed_cases() {
    fn mix32(x: u32) -> u16 {
        let mut cell = StateCell::bind(&cell_src("mix32"), "Mix32", None)
            .unwrap_or_else(|e| panic!("bind mix32: {e}"));
        cell.set("x", x as u64).unwrap();
        cell.run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run mix32: {e}"));
        cell.get("out").unwrap() as u16
    }

    assert_eq!(mix32(0), 0); // all-zero state is a fixed point of the whole chain
    assert_eq!(mix32(1), 31225); // minimal nonzero input still avalanches fully
    assert_eq!(mix32(0xFFFFFFFF), 61128); // all-ones input
    assert_eq!(mix32(0x10000), 19393); // high-word-only: proves full 32 bits are mixed
    assert_eq!(mix32(0x12345678), 12992); // bits spanning both halves
}

#[test]
fn hash4_matches_hand_computed_values() {
    // hash4 extends hash3's multiply-xor-multiply chain by one more term/prime
    // (0xC2B2 after 0x9E37, 0x85EB, 0xEB2F), then applies the same final avalanche
    // shift-xor. It is a state cell (4 inputs exceeds the 3-arg free-fn limit), so it
    // is exercised via StateCell::bind/set/run/get rather than run_cell. Values below
    // were hand-derived from the exact wrapping-mul/xor chain in Python before being
    // locked in as the oracle.
    fn step(a: u16, b: u16, c: u16, d: u16) -> (u16, u16) {
        let mut cell = cell80::StateCell::bind(&crate::common::cell_src("hash4"), "Hash4", None)
            .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("a", a as u64).unwrap();
        cell.set("b", b as u64).unwrap();
        cell.set("c", c as u64).unwrap();
        cell.set("d", d as u64).unwrap();
        let report = cell.run(cell80::DEFAULT_CYCLES).unwrap();
        (report.result, cell.get("out").unwrap() as u16)
    }

    let cases: &[(u16, u16, u16, u16, u16)] = &[
        (0, 0, 0, 0, 0),
        (1, 2, 3, 4, 65357),
        (0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 60142),
        (1, 0, 0, 0, 39875),
        (100, 200, 300, 400, 37861),
    ];

    let mut failures = Vec::new();
    for (a, b, c, d, exp) in cases {
        let (result, out) = step(*a, *b, *c, *d);
        if result != *exp || out != *exp {
            failures.push(format!(
                "hash4({a},{b},{c},{d}) = (result={result}, out={out}), expected {exp}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "cell mismatches:\n{}",
        failures.join("\n")
    );
}
