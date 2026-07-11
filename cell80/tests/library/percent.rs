//! Host-oracle tests for the percent pack (`cell80/cells/percent/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::{cell_src, run_cell};
use cell80::{StateCell, DEFAULT_CYCLES};

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

#[test]
fn percent_u32_wide_sibling_matches_defined_behaviour() {
    // percent_u32 (PercentWide) is the u32-wide sibling of percent: part*100/whole,
    // uncapped (no 65535 saturation, unlike the u16 original) and escalating
    // (needs_wider_math) if the part*100 multiply overflows u32 rather than
    // silently wrapping.
    fn cell_src(id: &str) -> String {
        let cells_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cells");
        let p = cell80::find_cell_file(&cells_dir, id).unwrap_or_else(|e| panic!("{e}"));
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
    }
    fn step(
        id: &str,
        strct: &str,
        fields: &[(&str, u64)],
    ) -> (u16, cell80::Report, cell80::StateCell) {
        let mut cell = cell80::StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(cell80::DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }

    // 25% of 200 -> part*100/whole = 2500/200 = 12
    let (_, report, cell) = step(
        "percent_u32",
        "PercentWide",
        &[("part", 25), ("whole", 200)],
    );
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(12));

    // whole == 0 guards to 0, same convention as the u16 original
    let (_, report, cell) = step("percent_u32", "PercentWide", &[("part", 5), ("whole", 0)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(0));

    // part*100 = 70,000,000, well past u16::MAX (65535) -- proves this is
    // uncapped rather than saturating like the u16 sibling would.
    let (_, report, cell) = step(
        "percent_u32",
        "PercentWide",
        &[("part", 700000), ("whole", 1000)],
    );
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(70000));

    // part*100 overflowing u32 (4294967295*100 > u32::MAX) escalates rather
    // than wrapping around.
    let (_, report, _) = step(
        "percent_u32",
        "PercentWide",
        &[("part", 4294967295), ("whole", 3)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

// Checks discount_percent_u32 (DiscountPercentWide) against hand-computed expectations:
// the u32-width discount value - value*pct/100 (0 if pct >= 100), including the
// checked-multiply escalation path when value*pct overflows u32.
#[test]
fn discount_percent_u32_cases() {
    fn step(fields: &[(&str, u64)]) -> (u16, cell80::Report, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("discount_percent_u32"),
            "DiscountPercentWide",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }

    // 1) 100000 - 100000*20/100 = 100000 - 20000 = 80000
    let (_, report, cell) = step(&[("value", 100_000), ("pct", 20)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(80_000));

    // 2) pct = 0 -> no discount: 100000 - 0 = 100000
    let (_, report, cell) = step(&[("value", 100_000), ("pct", 0)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(100_000));

    // 3) pct >= 100 -> 0, regardless of value (matches discount_percent's u16 semantics)
    let (_, report, cell) = step(&[("value", 50), ("pct", 150)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(0));

    // 4) 1000000 - 1000000*99/100 = 1000000 - 990000 = 10000 (exceeds u16 range, exercises the u32 width)
    let (_, report, cell) = step(&[("value", 1_000_000), ("pct", 99)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(10_000));

    // 5) value*pct overflows u32: 100000 * 100000 = 10_000_000_000 > u32::MAX -> escalate
    // (the multiply happens before the pct>=100 short-circuit, matching the sibling bps cells' style)
    let (_, report, _) = step(&[("value", 100_000), ("pct", 100_000)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn increase_percent_u32_matches_defined_behaviour() {
    // Wide sibling of increase_percent: value + value*pct/100 at u32, escalating
    // (Halt::Escalate(0xFF05), needs_wider_math) on multiply-or-add overflow instead of
    // saturating like the u16 sibling does.
    fn step(fields: &[(&str, u64)]) -> (u16, cell80::Report, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("increase_percent_u32"),
            "IncreasePercentWide",
            None,
        )
        .unwrap_or_else(|e| panic!("bind increase_percent_u32: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }

    // Ordinary case: 1000 + 1000*10/100 = 1100.
    let (_, report, cell) = step(&[("value", 1000), ("pct", 10)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(1100));

    // Past the u16 ceiling (exercises the u32 width): 65000 + 65000*50/100 = 97500.
    let (_, report, cell) = step(&[("value", 65000), ("pct", 50)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(97500));

    // value*pct itself overflows u32: 3_000_000_000 * 2 = 6_000_000_000 > u32::MAX -> escalate.
    let (_, report, _) = step(&[("value", 3_000_000_000), ("pct", 2)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // value*pct fits (4_290_000_000), but value + inc (4_290_000_000 + 42_900_000 =
    // 4_332_900_000) overflows u32::MAX (4_294_967_295) on the add step -> escalate.
    let (_, report, _) = step(&[("value", 4_290_000_000), ("pct", 1)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // Exact boundary, no escalation: 100 + 100*100/100 = 200.
    let (_, report, cell) = step(&[("value", 100), ("pct", 100)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(200));
}

// Checks permille_u32 (PermilleWide) against hand-computed expectations: the u32-width
// per-mille part*1000/whole (0 if whole == 0), uncapped (no 65535 saturation, unlike the
// u16 permille original) and escalating (needs_wider_math) if the part*1000 multiply
// overflows u32 rather than silently wrapping.
#[test]
fn permille_u32_wide_sibling_matches_defined_behaviour() {
    fn step(fields: &[(&str, u64)]) -> (u16, cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src("permille_u32"), "PermilleWide", None)
            .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }

    // 1) 25/200 -> part*1000/whole = 25000/200 = 125 (basic exact division)
    let (_, report, cell) = step(&[("part", 25), ("whole", 200)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(125));

    // 2) 1/4 -> 1000/4 = 250, matches the u16 permille sibling's own case on shared domain
    let (_, report, cell) = step(&[("part", 1), ("whole", 4)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(250));

    // 3) whole == 0 guards to 0, same convention as the u16 original
    let (_, report, cell) = step(&[("part", 5), ("whole", 0)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(0));

    // 4) part=700000, whole=1000 -> 700000, well past u16::MAX (65535) -- proves this is
    // uncapped rather than saturating like the u16 sibling would.
    let (_, report, cell) = step(&[("part", 700_000), ("whole", 1000)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(700_000));

    // 5) part*1000 overflowing u32 (4294967295*1000 > u32::MAX) escalates rather than
    // wrapping around.
    let (_, report, _) = step(&[("part", 4294967295), ("whole", 3)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn ratio_255_u32_matches_defined_behaviour() {
    // Wide sibling of ratio_255: part*255/whole at u32 width, uncapped (no 65535
    // saturation, unlike the u16 original) and escalating (needs_wider_math) if the
    // part*255 multiply overflows u32 rather than silently wrapping.
    fn step(fields: &[(&str, u64)]) -> (u16, cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src("ratio_255_u32"), "Ratio255Wide", None)
            .unwrap_or_else(|e| panic!("bind ratio_255_u32: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }

    // 1) part=1, whole=2 -> 1*255/2 = 127 (matches ratio_255's u16 case)
    let (_, report, cell) = step(&[("part", 1), ("whole", 2)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(127));

    // 2) part=1, whole=1 -> 1*255/1 = 255 (full ratio)
    let (_, report, cell) = step(&[("part", 1), ("whole", 1)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(255));

    // 3) whole == 0 guards to 0, same convention as the u16 original
    let (_, report, cell) = step(&[("part", 5), ("whole", 0)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(0));

    // 4) part=700000, whole=1000 -> 700000*255/1000 = 178500, well past u16::MAX
    // (65535) -- proves this is uncapped rather than saturating like the u16 sibling.
    let (_, report, cell) = step(&[("part", 700000), ("whole", 1000)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(178500));

    // 5) part*255 overflowing u32 (4294967295*255 > u32::MAX) escalates rather
    // than wrapping around.
    let (_, report, _) = step(&[("part", 4294967295), ("whole", 3)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}


// percent_change_between: percent-scale sibling of money-bps's bps_change_between, at
// u16 width using the percent pack's saturate (not halt) convention. Verifies rise,
// fall, the before==0 -> 0/0 case, saturation at 65535, an over-100% case, and the
// no-change case (neg forced to 0, not left ambiguous).
#[test]
fn percent_change_between_matches_defined_behaviour() {
    fn step(fields: &[(&str, u64)]) -> StateCell {
        let mut cell = StateCell::bind(&cell_src("percent_change_between"), "PercentChangeBetween", None)
            .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap();
        cell
    }

    // Rose: before=200, after=250 -> diff=50, 50*100/200 = 25% increase, neg=0.
    let cell = step(&[("before", 200), ("after", 250)]);
    assert_eq!(cell.get("pct_mag"), Some(25));
    assert_eq!(cell.get("pct_neg"), Some(0));

    // Fell: before=200, after=150 -> diff=50, same 25% magnitude, neg=1.
    let cell = step(&[("before", 200), ("after", 150)]);
    assert_eq!(cell.get("pct_mag"), Some(25));
    assert_eq!(cell.get("pct_neg"), Some(1));

    // before == 0 -> 0/0 convention, no halt (unlike bps_change_between, which halts).
    let cell = step(&[("before", 0), ("after", 500)]);
    assert_eq!(cell.get("pct_mag"), Some(0));
    assert_eq!(cell.get("pct_neg"), Some(0));

    // Saturates: before=1, after=65535 -> diff=65534, 65534*100/1 = 6553400 > 65535, clamps.
    let cell = step(&[("before", 1), ("after", 65535)]);
    assert_eq!(cell.get("pct_mag"), Some(65535));
    assert_eq!(cell.get("pct_neg"), Some(0));

    // Over 100%: before=10000, after=20000 -> diff=10000, 10000*100/10000 = 100.
    let cell = step(&[("before", 10000), ("after", 20000)]);
    assert_eq!(cell.get("pct_mag"), Some(100));
    assert_eq!(cell.get("pct_neg"), Some(0));

    // No change: before=1000, after=1000 -> mag=0, neg forced 0 (not ambiguous).
    let cell = step(&[("before", 1000), ("after", 1000)]);
    assert_eq!(cell.get("pct_mag"), Some(0));
    assert_eq!(cell.get("pct_neg"), Some(0));
}

// Checks percent_round against hand-computed expectations: round(part*100/whole) with
// ties rounding up (via round_div's overflow-safe w-r tie test, done in u32 so the
// *100 scaling never wraps), 0 if whole == 0, saturating at 65535 -- the rounding-mode
// sibling of percent, which always floors.
#[test]
fn percent_round_cases() {
    let cases: &[(&str, &[u16], u16)] = &[
        ("percent_round", &[0, 100], 0),      // 0*100/100 = 0, no remainder to round
        ("percent_round", &[1, 3], 33),       // 100/3 = 33.333.. floors to 33 (not a tie)
        ("percent_round", &[1, 8], 13),       // 100/8 = 12.5 exactly, a genuine tie -> rounds up to 13
        ("percent_round", &[50, 0], 0),       // whole == 0 guards to 0
        ("percent_round", &[65535, 1], 65535), // 65535*100 = 6553500, saturates at u16::MAX
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
fn percent_ceil_matches_hand_computed_cases() {
    // percent_ceil(part, whole) = smallest integer p such that p*whole/100 >= part,
    // i.e. ceil(part*100/whole) via the q+1-if-remainder technique (div_ceil_u32's
    // convention). Checks the zero-part edge, an inexact case that rounds up, an
    // inexact case matching percent's own floor example (13 vs percent's floor of 12),
    // the whole==0 guard, saturation past 65535, and an exact (no-remainder) case where
    // ceil coincides with floor.
    let cases: &[(&str, &[u16], u16)] = &[
        ("percent_ceil", &[0, 100], 0),        // part == 0 -> smallest p is 0
        ("percent_ceil", &[1, 3], 34),         // ceil(100/3) = 34 (33 gives 99/100=0 < 1)
        ("percent_ceil", &[25, 200], 13),      // ceil(2500/200) = 13 (12 gives 2400/100=24 < 25)
        ("percent_ceil", &[5, 0], 0),          // whole == 0 -> guarded to 0
        ("percent_ceil", &[65535, 1], 65535),  // exact result 6553500 saturates to 65535
        ("percent_ceil", &[700, 1000], 70),    // exact, no remainder: ceil == floor == 70
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

// Checks scale_percent_i16 against hand-computed expectations: pct percent of a signed
// i16 value with sign preserved (mag*pct/100, saturated to i16's representable magnitude
// range -- 32767 for non-negative values, 32768 for negative values since i16::MIN's
// magnitude is 32768). Args/results cross the cell boundary as raw u16 two's-complement
// bit patterns, so a local closure converts host i16 <-> the u16 bits run_cell expects.
#[test]
fn scale_percent_i16_matches_defined_behaviour() {
    let bits = |v: i16| -> u16 { v as u16 };

    // 1) Basic positive: 50% of 200 = 100.
    assert_eq!(run_cell("scale_percent_i16", &[bits(200), 50]), bits(100));

    // 2) Basic negative, sign preserved: 50% of -200 = -100.
    assert_eq!(run_cell("scale_percent_i16", &[bits(-200), 50]), bits(-100));

    // 3) Zero value: 50% of 0 = 0.
    assert_eq!(run_cell("scale_percent_i16", &[bits(0), 50]), bits(0));

    // 4) Positive saturation: 200% of i16::MAX (32767) would be 65534, saturates to i16::MAX.
    assert_eq!(run_cell("scale_percent_i16", &[bits(i16::MAX), 200]), bits(i16::MAX));

    // 5) Negative saturation: 200% of i16::MIN would need magnitude 65536, saturates to
    //    magnitude 32768 -> i16::MIN exactly (the asymmetric cap is what makes this safe).
    assert_eq!(run_cell("scale_percent_i16", &[bits(i16::MIN), 200]), bits(i16::MIN));

    // 6) Truncation toward zero: 50% of 7 = 3.5 -> 3 (integer division truncates, not rounds).
    assert_eq!(run_cell("scale_percent_i16", &[bits(7), 50]), bits(3));

    // 7) Exact 100% boundary on i16::MIN: magnitude hits the cap exactly (32768 == 32768),
    //    so no saturation branch fires, yet the result is still correct and representable.
    assert_eq!(run_cell("scale_percent_i16", &[bits(i16::MIN), 100]), bits(i16::MIN));
}

#[test]
fn combined_percent_increase_matches_defined_behaviour() {
    // combined = pct_a + pct_b + pct_a*pct_b/100 (single equivalent rate for two
    // successive percent increases, derived from (1+a/100)(1+b/100)-1 in percent terms),
    // computed in u32 internally and saturating at 65535.
    let cases: &[(&str, &[u16], u16)] = &[
        ("combined_percent_increase", &[10, 20], 32), // 10+20+(10*20/100=2)=32
        ("combined_percent_increase", &[0, 0], 0), // no increase at all
        ("combined_percent_increase", &[100, 100], 300), // 100+100+(10000/100=100)=300
        ("combined_percent_increase", &[50, 50], 125), // 50+50+(2500/100=25)=125
        ("combined_percent_increase", &[65535, 65535], 65535), // true total 43,079,432 -> saturates
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
fn combined_percent_discount_hand_computed() {
    // Two successive percent discounts (pct_a then pct_b) collapse to one equivalent
    // discount rate: pct_a + pct_b - pct_a*pct_b/100, floored at 0, computed in u32
    // internally so the intermediate product never overflows u16.
    let check = |pct_a: u16, pct_b: u16| -> u16 {
        let mut r = cell80::Runner::compile(&std::fs::read_to_string(
            cell80::find_cell_file(
                &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cells"),
                "combined_percent_discount",
            )
            .unwrap(),
        ).unwrap())
        .unwrap();
        r.run(None, &[pct_a, pct_b], cell80::DEFAULT_CYCLES).unwrap().result
    };

    // 20% then 10%: 20+10-20*10/100 = 30-2 = 28 (not a naive 30% -- discounts compound).
    assert_eq!(check(20, 10), 28);

    // 0% then 0%: no discount at all.
    assert_eq!(check(0, 0), 0);

    // 50% then 50%: 50+50-2500/100 = 100-25 = 75, not 100 -- two 50% discounts never
    // reach "free".
    assert_eq!(check(50, 50), 75);

    // 1000% then 1000% (nonsense-large inputs exercising the floor): 2000 - 10000 goes
    // negative in real math -- floored at 0 rather than wrapping.
    assert_eq!(check(1000, 1000), 0);

    // 0% then 65535% (u16 domain extreme): sum=65535, product=0, reduction=0 -> 65535,
    // exactly u16::MAX -- confirms the result never overflows u16 for u16-bounded inputs
    // (provable: a+b-floor(a*b/100) is bounded by 65535 on the whole [0,65535]^2 box).
    assert_eq!(check(0, 65535), 65535);

    // 65535% then 1%: sum=65536, product=65535, reduction=floor(65535/100)=655,
    // result=65536-655=64881 -- still fits u16 even though the raw sum alone would not.
    assert_eq!(check(65535, 1), 64881);
}
