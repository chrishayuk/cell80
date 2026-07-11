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


#[test]
fn bit_length_u32_wide_sibling_matches_hand_computed_cases() {
    // bit_length_u32: the u32-width sibling of bit_length -- index of the highest set
    // bit + 1 (0 for x == 0), via a u32 state field since bit_length's fn run(x: u16)
    // can't accept a 32-bit input under the 16-bit calling convention. Cases hand-computed:
    // zero, the smallest nonzero value, the top of the u16 domain (cross-checked against
    // bit_length(65535)=16), one bit past the u16 domain (bit_length can't even express
    // this as input), the top bit of a full u32, and all 32 bits set.
    fn bit_length_u32(x: u32) -> u16 {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("bit_length_u32"),
            "BitLengthU32",
            None,
        )
        .unwrap_or_else(|e| panic!("bind bit_length_u32: {e}"));
        cell.set("x", x as u64).unwrap();
        cell.run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run bit_length_u32: {e}"));
        cell.get("out").unwrap_or_else(|| panic!("no out field")) as u16
    }

    // x=0 -> no bits set -> 0.
    assert_eq!(bit_length_u32(0), 0);
    // x=1 -> highest set bit at index 0 -> 1.
    assert_eq!(bit_length_u32(1), 1);
    // x=0xFFFF (fits u16 domain, top bit at index 15) -> 16, matches bit_length(65535)=16.
    assert_eq!(bit_length_u32(0xFFFF), 16);
    // x=0x00010000 (bit 16 set, one past what bit_length can even take as input) -> 17.
    assert_eq!(bit_length_u32(0x0001_0000), 17);
    // x=0x80000000 (bit 31, the top bit of a full u32) -> 32.
    assert_eq!(bit_length_u32(0x8000_0000), 32);
    // x=0xFFFFFFFF (all 32 bits set) -> 32.
    assert_eq!(bit_length_u32(0xFFFF_FFFF), 32);
}

// leading_zeros_u32: the u32-width sibling of leading_zeros -- counts leading (high) zero
// bits across a full 32-bit value via a u32 state field, since leading_zeros's fn run(x: u16)
// cannot represent inputs with bits above position 15 at all. Cases hand-computed: zero (full
// 32-bit run of zeros), all-32-bits-set and the lone MSB (bit 31) both giving 0 leading zeros,
// a lone bit above the u16 domain (bit 16) which is exactly the case leading_zeros cannot even
// express as input, and the all-u16-domain-bits-set case cross-checked against the 16-bit
// leading_zeros(0xFFFF)==0 to confirm the widening actually matters (same bit pattern reads as
// "no leading zeros" at 16 bits but "16 leading zeros" once widened to 32).
#[test]
fn leading_zeros_u32_wide_sibling_matches_hand_computed_cases() {
    fn step(x: u64) -> u64 {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("leading_zeros_u32"),
            "LeadingZerosU32",
            None,
        )
        .unwrap_or_else(|e| panic!("bind leading_zeros_u32: {e}"));
        cell.set("x", x).unwrap();
        cell.run(cell80::DEFAULT_CYCLES).unwrap();
        cell.get("out").unwrap_or_else(|| panic!("no out field"))
    }

    // 1) x=0 -> no bits at all -> full 32-bit run of zeros -> 32.
    assert_eq!(step(0), 32);

    // 2) x=0xFFFFFFFF -> MSB (bit 31) already set -> 0 leading zeros.
    assert_eq!(step(0xFFFF_FFFF), 0);

    // 3) x=0x80000000 -> bit 31 only -> MSB set -> 0 leading zeros.
    assert_eq!(step(0x8000_0000), 0);

    // 4) x=0x00010000 -> highest set bit is bit 16 -> leading zeros = 31 - 16 = 15.
    //    This bit sits above the u16 domain entirely, so it's a case leading_zeros
    //    (u16-only) cannot even represent as input.
    assert_eq!(step(0x0001_0000), 15);

    // 5) x=0x0000FFFF -> highest set bit is bit 15 -> leading zeros = 31 - 15 = 16.
    //    Cross-checks against the 16-bit leading_zeros(0xFFFF) == 0: the same bit
    //    pattern reads as "no leading zeros" in the 16-bit domain but "16 leading
    //    zeros" once widened to 32 bits, confirming the widening actually matters.
    assert_eq!(step(0x0000_FFFF), 16);
}

