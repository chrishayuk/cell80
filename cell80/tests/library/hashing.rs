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


// fnv1a32_step: one true FNV-1a hash step over a byte on a full u32 accumulator, using the
// real FNV-1a-32 constants (prime 16777619, conventional offset basis 2166136261) --
// fnv1a_step's u16-only sibling widened to the canonical 32-bit definition. A state cell
// since hash/out are u32 and u32 cannot be a free-fn parameter under the 16-bit calling
// convention (mirrors crc32_step's exact shape). Requires `use cell80::{StateCell,
// DEFAULT_CYCLES};` and `crate::common::cell_src` at the top of this pack test file.
#[test]
fn fnv1a32_step_matches_hand_computed_fnv1a32() {
    fn step(hash: u32, byte: u16) -> u32 {
        let mut cell = StateCell::bind(&cell_src("fnv1a32_step"), "Fnv1a32Step", None)
            .unwrap_or_else(|e| panic!("bind fnv1a32_step: {e}"));
        cell.set("hash", hash as u64).unwrap();
        cell.set("byte", byte as u64).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.halt, cell80::Halt::Returned);
        assert_eq!(report.result, 1u16, "ack should be 1");
        cell.get("out").unwrap_or_else(|| panic!("no out field")) as u32
    }

    // Hand-computed independently from the FNV-1a definition h = (h ^ byte) * 16777619
    // mod 2^32 using the real FNV-1a-32 constants (not from the compiled cell's own output).
    let cases: &[(u32, u16, u32)] = &[
        (0, 0, 0x00000000),             // zero hash, zero byte -> stays zero
        (2166136261, 97, 0xE40C292C),   // offset basis ^ 'a' -- matches published FNV-1a32("a")
        (0xFFFFFFFF, 0, 0xFEFFFE6D),    // all-ones hash, zero byte
        (0x12345678, 0x56, 0xD663AA6A), // generic mixed hash + byte
        (0, 0x1FF, 0xFF01916D),         // byte masked to 0xFF: same as byte=0xFF
    ];
    for (hash, byte, exp) in cases {
        let got = step(*hash, *byte);
        assert_eq!(
            got, *exp,
            "fnv1a32_step({hash:#010x}, {byte:#06x}) = {got:#010x}, expected {exp:#010x}"
        );
    }

    // Full-message check: "foobar" fed byte-by-byte through fnv1a32_step starting from the
    // canonical offset basis reproduces the well-known FNV-1a-32 test vector 0xBF9CF968.
    let mut hash = 2166136261u32;
    for b in b"foobar" {
        hash = step(hash, *b as u16);
    }
    assert_eq!(hash, 0xBF9CF968, "\"foobar\" -> {hash:#010x}, expected 0xbf9cf968");
}

