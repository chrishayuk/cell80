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

#[test]
fn lowest_set_bit_matches_hand_computed_cases() {
    // lowest_set_bit isolates just the value of x's least-significant set bit via
    // x & (0 - x); this differs from bit_is_set (needs a named bit index) and from
    // popcount (returns a count, not a value).
    let cases: &[(&str, &[u16], u16)] = &[
        ("lowest_set_bit", &[0], 0),         // no bits set -> 0
        ("lowest_set_bit", &[12], 4),        // 0b1100 -> lowest set bit is bit 2 (4)
        ("lowest_set_bit", &[64], 64),       // power of two is its own lowest set bit
        ("lowest_set_bit", &[7], 1),         // odd number always isolates bit 0 (1)
        ("lowest_set_bit", &[80], 16),       // 0b1010000 -> lowest set bit is bit 4 (16)
        ("lowest_set_bit", &[65535], 1),     // all bits set -> lowest set bit is bit 0 (1)
        ("lowest_set_bit", &[32768], 32768), // only the top bit set -> isolates itself
    ];

    let mut failures = Vec::new();
    for (id, args, exp) in cases {
        let got = run_cell(id, args);
        if got != *exp {
            failures.push(format!("{id}({args:?}) = {got}, expected {exp}"));
        }
    }
    assert!(failures.is_empty(), "failures:\n{}", failures.join("\n"));
}