#[test]
fn trailing_zeros_u32_matches_hand_computed_expectations() {
    // trailing_zeros_u32: the u32-width sibling of trailing_zeros -- counts trailing zero
    // bits across a full 32-bit value via a u32 state field, since trailing_zeros's
    // fn run(x: u16) cannot represent inputs with bits above position 15 at all.
    fn step(x: u64) -> u16 {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("trailing_zeros_u32"),
            "TrailingZerosU32",
            None,
        )
        .unwrap_or_else(|e| panic!("bind trailing_zeros_u32: {e}"));
        cell.set("x", x).unwrap();
        cell.run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run trailing_zeros_u32: {e}"));
        cell.get("out").unwrap_or_else(|| panic!("no out field")) as u16
    }

    // 1) x=0 -> no set bits at all -> 32 (full-width sentinel).
    assert_eq!(step(0), 32);

    // 2) x=1 (0b...0001) -> bit 0 set -> 0.
    assert_eq!(step(1), 0);

    // 3) x=8 (0b1000) -> bits 0-2 zero, bit 3 set -> 3. Cross-checks trailing_zeros(8)=3.
    assert_eq!(step(8), 3);

    // 4) x=0x80000000 (bit 31 only) -> 31 zero bits below it. Proves wide bits above the
    //    u16 domain are counted (trailing_zeros can't even represent this input).
    assert_eq!(step(0x8000_0000), 31);

    // 5) x=0x00010000 (bit 16 only, the low/high u16-word boundary) -> 16.
    assert_eq!(step(0x0001_0000), 16);

    // 6) x=0xFFFFFFFF (all bits set) -> bit 0 already set -> 0.
    assert_eq!(step(0xFFFF_FFFF), 0);
}

// reverse_bits_u32: full-width sibling of reverse_bits, reversing all 32 bits of a u32
// (bit 0 <-> bit 31, bit 1 <-> bit 30, ...) via a state cell since u32 cannot be a free-fn
// parameter under the 16-bit calling convention. Cases hand-computed: zero and all-ones are
// fixed points, single-low-bit and single-bit-just-above-it move to their mirrored high
// positions, and the generic mixed case is hand-derived by reversing each byte's bits
// individually then reversing byte order (0x12,0x34,0x56,0x78 -> reverse_bits8 each ->
// 0x48,0x2C,0x6A,0x1E -> reorder -> 0x1E6A2C48), the same cross-check technique crc32_step's
// test uses against its byte-level siblings.
#[test]
fn reverse_bits_u32_matches_hand_computed_cases() {
    fn reverse(x: u32) -> u32 {
        let mut cell = cell80::StateCell::bind(&crate::common::cell_src("reverse_bits_u32"), "ReverseBitsU32", None)
            .unwrap_or_else(|e| panic!("bind reverse_bits_u32: {e}"));
        cell.set("x", x as u64).unwrap();
        let report = cell.run(cell80::DEFAULT_CYCLES).unwrap();
        assert_eq!(report.result, 1, "status flag should be 1");
        cell.get("out").unwrap_or_else(|| panic!("no out field")) as u32
    }

    assert_eq!(reverse(0), 0); // all-zero is a fixed point
    assert_eq!(reverse(1), 0x80000000); // bit0 set moves to bit31
    assert_eq!(reverse(0xFFFFFFFF), 0xFFFFFFFF); // all-ones is a fixed point
    assert_eq!(reverse(0x00000002), 0x40000000); // bit1 set moves to bit30
    assert_eq!(reverse(0x12345678), 0x1E6A2C48); // generic value spanning both halves
}

// swap_bytes_u32: the u32-width sibling of swap_bytes -- endian-swaps all four bytes of a
// full 32-bit value via a u32 state field, since swap_bytes's fn run(x: u16) can only swap
// two bytes. Cases hand-computed: all-zero and all-ones fixed points, a generic four-distinct-
// byte value, and a boundary case where only the lowest byte is set (must land in the highest
// byte position after the swap).
#[test]
fn swap_bytes_u32_matches_hand_computed_cases() {
    fn swap(x: u32) -> u32 {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("swap_bytes_u32"),
            "SwapBytesU32",
            None,
        )
        .unwrap_or_else(|e| panic!("bind swap_bytes_u32: {e}"));
        cell.set("x", x as u64).unwrap();
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run swap_bytes_u32: {e}"));
        assert_eq!(report.result, 1, "status flag should be 1");
        cell.get("out").unwrap_or_else(|| panic!("no out field")) as u32
    }

    // 1) all-zero is a fixed point.
    assert_eq!(swap(0), 0);
    // 2) all-ones is a fixed point.
    assert_eq!(swap(0xFFFFFFFF), 0xFFFFFFFF);
    // 3) bytes (hi->lo) 12,34,56,78 reversed to 78,56,34,12 -> 0x78563412.
    assert_eq!(swap(0x12345678), 0x78563412);
    // 4) only the lowest byte set -> moves to the highest byte position.
    assert_eq!(swap(0x000000FF), 0xFF000000);
    // 5) four distinct bytes 01,02,03,04 -> reversed order 04,03,02,01 -> 0x04030201.
    assert_eq!(swap(0x01020304), 0x04030201);
}

