//! Host-oracle tests for the bounds pack (`cell80/cells/bounds/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::run_cell;

#[test]
fn first_wave_bounds_cells_match_defined_behaviour() {
    let cases: &[(&str, &[u16], u16)] = &[
        ("between_exclusive", &[5, 0, 10], 1),
        ("between_exclusive", &[0, 0, 10], 0),
        ("between_exclusive", &[10, 0, 10], 0),
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
fn between_exclusive_u32_matches_defined_behaviour() {
    // BetweenExclusiveWide: wide (u32) sibling of between_exclusive — 1 if lo < x < hi
    // (strict, exclusive bounds), else 0. Exercised past the u16/65535 ceiling since
    // that's the whole point of the wide variant (e.g. money totals in cents).
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> u16 {
        let mut cell = cell80::StateCell::bind(&crate::common::cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(cell80::DEFAULT_CYCLES).unwrap().result
    }

    // strictly inside -> 1
    assert_eq!(
        step(
            "between_exclusive_u32",
            "BetweenExclusiveWide",
            &[("x", 5), ("lo", 0), ("hi", 10)]
        ),
        1
    );
    // at the lower bound (not strictly greater) -> 0
    assert_eq!(
        step(
            "between_exclusive_u32",
            "BetweenExclusiveWide",
            &[("x", 0), ("lo", 0), ("hi", 10)]
        ),
        0
    );
    // at the upper bound (not strictly less) -> 0
    assert_eq!(
        step(
            "between_exclusive_u32",
            "BetweenExclusiveWide",
            &[("x", 10), ("lo", 0), ("hi", 10)]
        ),
        0
    );
    // wide: strictly inside past the u16 ceiling -> 1
    assert_eq!(
        step(
            "between_exclusive_u32",
            "BetweenExclusiveWide",
            &[("x", 100_000), ("lo", 50_000), ("hi", 200_000)]
        ),
        1
    );
    // wide: at the lower bound past the u16 ceiling -> 0
    assert_eq!(
        step(
            "between_exclusive_u32",
            "BetweenExclusiveWide",
            &[("x", 50_000), ("lo", 50_000), ("hi", 200_000)]
        ),
        0
    );
    // wide: strictly inside near u32::MAX -> 1
    assert_eq!(
        step(
            "between_exclusive_u32",
            "BetweenExclusiveWide",
            &[
                ("x", 4_000_000_000),
                ("lo", 3_000_000_000),
                ("hi", 4_294_967_295)
            ]
        ),
        1
    );
}

#[test]
fn round_to_multiple_u32_matches_hand_computed_expectations() {
    // Local helper: bind the state cell, set x/step, run it, and hand back (result, halt-report, cell)
    // so callers can inspect either the returned field or the halt code.
    fn step(x: u32, step_v: u32) -> (u16, cell80::Report, cell80::StateCell) {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("cells/bounds/round_to_multiple_u32.rs");
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let mut cell = cell80::StateCell::bind(&src, "RoundToMultipleWide", None)
            .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("x", x as u64).unwrap();
        cell.set("step", step_v as u64).unwrap();
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run: {e}"));
        let result = report.result;
        (result, report, cell)
    }

    // Normal case: 47 rounds to nearest multiple of 10 -> 50 (distance 3 vs 7).
    let (_, report, cell) = step(47, 10);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(50));

    // Normal case: 44 rounds to nearest multiple of 10 -> 40 (distance 4 vs 6).
    let (_, report, cell) = step(44, 10);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(40));

    // Exact tie: 6 is equidistant from 4 and 8 (multiples of 4) -> ties up to 8.
    let (_, report, cell) = step(6, 4);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(8));

    // step == 0 passthrough: result is x unchanged.
    let (_, report, cell) = step(5, 0);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(5));

    // Overflow in the ties-up add: x = u32::MAX - 9, step = 20 -> half = 10,
    // x + half = u32::MAX + 1, which overflows u32 -> escalate (needs_wider_math).
    let (_, report, _) = step(u32::MAX - 9, 20);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn snap_down_u32_matches_defined_behaviour() {
    use cell80::{StateCell, DEFAULT_CYCLES};

    // Local helper: bind SnapDownWide, set x/step, run, and read back `result`.
    let step = |x: u64, s: u64| -> u64 {
        let mut cell = StateCell::bind(
            &crate::common::cell_src("snap_down_u32"),
            "SnapDownWide",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("x", x).unwrap();
        cell.set("step", s).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap();
        cell.get("result").unwrap()
    };

    // 100 floors to the nearest multiple of 30 below it: 3*30 = 90.
    assert_eq!(step(100, 30), 90);
    // u32::MAX floors to the nearest multiple of 1000 below it: 4294967295 // 1000 * 1000 = 4294967000.
    assert_eq!(step(4294967295, 1000), 4294967000);
    // step == 0 passes x through unchanged (no grid to snap to).
    assert_eq!(step(500, 0), 500);
    // x == 0 floors to 0 regardless of step.
    assert_eq!(step(0, 5), 0);
    // Widened version of the documented snap_down/round_to_multiple divergence case at
    // u16 scale (65531, 3): 65531 // 3 * 3 = 65529.
    assert_eq!(step(65531, 3), 65529);
}

#[test]
fn snap_up_u32_matches_defined_behaviour() {
    // snap_up_u32 is the u32-width sibling of snap_up: ceil x to the nearest multiple of
    // step, escalating instead of wrapping if the ceiling scale-back multiply overflows u32.
    let step = |fields: &[(&str, u64)]| -> (u16, cell80::Report, cell80::StateCell) {
        let mut cell =
            cell80::StateCell::bind(&crate::common::cell_src("snap_up_u32"), "SnapUpWide", None)
                .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(cell80::DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    };

    // 41 snapped up to nearest multiple of 10 -> 50 (mirrors snap_up's u16 semantics, at u32 width)
    let (_, report, cell) = step(&[("x", 41), ("step", 10)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(50));

    // 40 is already a multiple of 10 -> stays 40 (no rounding up past an exact hit)
    let (_, report, cell) = step(&[("x", 40), ("step", 10)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(40));

    // x == 0 -> 0 regardless of step (explicit x==0 special case)
    let (_, report, cell) = step(&[("x", 0), ("step", 10)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(0));

    // step == 0 -> x unchanged, exercised past u16 range (100000) to confirm u32 width is real
    let (_, report, cell) = step(&[("x", 100_000), ("step", 0)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(100_000));

    // 100000 snapped up to nearest multiple of 7: ceil(100000/7) = 14286, 14286*7 = 100002
    let (_, report, cell) = step(&[("x", 100_000), ("step", 7)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(100_002));

    // Near u32::MAX with a step that forces the ceiling scale-back multiply past u32::MAX:
    // x = u32::MAX - 4 = 4294967291, step = 100000.
    // q = (x-1)/step + 1 = 4294967290/100000 + 1 = 42949 + 1 = 42950
    // q*step = 42950 * 100000 = 4295000000 > u32::MAX (4294967295) -> escalate
    let (_, report, _) = step(&[("x", 4_294_967_291), ("step", 100_000)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}
