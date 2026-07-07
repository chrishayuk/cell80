//! Host-oracle tests for the bucket-convert pack (`cell80/cells/bucket-convert/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::run_cell;

#[test]
fn first_wave_bucket_convert_cells_match_defined_behaviour() {
    let cases: &[(&str, &[u16], u16)] = &[
        ("bucket3", &[5, 10, 20], 0),
        ("bucket3", &[15, 10, 20], 1),
        ("bucket3", &[25, 10, 20], 2),
        ("percent_to_byte", &[100], 255),
        ("percent_to_byte", &[50], 127),
        ("byte_to_percent", &[255], 100),
        ("byte_to_percent", &[127], 49),
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
