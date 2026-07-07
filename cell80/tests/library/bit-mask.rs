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
