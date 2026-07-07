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
