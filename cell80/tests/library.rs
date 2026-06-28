//! Host-oracle tests for the first library wave (`cell80/cells/*.rs`). Each cell is
//! compiled straight from its source file and run through the warm `Runner`, then checked
//! against its **defined** behaviour — saturating arithmetic, `div`/`mod` by zero → 0,
//! predicates → `0`/`1`, runtime bit shifts, and the integer (`u16`) envelope. This is the
//! per-cell edge-case guard the contribution rule asks for; it complements the
//! `cell-eval` retrieval/composition datasets (which exercise discovery + chaining).

use cell80::{Runner, DEFAULT_CYCLES};
use std::path::PathBuf;

/// Read a library cell's source by id (`cells/<id>.rs`).
fn cell_src(id: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("cells")
        .join(format!("{id}.rs"));
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

/// Compile + run a cell on `args`, returning its `HL` result.
fn run_cell(id: &str, args: &[u16]) -> u16 {
    let mut r = Runner::compile(&cell_src(id)).unwrap_or_else(|e| panic!("compile {id}: {e}"));
    r.run(None, args, DEFAULT_CYCLES)
        .unwrap_or_else(|e| panic!("run {id}: {e}"))
        .result
}

#[test]
fn first_wave_cells_match_defined_behaviour() {
    // (id, args, expected). Chosen to hit each cell's edges: ties, zero divisors,
    // saturation, exclusive/inclusive bounds, the percent domain, and bit boundaries.
    let cases: &[(&str, &[u16], u16)] = &[
        // ── comparison predicates (→ 0/1) ──
        ("eq", &[5, 5], 1),
        ("eq", &[5, 6], 0),
        ("neq", &[5, 6], 1),
        ("neq", &[5, 5], 0),
        ("is_lt", &[3, 5], 1),
        ("is_lt", &[5, 5], 0),
        ("is_le", &[5, 5], 1),
        ("is_le", &[6, 5], 0),
        ("is_gt", &[6, 5], 1),
        ("is_gt", &[5, 5], 0),
        ("is_ge", &[5, 5], 1),
        ("is_ge", &[4, 5], 0),
        ("is_zero", &[0], 1),
        ("is_zero", &[3], 0),
        ("nonzero", &[3], 1),
        ("nonzero", &[0], 0),
        ("is_even", &[4], 1),
        ("is_even", &[0], 1),
        ("is_even", &[7], 0),
        ("is_odd", &[7], 1),
        ("is_odd", &[4], 0),
        // ── safe / core arithmetic ──
        ("add_sat", &[100, 50], 150),
        ("add_sat", &[65535, 10], 65535),
        ("add_sat", &[60000, 6000], 65535),
        ("sub_sat", &[100, 30], 70),
        ("sub_sat", &[30, 100], 0),
        ("mul_sat", &[12, 12], 144),
        ("mul_sat", &[0, 9999], 0),
        ("mul_sat", &[1000, 1000], 65535),
        ("safe_div", &[17, 5], 3),
        ("safe_div", &[9, 0], 0),
        ("safe_mod", &[17, 5], 2),
        ("safe_mod", &[9, 0], 0),
        ("ceil_div", &[17, 5], 4),
        ("ceil_div", &[10, 5], 2),
        ("ceil_div", &[0, 5], 0),
        ("ceil_div", &[9, 0], 0),
        ("ceil_div", &[65535, 2], 32768),
        ("avg2", &[10, 20], 15),
        ("avg2", &[65534, 65534], 65534),
        ("square", &[12], 144),
        ("square", &[255], 65025),
        ("square", &[256], 65535),
        // ── bounds ──
        ("between_exclusive", &[5, 0, 10], 1),
        ("between_exclusive", &[0, 0, 10], 0),
        ("between_exclusive", &[10, 0, 10], 0),
        ("wrap", &[13, 10], 3),
        ("wrap", &[10, 10], 0),
        ("wrap", &[5, 0], 0),
        ("normalize_0_100", &[50, 0, 200], 25),
        ("normalize_0_100", &[300, 0, 200], 100),
        ("normalize_0_100", &[5, 10, 10], 0),
        ("snap_down", &[47, 10], 40),
        ("snap_down", &[9, 10], 0),
        ("snap_down", &[7, 0], 7),
        ("snap_up", &[41, 10], 50),
        ("snap_up", &[40, 10], 40),
        ("snap_up", &[0, 10], 0),
        ("round_to_multiple", &[47, 10], 50),
        ("round_to_multiple", &[44, 10], 40),
        // ── percent / ratio (u16 domain: value·scale ≤ 65535) ──
        ("percent", &[25, 200], 12),
        ("percent", &[1, 4], 25),
        ("percent", &[5, 0], 0),
        ("permille", &[1, 4], 250),
        ("permille", &[5, 0], 0),
        ("ratio_255", &[1, 2], 127),
        ("ratio_255", &[1, 1], 255),
        ("scale_percent", &[80, 25], 20),
        ("increase_percent", &[600, 50], 900),
        ("increase_percent", &[65000, 1], 65535),
        ("discount_percent", &[100, 20], 80),
        ("discount_percent", &[50, 150], 0),
        ("within_percent", &[95, 100, 10], 1),
        ("within_percent", &[80, 100, 10], 0),
        // ── ranking / extremum / stats ──
        ("min3", &[5, 2, 8], 2),
        ("min3", &[9, 9, 9], 9),
        ("max3", &[5, 2, 8], 8),
        ("max3", &[1, 40000, 2], 40000),
        ("median3", &[5, 2, 8], 5),
        ("median3", &[1, 2, 3], 2),
        ("median3", &[40000, 65535, 1], 40000),
        ("argmax2", &[5, 8], 1),
        ("argmax2", &[5, 5], 0),
        ("argmin2", &[8, 5], 1),
        ("argmax3", &[5, 2, 8], 2),
        ("argmax3", &[9, 9, 9], 0),
        ("argmin3", &[5, 8, 2], 2),
        ("sum3", &[10, 20, 30], 60),
        ("sum3", &[60000, 60000, 60000], 65535),
        ("mean3", &[10, 20, 30], 20),
        ("mean3", &[65535, 65535, 65535], 65535),
        ("range3", &[1, 40000, 100], 39999),
        // ── bit ops (runtime shifts) ──
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
    ];

    let mut failures = Vec::new();
    for (id, args, exp) in cases {
        let got = run_cell(id, args);
        if got != *exp {
            failures.push(format!("{id}({args:?}) = {got}, expected {exp}"));
        }
    }
    assert!(failures.is_empty(), "cell mismatches:\n{}", failures.join("\n"));
}
