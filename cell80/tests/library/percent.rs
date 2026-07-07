//! Host-oracle tests for the percent pack (`cell80/cells/percent/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::run_cell;

#[test]
fn first_wave_percent_cells_match_defined_behaviour() {
    let cases: &[(&str, &[u16], u16)] = &[
        ("percent", &[25, 200], 12),
        ("percent", &[1, 4], 25),
        ("percent", &[5, 0], 0),
        ("percent", &[700, 1000], 70), // part*100 > 65535 — the old u16 wrap gave 4
        ("percent", &[65535, 65535], 100), // the domain extreme
        ("permille", &[1, 4], 250),
        ("permille", &[5, 0], 0),
        ("permille", &[700, 1000], 700), // part*1000 wraps hard at u16
        ("ratio_255", &[1, 2], 127),
        ("ratio_255", &[1, 1], 255),
        ("ratio_255", &[300, 255], 300), // part*255 > 65535
        ("scale_percent", &[80, 25], 20),
        ("scale_percent", &[1000, 200], 2000), // value*pct > 65535
        ("scale_percent", &[65535, 65535], 65535), // saturates at the u16 return
        ("increase_percent", &[600, 50], 900),
        ("increase_percent", &[65000, 1], 65535),
        ("discount_percent", &[100, 20], 80),
        ("discount_percent", &[50, 150], 0),
        ("within_percent", &[95, 100, 10], 1),
        ("within_percent", &[80, 100, 10], 0),
        ("within_percent", &[1500, 1000, 100], 1), // target*pct wraps at u16 — flipped the predicate
        ("within_percent", &[3000, 1000, 100], 0), // wide compare on both sides
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