#[test]
fn rotl32_matches_hand_computed_cases() {
    // rotl32: the u32-width sibling of rotl16 -- rotates all 32 bits of x left by n (n mod
    // 32). Needs a state cell (x/out are u32) since u32 cannot be a free-fn parameter under
    // the 16-bit calling convention, and is built from a bounded loop of single-bit
    // rotations because this dialect's u32 shifts accept only constant-literal amounts (no
    // single variable-shift expression like rotl16's is possible at 32 bits). Requires
    // cell80::StateCell / cell80::DEFAULT_CYCLES fully-qualified and crate::common::cell_src
    // (mirrors bit-mask.rs's popcount_u32 test note -- no new use needed at the top of this
    // file).
    fn rotl32(x: u32, n: u16) -> u32 {
        let mut cell = cell80::StateCell::bind(&crate::common::cell_src("rotl32"), "Rotl32", None)
            .unwrap_or_else(|e| panic!("bind rotl32: {e}"));
        cell.set("x", x as u64).unwrap();
        cell.set("n", n as u64).unwrap();
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run rotl32: {e}"));
        assert_eq!(report.halt, cell80::Halt::Returned);
        cell.get("out").unwrap_or_else(|| panic!("no out field")) as u32
    }

    // 1) 0x00000001 rotl 1 = 0x00000002 (bit 0 moves to bit 1).
    assert_eq!(rotl32(1, 1), 2, "case 1: 1 rotl 1");

    // 2) 0x80000000 rotl 1 = 0x00000001 (top bit wraps around to bit 0).
    assert_eq!(rotl32(0x80000000, 1), 1, "case 2: top-bit wraparound");

    // 3) 0x12345678 rotl 8 = 0x34567812 (whole-byte rotate: bytes shift up one position,
    //    the top byte 0x12 wraps around to the bottom).
    assert_eq!(rotl32(0x12345678, 8), 0x34567812, "case 3: byte rotate");

    // 4) 0xFFFFFFFF rotl 17 = 0xFFFFFFFF (all bits set is invariant under any rotation).
    assert_eq!(rotl32(0xFFFFFFFF, 17), 0xFFFFFFFF, "case 4: all-ones invariant");

    // 5) 0x00000001 rotl 33 = 0x00000002 (n=33 mod 32 = 1, same as case 1 -- proves the
    //    n-mod-32 wraparound for n >= 32, not just n < 32 inputs).
    assert_eq!(rotl32(1, 33), 2, "case 5: n mod 32 wraparound");

    // 6) 0x0000FFFF rotl 16 = 0xFFFF0000 (half-word swap).
    assert_eq!(rotl32(0x0000FFFF, 16), 0xFFFF0000, "case 6: half swap");

    // 7) x rotl 32 = x unchanged (n=32 mod 32 = 0, identity rotation).
    assert_eq!(rotl32(0x12345678, 32), 0x12345678, "case 7: n=32 is identity");
}

#[test]
fn rotr32_matches_hand_computed_cases() {
    // rotr32: the u32-width sibling of rotr16 -- rotates a full 32-bit value right by
    // n (n mod 32) using a state cell, since the 16-bit calling convention has no u32
    // free-fn parameters. Internally it loops a single-bit rotate (literal shifts by
    // 1 and 31) `s` times, because this dialect only allows *constant* shift amounts
    // on u32 values (no runtime-amount 32-bit shift). Cases hand-computed against the
    // standard rotate-right definition (x >> s) | (x << (32 - s)).
    fn rotr32(x: u32, n: u16) -> u32 {
        let mut cell = cell80::StateCell::bind(&crate::common::cell_src("rotr32"), "Rotr32", None)
            .unwrap_or_else(|e| panic!("bind rotr32: {e}"));
        cell.set("x", x as u64).unwrap();
        cell.set("n", n as u64).unwrap();
        let report = cell.run(cell80::DEFAULT_CYCLES).unwrap();
        assert_eq!(report.result, 1, "status flag should be 1");
        cell.get("out").unwrap_or_else(|| panic!("no out field")) as u32
    }

    // 1) lowest bit set, rotate right by 1: bit 0 wraps around to bit 31.
    assert_eq!(rotr32(0x00000001, 1), 0x80000000);
    // 2) highest bit set, rotate right by 1: bit 31 moves down to bit 30.
    assert_eq!(rotr32(0x80000000, 1), 0x40000000);
    // 3) rotate right by 4 = rotate right by one hex nibble: the low nibble (8) wraps
    //    to become the new top nibble, everything else shifts down by one nibble.
    assert_eq!(rotr32(0x12345678, 4), 0x81234567);
    // 4) all bits set is a fixed point under rotation, regardless of n.
    assert_eq!(rotr32(0xFFFFFFFF, 17), 0xFFFFFFFF);
    // 5) n=32 reduces to n mod 32 == 0, so the value passes through unchanged.
    assert_eq!(rotr32(0x00000001, 32), 0x00000001);
}
