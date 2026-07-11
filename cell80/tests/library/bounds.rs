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

#[test]
fn normalize_0_100_u32_matches_hand_computed_expectations() {
    // NormalizeWide: wide (u32) sibling of normalize_0_100 -- rescales x within [lo, hi]
    // to a 0..100 percentage, clamping x to [lo, hi] first (0 if hi <= lo). The
    // intermediate multiply (clamped x - lo) * 100 is checked, so it escalates instead
    // of silently wrapping when lo/hi span a wide enough range.
    let step = |x: u64, lo: u64, hi: u64| -> (u64, cell80::Report) {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("normalize_0_100_u32"),
            "NormalizeWide",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("x", x).unwrap();
        cell.set("lo", lo).unwrap();
        cell.set("hi", hi).unwrap();
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run: {e}"));
        (cell.get("result").unwrap_or(0), report)
    };

    // In-range: 50 within [0, 200] -> 25%.
    let (r, rep) = step(50, 0, 200);
    assert_eq!(rep.halt, cell80::Halt::Returned);
    assert_eq!(r, 25);

    // Above hi clamps to hi first: 300 clamped to 200, within [0, 200] -> 100%.
    let (r, rep) = step(300, 0, 200);
    assert_eq!(rep.halt, cell80::Halt::Returned);
    assert_eq!(r, 100);

    // Degenerate range hi <= lo -> 0 regardless of x.
    let (r, rep) = step(5, 10, 10);
    assert_eq!(rep.halt, cell80::Halt::Returned);
    assert_eq!(r, 0);

    // Wide case past the u16/65535 ceiling: 100_000 within [0, 200_000] -> 50%.
    let (r, rep) = step(100_000, 0, 200_000);
    assert_eq!(rep.halt, cell80::Halt::Returned);
    assert_eq!(r, 50);

    // Overflow escalation: (c - lo) * 100 overflows u32. x = hi = 50_000_000, lo = 0
    // -> c - lo = 50_000_000; 50_000_000 * 100 = 5_000_000_000 > u32::MAX
    // (4_294_967_295) -> escalates (needs_wider_math).
    let (_, rep) = step(50_000_000, 0, 50_000_000);
    assert_eq!(rep.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn value_at_percent_matches_hand_computed_expectations() {
    // Local helper: compile+run the bounds/value_at_percent free function on (lo, hi, pct).
    let run = |lo: u16, hi: u16, pct: u16| -> u16 {
        let mut r = cell80::Runner::compile(&crate::common::cell_src("value_at_percent"))
            .unwrap_or_else(|e| panic!("compile: {e}"));
        r.run(None, &[lo, hi, pct], cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run: {e}"))
            .result
    };

    // 0% of [0,200] -> lo = 0
    assert_eq!(run(0, 200, 0), 0);
    // 50% of [0,200] -> 0 + 200*50/100 = 100
    assert_eq!(run(0, 200, 50), 100);
    // 100% of [0,200] -> hi = 200
    assert_eq!(run(0, 200, 100), 200);
    // pct clamped past 100: 150% of [0,200] behaves like 100% -> 200
    assert_eq!(run(0, 200, 150), 200);
    // offset range: 25% of [100,300] -> 100 + (300-100)*25/100 = 150
    assert_eq!(run(100, 300, 25), 150);
    // degenerate hi == lo -> returns lo unconditionally
    assert_eq!(run(50, 50, 50), 50);
    // degenerate hi < lo -> returns lo
    assert_eq!(run(200, 100, 50), 200);
    // exact inverse of normalize_0_100(50, 0, 200) == 25 -> recovers 50
    assert_eq!(run(0, 200, 25), 50);
}


#[test]
fn value_at_percent_u32_matches_hand_computed_expectations() {
    // ValueAtPercentWide: wide (u32) sibling of value_at_percent -- given range [lo, hi]
    // and percentage pct (clamped to 100 if over), returns lo + (hi-lo)*pct/100 at u32
    // width (returns lo if hi <= lo). The intermediate multiply (hi-lo)*pct is checked,
    // so it escalates instead of silently wrapping when the span/pct combination is
    // wide enough to overflow u32.
    let step = |lo: u64, hi: u64, pct: u64| -> (u16, cell80::Report, cell80::StateCell) {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("value_at_percent_u32"),
            "ValueAtPercentWide",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("lo", lo).unwrap();
        cell.set("hi", hi).unwrap();
        cell.set("pct", pct).unwrap();
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run: {e}"));
        let result = report.result;
        (result, report, cell)
    };

    // 50% of [0, 200] -> 0 + 200*50/100 = 100
    let (_, report, cell) = step(0, 200, 50);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(100));

    // offset range: 25% of [100, 300] -> 100 + (300-100)*25/100 = 150
    let (_, report, cell) = step(100, 300, 25);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(150));

    // pct clamped past 100: 150% of [0, 200] behaves like 100% -> 200
    let (_, report, cell) = step(0, 200, 150);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(200));

    // degenerate hi < lo -> returns lo unconditionally
    let (_, report, cell) = step(200, 100, 50);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(200));

    // wide case past the u16/65535 ceiling: 50% of [0, 1_000_000] -> 500_000
    let (_, report, cell) = step(0, 1_000_000, 50);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(500_000));

    // Overflow escalation: span * pct overflows u32. span = hi - lo = 50_000_000,
    // pct = 100 (already <= 100) -> num = 50_000_000 * 100 = 5_000_000_000, which is
    // greater than u32::MAX (4_294_967_295) -> escalates (needs_wider_math).
    let (_, report, _) = step(0, 50_000_000, 100);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn outside_range_matches_defined_behaviour() {
    // outside_range: the exact logical complement of between_exclusive -- 1 if x <= lo
    // or x >= hi (x lies on or beyond either edge of the open interval), else 0.
    let cases: &[(&str, &[u16], u16)] = &[
        ("outside_range", &[5, 0, 10], 0),   // strictly inside (0,10) -> not outside
        ("outside_range", &[0, 0, 10], 1),   // x == lo (boundary excluded from open interval) -> outside
        ("outside_range", &[10, 0, 10], 1),  // x == hi (boundary excluded from open interval) -> outside
        ("outside_range", &[15, 0, 10], 1),  // x beyond hi -> outside
        ("outside_range", &[9, 0, 10], 0),   // x just inside the upper edge -> not outside
        ("outside_range", &[5, 5, 5], 1),    // degenerate empty interval (lo == hi) -> everything is outside
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
fn outside_range_u32_matches_hand_computed_expectations() {
    // OutsideRangeWide: wide (u32) sibling of outside_range -- 1 if x is outside the open
    // interval (lo, hi), i.e. x <= lo || x >= hi, else 0. This is the exact logical
    // complement of between_exclusive_u32, exercised past the u16/65535 ceiling since
    // that's the whole point of the wide variant (e.g. money totals in cents).
    let step = |fields: &[(&str, u64)]| -> u16 {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("outside_range_u32"),
            "OutsideRangeWide",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(cell80::DEFAULT_CYCLES).unwrap().result
    };

    // strictly inside -> not outside -> 0
    assert_eq!(step(&[("x", 5), ("lo", 0), ("hi", 10)]), 0);
    // at the lower bound (x <= lo) -> outside -> 1
    assert_eq!(step(&[("x", 0), ("lo", 0), ("hi", 10)]), 1);
    // at the upper bound (x >= hi) -> outside -> 1
    assert_eq!(step(&[("x", 10), ("lo", 0), ("hi", 10)]), 1);
    // wide: strictly inside past the u16 ceiling -> 0
    assert_eq!(step(&[("x", 100_000), ("lo", 50_000), ("hi", 200_000)]), 0);
    // wide: at the lower bound past the u16 ceiling -> 1
    assert_eq!(step(&[("x", 50_000), ("lo", 50_000), ("hi", 200_000)]), 1);
    // wide: strictly inside near u32::MAX -> 0 (exact complement of the matching
    // between_exclusive_u32 case, which returns 1 for the same inputs)
    assert_eq!(
        step(&[
            ("x", 4_000_000_000),
            ("lo", 3_000_000_000),
            ("hi", 4_294_967_295)
        ]),
        0
    );
    // wide: x >= hi at u32::MAX itself -> 1
    assert_eq!(
        step(&[
            ("x", 4_294_967_295),
            ("lo", 3_000_000_000),
            ("hi", 4_294_967_295)
        ]),
        1
    );
}

#[test]
fn remap_range_matches_hand_computed_expectations() {
    // RemapRange: fully general linear remap of x from [in_lo, in_hi] to [out_lo, out_hi].
    // clamps x into the input range first, then scales: out_lo + (x-in_lo)*(out_hi-out_lo)/(in_hi-in_lo).
    // Degenerate input range (in_hi <= in_lo) always returns out_lo.
    fn step(x: u16, in_lo: u16, in_hi: u16, out_lo: u16, out_hi: u16) -> u16 {
        let mut cell = cell80::StateCell::bind(&crate::common::cell_src("remap_range"), "RemapRange", None)
            .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("x", x as u64).unwrap();
        cell.set("in_lo", in_lo as u64).unwrap();
        cell.set("in_hi", in_hi as u64).unwrap();
        cell.set("out_lo", out_lo as u64).unwrap();
        cell.set("out_hi", out_hi as u64).unwrap();
        cell.run(cell80::DEFAULT_CYCLES).unwrap_or_else(|e| panic!("run: {e}"));
        cell.get("result").unwrap() as u16
    }

    // Basic doubling scale: x=50 in [0,100] -> [0,200]: (50-0)*(200-0)/(100-0) = 100.
    assert_eq!(step(50, 0, 100, 0, 200), 100);
    // Clamp above in_hi: x=150 clamps to 100 first -> (100-0)*(200-0)/(100-0) = 200.
    assert_eq!(step(150, 0, 100, 0, 200), 200);
    // Clamp below in_lo: x=5 clamps to in_lo=10 -> numerator is 0 -> result = out_lo = 20.
    assert_eq!(step(5, 10, 100, 20, 220), 20);
    // Degenerate input range (in_hi <= in_lo) -> always out_lo, regardless of x.
    assert_eq!(step(30, 50, 50, 7, 99), 7);
    // Fully arbitrary two-range remap: x=25 in [0,50] -> [100,200]:
    // (25-0)*(200-100)/(50-0) = 25*100/50 = 50 -> result = 100+50 = 150.
    assert_eq!(step(25, 0, 50, 100, 200), 150);
    // Cross-check against value_at_percent(lo=10,hi=20,pct=50)=15: using [0,100] as the
    // percent domain for `in` and [10,20] as `out` should reproduce the same answer.
    assert_eq!(step(50, 0, 100, 10, 20), 15);
}

#[test]
fn remap_range_u32_matches_hand_computed_expectations() {
    // RemapRangeWide: wide (u32) sibling of remap_range -- linearly maps x from
    // [in_lo, in_hi] into [out_lo, out_hi] (clamping x into the source range first,
    // falling back to out_lo if in_hi <= in_lo), using mul_checked_u32 for the
    // intermediate multiply since u32 operands can themselves overflow u32.
    let step = |fields: &[(&str, u64)]| -> (u64, cell80::Report) {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("remap_range_u32"),
            "RemapRangeWide",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run: {e}"));
        (cell.get("result").unwrap_or(0), report)
    };

    // Normal case: 50 within [0,100] maps to [0,200] -> midpoint -> 100
    let (r, rep) = step(&[("x", 50), ("in_lo", 0), ("in_hi", 100), ("out_lo", 0), ("out_hi", 200)]);
    assert_eq!(rep.halt, cell80::Halt::Returned);
    assert_eq!(r, 100);

    // x below in_lo clamps to in_lo -> maps to out_lo
    let (r, rep) = step(&[("x", 5), ("in_lo", 10), ("in_hi", 20), ("out_lo", 100), ("out_hi", 200)]);
    assert_eq!(rep.halt, cell80::Halt::Returned);
    assert_eq!(r, 100);

    // x above in_hi clamps to in_hi -> maps to out_hi
    let (r, rep) = step(&[("x", 999), ("in_lo", 10), ("in_hi", 20), ("out_lo", 100), ("out_hi", 200)]);
    assert_eq!(rep.halt, cell80::Halt::Returned);
    assert_eq!(r, 200);

    // Offset ranges: 150 within [100,300] -> (150-100)*(1000-0)/(300-100) = 50000/200 = 250
    let (r, rep) = step(&[("x", 150), ("in_lo", 100), ("in_hi", 300), ("out_lo", 0), ("out_hi", 1000)]);
    assert_eq!(rep.halt, cell80::Halt::Returned);
    assert_eq!(r, 250);

    // Degenerate source range (in_hi <= in_lo) -> falls back to out_lo regardless of x
    let (r, rep) = step(&[("x", 1000), ("in_lo", 50), ("in_hi", 50), ("out_lo", 7), ("out_hi", 99)]);
    assert_eq!(rep.halt, cell80::Halt::Returned);
    assert_eq!(r, 7);

    // Wide-scale normal case past the u16/65535 ceiling:
    // 1,500,000 within [1,000,000, 2,000,000] -> (500,000*100)/1,000,000 = 50
    let (r, rep) = step(&[
        ("x", 1_500_000),
        ("in_lo", 1_000_000),
        ("in_hi", 2_000_000),
        ("out_lo", 0),
        ("out_hi", 100),
    ]);
    assert_eq!(rep.halt, cell80::Halt::Returned);
    assert_eq!(r, 50);

    // Overflow escalation: (c - in_lo) * (out_hi - out_lo) overflows u32.
    // x=3 clamped within [0,10] -> c-in_lo=3; out_hi-out_lo=2,000,000,000;
    // 3 * 2,000,000,000 = 6,000,000,000 > u32::MAX (4,294,967,295) -> escalate.
    let (_, rep) = step(&[("x", 3), ("in_lo", 0), ("in_hi", 10), ("out_lo", 0), ("out_hi", 2_000_000_000)]);
    assert_eq!(rep.halt, cell80::Halt::Escalate(0xFF05));
}
