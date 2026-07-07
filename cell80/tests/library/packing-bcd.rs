//! Host-oracle tests for the packing-bcd pack (`cell80/cells/packing-bcd/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::run_cell;

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
