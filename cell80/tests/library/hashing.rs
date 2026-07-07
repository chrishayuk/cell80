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