#[test]
fn highest_set_bit_matches_hand_computed_cases() {
    // highest_set_bit(x): value of the top set bit of x (a mask, not a bit index),
    // 0 when x == 0. Implemented via smear-then-subtract: OR x down into all lower
    // bits until every bit below the highest set bit is 1, then subtract the
    // right-shift-by-1 of that from itself to leave only the top bit standing.
    fn run_cell(id: &str, args: &[u16]) -> u16 {
        let mut r = cell80::Runner::compile(&crate::common::cell_src(id))
            .unwrap_or_else(|e| panic!("compile {id}: {e}"));
        r.run(None, args, cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run {id}: {e}"))
            .result
    }

    let cases: &[(&str, &[u16], u16)] = &[
        ("highest_set_bit", &[0], 0),       // no bits set at all -> defined as 0
        ("highest_set_bit", &[1], 1),       // only bit 0 set -> top bit is itself
        ("highest_set_bit", &[5], 4),       // 0b101 -> highest bit is bit 2 (4)
        ("highest_set_bit", &[6], 4),       // 0b110 -> highest bit is bit 2 (4)
        ("highest_set_bit", &[1024], 1024), // already a lone power of two -> mirrors back
        ("highest_set_bit", &[65535], 32768), // 0xFFFF -> highest bit is bit 15 (0x8000)
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
fn clear_lowest_set_bit_matches_hand_computed_cases() {
    // x & (x - 1) clears the lowest set bit of x. Hand-computed:
    //   0     (0b0000000000000000) -> x-1 wraps to 0xFFFF, 0 & 0xFFFF = 0
    //   1     (0b0000000000000001) -> x-1 = 0,      1 & 0 = 0
    //   6     (0b0000000000000110) -> x-1 = 5 (0b101), 6 & 5 = 4 (0b100)
    //   12    (0b0000000000001100) -> x-1 = 11 (0b1011), 12 & 11 = 8 (0b1000)
    //   65535 (0b1111111111111111) -> x-1 = 65534 (0b1111111111111110), 65535 & 65534 = 65534
    let cases: &[(u16, u16)] = &[(0, 0), (1, 0), (6, 4), (12, 8), (65535, 65534)];

    for (x, expected) in cases {
        let got = run_cell("clear_lowest_set_bit", &[*x]);
        assert_eq!(
            got, *expected,
            "clear_lowest_set_bit({x}) = {got}, expected {expected}"
        );
    }
}

#[test]
fn mask_overlap_count_matches_defined_behaviour() {
    // mask_overlap_count(a, b) = popcount(a & b): the number of bit positions where
    // both a and b are set. Distinct from mask_intersection (returns the mask a & b
    // itself) and hamming_distance16 (counts bits that DIFFER, popcount(a ^ b)).
    // Cases hand-computed below.
    let cases: &[(&str, &[u16], u16)] = &[
        ("mask_overlap_count", &[15, 5], 2), // 0b1111 & 0b0101 = 0b0101 -> 2 set bits
        ("mask_overlap_count", &[65535, 65535], 16), // full overlap on all 16 bits
        ("mask_overlap_count", &[0, 65535], 0), // a has no bits -> no overlap possible
        ("mask_overlap_count", &[10, 12], 1), // 0b1010 & 0b1100 = 0b1000 -> 1 set bit
        ("mask_overlap_count", &[255, 256], 0), // disjoint bit ranges (low byte vs bit 8) -> 0
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
fn set_bit_u32_wide_sibling_matches_hand_computed_cases() {
    // set_bit_u32: the wide (u32-domain) sibling of set_bit -- sets bit number `bit`
    // (0-31) of x to 1 via a hi/lo u16-half split with constant-shift OR, since
    // set_bit's fn run(x: u16, bit: u16) can't accept a 32-bit x under the 16-bit
    // calling convention. Cases hand-computed via x | (1 << bit).
    fn set_bit_u32(x: u32, bit: u16) -> u32 {
        let mut cell =
            cell80::StateCell::bind(&crate::common::cell_src("set_bit_u32"), "SetBitU32", None)
                .unwrap_or_else(|e| panic!("bind set_bit_u32: {e}"));
        cell.set("x", x as u64).unwrap();
        cell.set("bit", bit as u64).unwrap();
        cell.run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run set_bit_u32: {e}"));
        cell.get("out").unwrap() as u32
    }

    // low word, low bit: x=0, bit=3 -> 0x00000008
    assert_eq!(set_bit_u32(0, 3), 0x0000_0008);
    // low word, top bit of low half: x=0, bit=15 -> 0x00008000
    assert_eq!(set_bit_u32(0, 15), 0x0000_8000);
    // boundary: first bit of high half: x=0, bit=16 -> 0x00010000
    assert_eq!(set_bit_u32(0, 16), 0x0001_0000);
    // high word, existing high bit stays, sets bit 30: x=0x80000000, bit=30 -> 0xC0000000
    assert_eq!(set_bit_u32(0x8000_0000, 30), 0xC000_0000);
    // low half retains its other bits when OR-ing bit 0: x=0xFFFF0000, bit=0 -> 0xFFFF0001
    assert_eq!(set_bit_u32(0xFFFF_0000, 0), 0xFFFF_0001);
    // setting an already-set bit is a no-op: x=0x00010000, bit=16 -> 0x00010000
    assert_eq!(set_bit_u32(0x0001_0000, 16), 0x0001_0000);
}

#[test]
fn clear_bit_u32_wide_sibling_matches_hand_computed_cases() {
    // clear_bit_u32: the u32-width sibling of clear_bit, clears bit number `bit` (0-31) of
    // a 32-bit value x to 0. Needs a u32 state field (state cell) since clear_bit's
    // fn run(x: u16, bit: u16) can't accept a 32-bit x under the 16-bit calling convention.
    // Splits x into hi/lo 16-bit halves and clears within whichever half holds the target
    // bit, the same hi/lo split technique bit_is_set_u32 uses to read a bit.
    fn clear_bit_u32(x: u32, bit: u16) -> u32 {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("clear_bit_u32"),
            "ClearBitU32",
            None,
        )
        .unwrap_or_else(|e| panic!("bind clear_bit_u32: {e}"));
        cell.set("x", x as u64).unwrap();
        cell.set("bit", bit as u64).unwrap();
        cell.run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run clear_bit_u32: {e}"));
        cell.get("out").unwrap() as u32
    }

    // low word, cross-checks clear_bit(15,1)=13 when x fits entirely in the low half.
    assert_eq!(clear_bit_u32(15, 1), 13);
    // low word, cross-checks clear_bit(8,3)=0.
    assert_eq!(clear_bit_u32(8, 3), 0);
    // high word boundary: bit 31 is the sole set bit -> clearing it yields 0.
    assert_eq!(clear_bit_u32(0x8000_0000, 31), 0);
    // all 32 bits set, clear bit 16 (lowest bit of the high half) -> removes 0x00010000.
    assert_eq!(clear_bit_u32(0xFFFF_FFFF, 16), 0xFFFE_FFFF);
    // hi/lo boundary: bit15 of low half set alongside bit16 of high half; clearing bit15
    // must not bleed into the high half -> 0x00010000 survives untouched.
    assert_eq!(clear_bit_u32(0x0001_8000, 15), 0x0001_0000);
}

#[test]
fn toggle_bit_u32_wide_sibling_matches_hand_computed_cases() {
    // toggle_bit_u32: the u32-width sibling of toggle_bit -- flips bit number `bit`
    // (0-31) of a 32-bit value x via XOR, needs a u32 state field since toggle_bit's
    // fn run(x: u16, bit: u16) can't accept a 32-bit x under the 16-bit calling
    // convention. Splits x into hi/lo 16-bit halves and XORs the target bit within
    // whichever half holds it. Cases hand-computed via x ^ (1 << bit).
    fn toggle_bit_u32(x: u32, bit: u16) -> u32 {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("toggle_bit_u32"),
            "ToggleBitU32",
            None,
        )
        .unwrap_or_else(|e| panic!("bind toggle_bit_u32: {e}"));
        cell.set("x", x as u64).unwrap();
        cell.set("bit", bit as u64).unwrap();
        cell.run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run toggle_bit_u32: {e}"));
        cell.get("out").unwrap() as u32
    }

    // low half, currently clear -> flips on: 0, bit 3 -> 8 (0b1000)
    assert_eq!(toggle_bit_u32(0, 3), 8);
    // low half, currently set -> flips off: 8, bit 3 -> 0
    assert_eq!(toggle_bit_u32(8, 3), 0);
    // high half, top bit, currently clear -> flips on: 0, bit 31 -> 0x80000000
    assert_eq!(toggle_bit_u32(0, 31), 0x8000_0000);
    // high half, top bit, currently set -> flips off: 0x80000000, bit 31 -> 0
    assert_eq!(toggle_bit_u32(0x8000_0000, 31), 0);
    // low/high boundary: 0x0000FFFF, bit 16 (lowest bit of high half, clear) -> flips
    // on without disturbing the low half -> 0x0001FFFF
    assert_eq!(toggle_bit_u32(0x0000_FFFF, 16), 0x0001_FFFF);
    // all bits set, low bit -> flips off: 0xFFFFFFFF, bit 0 -> 0xFFFFFFFE
    assert_eq!(toggle_bit_u32(0xFFFF_FFFF, 0), 0xFFFF_FFFE);
}

#[test]
fn hamming_distance32_matches_hand_computed_expectations() {
    // hamming_distance32: the u32-width sibling of hamming_distance16 -- counts differing
    // bit positions across a full 32-bit pair via u32 state fields, since
    // hamming_distance16's fn run(a: u16, b: u16) cannot represent inputs with bits above
    // position 15. Cases hand-computed: identical values, full complement, a lone high bit
    // (bit 31, beyond hamming_distance16's domain), a mixed nibble pattern, and a general
    // two-nonzero-operand case cross-checked against hamming_distance16's own worked example
    // (0xABCD, 0x1234) -> 11, repeated in both 16-bit halves of the 32-bit word.
    fn hamming32(a: u32, b: u32) -> u16 {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("hamming_distance32"),
            "HammingDistance32",
            None,
        )
        .unwrap_or_else(|e| panic!("bind hamming_distance32: {e}"));
        cell.set("a", a as u64).unwrap();
        cell.set("b", b as u64).unwrap();
        cell.run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run hamming_distance32: {e}"));
        cell.get("out").unwrap() as u16
    }

    // 1) a=0, b=0 -> xor=0 -> 0 bits differ.
    assert_eq!(hamming32(0, 0), 0);

    // 2) a=0xFFFFFFFF, b=0 -> xor=0xFFFFFFFF -> all 32 bits differ -> 32.
    assert_eq!(hamming32(0xFFFF_FFFF, 0), 32);

    // 3) a=0x80000000 (bit 31 only), b=0 -> xor has 1 bit set -> 1. Proves the high
    //    word (beyond hamming_distance16's 16-bit domain) is counted correctly.
    assert_eq!(hamming32(0x8000_0000, 0), 1);

    // 4) a=0x12345678, b=0 -> xor=0x12345678, nibble popcounts 1,1,2,1,2,2,3,1 -> sum=13.
    assert_eq!(hamming32(0x1234_5678, 0), 13);

    // 5) a=0xABCDABCD, b=0x12341234 -> xor = 0xB9F9B9F9. Each 16-bit half is
    //    0xABCD ^ 0x1234 = 0xB9F9 (1011 1001 1111 1001 -> 3+2+4+2 = 11 bits), matching
    //    hamming_distance16(0xABCD, 0x1234)=11 exactly, repeated in both halves -> 22.
    assert_eq!(hamming32(0xABCD_ABCD, 0x1234_1234), 22);
}