#[test]
fn hash5_matches_hand_computed_values() {
    // hash5 extends hash4's multiply-xor-multiply chain by one more term/prime
    // (0x27D4 after 0x9E37, 0x85EB, 0xEB2F, 0xC2B2), then applies the same final
    // avalanche shift-xor. It is a state cell (5 inputs exceeds the 3-arg free-fn
    // limit), so it is exercised via StateCell::bind/set/run/get rather than
    // run_cell. Values below were hand-derived from the exact wrapping-mul/xor
    // chain in Python (16-bit wrapping mul, masked to 0xFFFF at each step) before
    // being locked in as the oracle.
    fn step(a: u16, b: u16, c: u16, d: u16, e: u16) -> (u16, u16) {
        let mut cell = cell80::StateCell::bind(&crate::common::cell_src("hash5"), "Hash5", None)
            .unwrap_or_else(|err| panic!("bind: {err}"));
        cell.set("a", a as u64).unwrap();
        cell.set("b", b as u64).unwrap();
        cell.set("c", c as u64).unwrap();
        cell.set("d", d as u64).unwrap();
        cell.set("e", e as u64).unwrap();
        let report = cell.run(cell80::DEFAULT_CYCLES).unwrap();
        (report.result, cell.get("out").unwrap() as u16)
    }

    let cases: &[(u16, u16, u16, u16, u16, u16)] = &[
        (0, 0, 0, 0, 0, 0),
        (1, 2, 3, 4, 5, 33253),
        (0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 33991),
        (100, 200, 300, 400, 500, 53942),
        (1, 0, 0, 0, 0, 52259),
    ];

    let mut failures = Vec::new();
    for (a, b, c, d, e, exp) in cases {
        let (result, out) = step(*a, *b, *c, *d, *e);
        if result != *exp || out != *exp {
            failures.push(format!(
                "hash5({a},{b},{c},{d},{e}) = (result={result}, out={out}), expected {exp}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "cell mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn hash_pair32_matches_hand_computed_values() {
    // hash_pair32 is the u32-domain analogue of hash_pair's multiply-xor-multiply chain:
    // h = a * 0x9E3779B9 (the 32-bit golden-ratio constant hash_pair's own 0x9E37 is the
    // top half of); h = (h ^ b) * 0x85EBCA6B (mod 2^32, the same constant mix32 already
    // established as this pack's u32-domain avalanche prime); h ^= h >> 7 (same shift
    // distance as hash_pair's own final avalanche step). Since out is u16 but h is u32,
    // the result is folded (lo ^ hi), mix32's own move, rather than truncated, so bits
    // from the whole 32-bit state still reach the output. Values were hand-derived by
    // tracing this exact chain in Python before being locked in as the oracle.
    fn hash_pair32(a: u32, b: u32) -> u16 {
        let mut cell = StateCell::bind(&cell_src("hash_pair32"), "HashPair32", None)
            .unwrap_or_else(|e| panic!("bind hash_pair32: {e}"));
        cell.set("a", a as u64).unwrap();
        cell.set("b", b as u64).unwrap();
        let report = cell
            .run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run hash_pair32: {e}"));
        assert_eq!(report.halt, cell80::Halt::Returned);
        let out = cell.get("out").unwrap_or_else(|| panic!("no out field")) as u16;
        assert_eq!(report.result, out, "return value should match out field");
        out
    }

    let cases: &[(u32, u32, u16)] = &[
        (0, 0, 0),
        (1, 0, 1899),
        (0, 1, 39199),
        (0x12345678, 0x9ABCDEF0, 39994),
        (0x10000, 0, 32231), // high-word-only a: proves the full 32 bits of a are mixed, not truncated
    ];

    let mut failures = Vec::new();
    for (a, b, exp) in cases {
        let got = hash_pair32(*a, *b);
        if got != *exp {
            failures.push(format!("hash_pair32({a:#x},{b:#x}) = {got}, expected {exp}"));
        }
    }
    assert!(failures.is_empty(), "cell mismatches:\n{}", failures.join("\n"));
}

// adler32_step: one Adler-32 checksum step over a byte (s1=(s1+byte)%65521,
// s2=(s2+s1)%65521, packed as checksum=(s2<<16)|s1) -- a two-running-sums-mod-a-prime
// checksum, a different algorithm family from the crc*_step shift-xor reflected-
// polynomial line. State cell (checksum/out are u32), so it's exercised via
// StateCell::bind/set/run/get like crc32_step's own test just above/below it.
#[test]
fn adler32_step_matches_hand_computed_values() {
    fn step(checksum: u32, byte: u16) -> u32 {
        let mut cell = StateCell::bind(&cell_src("adler32_step"), "Adler32Step", None)
            .unwrap_or_else(|e| panic!("bind adler32_step: {e}"));
        cell.set("checksum", checksum as u64).unwrap();
        cell.set("byte", byte as u64).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.halt, cell80::Halt::Returned);
        cell.get("out").unwrap_or_else(|| panic!("no out field")) as u32
    }

    // Hand-computed by unpacking s1 = checksum & 0xFFFF, s2 = checksum >> 16, then
    // s1n = (s1+byte) % 65521; s2n = (s2+s1n) % 65521; out = (s2n<<16)|s1n.
    let cases: &[(u32, u16, u32)] = &[
        (0, 0, 0),                 // zero state, zero byte -> stays zero
        (1, 0, 0x00010001),        // conventional adler32 init (s1=1,s2=0) stepped over 0
        (1, 97, 0x00620062),       // init state stepped over 'a' (97): s1=98, s2=98
        (65520, 5, 0x00040004),    // s1 wraps past the 65521 modulus back down to 4
        (1, 256, 0x00010001),      // byte masked to & 0xFF -> identical to byte=0
    ];
    for (checksum, byte, exp) in cases {
        let got = step(*checksum, *byte);
        assert_eq!(
            got, *exp,
            "adler32_step({checksum:#010x}, {byte:#06x}) = {got:#010x}, expected {exp:#010x}"
        );
    }

    // Full-message check: "abc" through adler32_step from the conventional initial
    // state checksum=1 (s1=1, s2=0) reproduces the standard Adler-32 test vector
    // 0x024D0127, the same "feed bytes through the step fn" convention crc16_step's
    // and crc32_step's own tests use for their known-vector checks.
    let mut checksum = 1u32;
    for b in b"abc" {
        checksum = step(checksum, *b as u16);
    }
    assert_eq!(
        checksum, 0x024D0127,
        "\"abc\" -> {checksum:#010x}, expected 0x024d0127"
    );
}

#[test]
fn hash_pair_sym_matches_defined_behaviour() {
    // hash_pair_sym mixes the *commutative* sum and product of (a, b) through an avalanche
    // chain, so unlike hash_pair (order-sensitive left-to-right fold), swapping the arguments
    // must never change the result. Values hand-derived from the exact algorithm: sum =
    // a.wrapping_add(b), prod = (a as u32) * (b as u32) (always fits u32 since a, b <= u16::MAX),
    // h = sum.wrapping_mul(0x9E3779B9) ^ prod, two xor-shift-13/multiply avalanche rounds
    // (0x85EBCA6B, then 0xC2B2AE35), a final xor-shift-16, then fold lo ^ hi.
    let cases: &[(&str, &[u16], u16)] = &[
        ("hash_pair_sym", &[0, 0], 0),
        ("hash_pair_sym", &[1, 2], 11305),
        ("hash_pair_sym", &[5, 5], 36739),
        ("hash_pair_sym", &[3, 7], 26626),
        ("hash_pair_sym", &[65535, 65535], 28981),
        ("hash_pair_sym", &[100, 200], 64984),
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

    // The defining property: order must never matter, checked directly against the same
    // running cell rather than baked into a fixed table -- covers a==b and the all-bits pairs.
    let symmetric_pairs: &[(u16, u16)] = &[
        (1, 2),
        (3, 7),
        (0, 65535),
        (12345, 54321),
        (5, 5),
        (65535, 65535),
    ];
    for (a, b) in symmetric_pairs {
        let forward = run_cell("hash_pair_sym", &[*a, *b]);
        let backward = run_cell("hash_pair_sym", &[*b, *a]);
        assert_eq!(
            forward, backward,
            "hash_pair_sym({a}, {b}) = {forward} but hash_pair_sym({b}, {a}) = {backward}"
        );
    }
}

#[test]
fn hash_slide_step_matches_hand_computed_values() {
    // hash_slide_step is the Rabin-Karp incremental window slide: hash' = (hash -
    // old_byte*high_pow) * 257 + new_byte, all wrapping u16. It is a state cell (4 inputs
    // exceeds the 3-arg free-fn limit), so it is exercised via StateCell::bind/set/run/get.
    fn step(hash: u16, old_byte: u16, new_byte: u16, high_pow: u16) -> u16 {
        let mut cell = StateCell::bind(&cell_src("hash_slide_step"), "HashSlideStep", None)
            .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("hash", hash as u64).unwrap();
        cell.set("old_byte", old_byte as u64).unwrap();
        cell.set("new_byte", new_byte as u64).unwrap();
        cell.set("high_pow", high_pow as u64).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap_or_else(|e| panic!("run: {e}"));
        assert_eq!(report.halt, cell80::Halt::Returned);
        cell.get("out").unwrap() as u16
    }

    let cases: &[(u16, u16, u16, u16, u16)] = &[
        // all-zero: nothing to remove, nothing added.
        (0, 0, 0, 0, 0),
        // no removal (high_pow=0), just (hash*257 + new_byte).
        (100, 0, 5, 0, 25705),
        // real rolling-hash check: window "ab" (hash=97*257+98=25027) slides to "bc"
        // with high_pow=257^(len-1)=257 for a 2-byte window; result must equal a
        // from-scratch hash of "bc" = 98*257+99 = 25285.
        (25027, 97, 99, 257, 25285),
        // deliberate underflow: old_byte*high_pow (500) exceeds hash (10), so the
        // subtraction wraps mod 65536 before the multiply/add continue.
        (10, 5, 7, 100, 5149),
        // near-max stress case, wraps through the multiply too.
        (65535, 255, 255, 255, 253),
    ];

    let mut failures = Vec::new();
    for (hash, old_byte, new_byte, high_pow, exp) in cases {
        let got = step(*hash, *old_byte, *new_byte, *high_pow);
        if got != *exp {
            failures.push(format!(
                "hash_slide_step(hash={hash}, old_byte={old_byte}, new_byte={new_byte}, high_pow={high_pow}) = {got}, expected {exp}"
            ));
        }
    }
    assert!(failures.is_empty(), "cell mismatches:\n{}", failures.join("\n"));
}
