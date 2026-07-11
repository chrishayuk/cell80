//! Host-oracle tests for the money-bps pack (`cell80/cells/money-bps/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::cell_src;
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn money_bps_state_cells_match_defined_behaviour() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }

    // The GSM8K math-campaign money/basis-points pack (Phase 2.3, M1 pack 2/5) — basis
    // points, never float percentages, per the campaign spec.

    let (_, _, cell) = step("bps_of", "BpsOf", &[("value", 1000), ("bps", 500)]);
    assert_eq!(cell.get("result"), Some(50)); // 5% of 1000

    let (_, _, cell) = step(
        "increase_by_bps",
        "IncreaseByBps",
        &[("value", 1000), ("bps", 500)],
    );
    assert_eq!(cell.get("result"), Some(1050));

    let (_, _, cell) = step(
        "decrease_by_bps",
        "DecreaseByBps",
        &[("value", 1000), ("bps", 500)],
    );
    assert_eq!(cell.get("result"), Some(950));

    // The reverse-percent pair recovers the original value exactly.
    let (_, _, cell) = step(
        "original_before_bps_increase",
        "OriginalBeforeIncrease",
        &[("final_value", 1050), ("bps", 500)],
    );
    assert_eq!(cell.get("original"), Some(1000));
    let (_, _, cell) = step(
        "original_before_bps_decrease",
        "OriginalBeforeDecrease",
        &[("final_value", 950), ("bps", 500)],
    );
    assert_eq!(cell.get("original"), Some(1000));
    // bps == 10000 (100% discount) escalates rather than dividing by zero.
    let (_, report, _) = step(
        "original_before_bps_decrease",
        "OriginalBeforeDecrease",
        &[("final_value", 950), ("bps", 10000)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    let (_, _, cell) = step(
        "cents_mul_qty",
        "CentsMulQty",
        &[("unit_cents", 150), ("qty", 3)],
    );
    assert_eq!(cell.get("total"), Some(450));
}

#[test]
fn money_bps_wave2_cells_match_defined_behaviour() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }

    // GSM8K math-campaign money/bps pack, second slice: the missing inverse — recovering
    // the *rate* from a before/after pair, complementing increase_by_bps/decrease_by_bps
    // (rate -> final value) and original_before_bps_increase/decrease (rate + final ->
    // original). Checked docs/cell-index.md first: cents_div_qty/change_due-style
    // candidates were already considered and rejected as duplicates of
    // div_floor_u32/sub_checked_u32 (docs/library-growth.md, M1 pack 2/5) — not repeated.

    let (_, _, cell) = step(
        "bps_increase_between",
        "BpsIncreaseBetween",
        &[("before", 200), ("after", 250)],
    );
    assert_eq!(cell.get("bps"), Some(2500)); // 25% increase
    let (_, report, _) = step(
        "bps_increase_between",
        "BpsIncreaseBetween",
        &[("before", 250), ("after", 200)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06)); // after < before

    let (_, _, cell) = step(
        "bps_decrease_between",
        "BpsDecreaseBetween",
        &[("before", 200), ("after", 150)],
    );
    assert_eq!(cell.get("bps"), Some(2500)); // 25% decrease
    let (_, report, _) = step(
        "bps_decrease_between",
        "BpsDecreaseBetween",
        &[("before", 150), ("after", 200)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06)); // after > before
}

// Checked against bps_increase_between/bps_decrease_between (each requires a fixed
// direction and halts on the other); bps_change_between accepts either direction and
// reports the rate as a sign-magnitude pair instead.
#[test]
fn bps_change_between_matches_defined_behaviour() {
    fn step(fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src("bps_change_between"), "BpsChangeBetween", None)
            .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // Rose: before=200, after=250 -> +25% = 2500 bps, neg=0 (same magnitude
    // bps_increase_between would report for this pair).
    let (_, cell) = step(&[("before", 200), ("after", 250)]);
    assert_eq!(cell.get("bps_mag"), Some(2500));
    assert_eq!(cell.get("bps_neg"), Some(0));

    // Fell: before=200, after=150 -> -25% = 2500 bps, neg=1 (same magnitude
    // bps_decrease_between would report for this pair) -- one cell handles both.
    let (_, cell) = step(&[("before", 200), ("after", 150)]);
    assert_eq!(cell.get("bps_mag"), Some(2500));
    assert_eq!(cell.get("bps_neg"), Some(1));

    // No change: mag=0, and neg is forced to 0 rather than left ambiguous.
    let (_, cell) = step(&[("before", 1000), ("after", 1000)]);
    assert_eq!(cell.get("bps_mag"), Some(0));
    assert_eq!(cell.get("bps_neg"), Some(0));

    // before == 0 is out of domain (a base of zero has no defined rate), regardless
    // of direction.
    let (report, _) = step(&[("before", 0), ("after", 100)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // A large enough diff overflows the *10000 scale step and escalates rather than
    // silently wrapping.
    let (report, _) = step(&[("before", 1), ("after", 500000)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn compound_increase_by_bps_matches_defined_behaviour() {
    fn step(fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("compound_increase_by_bps"),
            "CompoundIncreaseByBps",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // 10% growth compounded for 3 periods: 1000 -> 1100 -> 1210 -> 1331 (like increase_by_bps
    // applied three times in a row, but looped internally rather than called three times).
    let (_, cell) = step(&[("value", 1000), ("bps", 1000), ("periods", 3)]);
    assert_eq!(cell.get("result"), Some(1331));

    // periods = 0 is a no-op: the while-loop body never runs, value passes through unchanged.
    let (_, cell) = step(&[("value", 500), ("bps", 250), ("periods", 0)]);
    assert_eq!(cell.get("result"), Some(500));

    // bps = 10000 (100% growth) doubles every period: 1 -> 2 -> 4 -> 8 -> 16.
    let (_, cell) = step(&[("value", 1), ("bps", 10000), ("periods", 4)]);
    assert_eq!(cell.get("result"), Some(16));

    // Multiply overflow: value * bps itself exceeds u32::MAX on the first iteration.
    let (report, _) = step(&[("value", 4_000_000_000), ("bps", 5000), ("periods", 1)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // Add-only overflow: value=4294967290, bps=1 keeps the multiply (product=4294967290)
    // safely inside u32, but value+delta=4295396786 overflows u32::MAX -- proves the checked
    // add, not just the checked multiply, is load-bearing every iteration.
    let (report, _) = step(&[("value", 4_294_967_290), ("bps", 1), ("periods", 1)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn compound_decrease_by_bps_matches_hand_computed_expectations() {
    use crate::common::cell_src;
    use cell80::{StateCell, DEFAULT_CYCLES};

    // Checked against decrease_by_bps (single application) and compound_increase_by_bps
    // (the opposite-direction sibling): compounds the same bps discount rate over
    // `periods` iterations, escalating if any step's discount would exceed the running value.
    fn step(value: u64, bps: u64, periods: u64) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("compound_decrease_by_bps"),
            "CompoundDecreaseByBps",
            None,
        )
        .unwrap_or_else(|e| panic!("bind compound_decrease_by_bps: {e}"));
        cell.set("value", value).unwrap();
        cell.set("bps", bps).unwrap();
        cell.set("periods", periods).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // value=1000, bps=1000 (10%), periods=3: 1000 -100=900; 900 -90=810; 810 -81=729.
    let (_, cell) = step(1000, 1000, 3);
    assert_eq!(cell.get("result"), Some(729));

    // periods=0 is the identity: result == value unchanged.
    let (_, cell) = step(12345, 250, 0);
    assert_eq!(cell.get("result"), Some(12345));

    // periods=1 matches a single decrease_by_bps application: 2000 - (2000*750/10000=150) = 1850.
    let (_, cell) = step(2000, 750, 1);
    assert_eq!(cell.get("result"), Some(1850));

    // bps > 10000 (would decrease past zero): value=1, bps=20000 ->
    // product=20000, delta=floor(20000/10000)=2, 2 > 1 -> halt 0xFF05.
    let (report, _) = step(1, 20000, 1);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // Multiply overflow inside a step: value=900_000_000, bps=10000 ->
    // product = 9_000_000_000_000, far past u32::MAX -> mul_checked_u32 halts 0xFF05.
    let (report, _) = step(900_000_000, 10000, 1);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // Two periods at 25%: 800 -200=600; 600 -150=450.
    let (_, cell) = step(800, 2500, 2);
    assert_eq!(cell.get("result"), Some(450));
}

#[test]
fn whole_from_bps_of_matches_hand_computed_expectations() {
    // Checked against bps_of (the pack's forward cell: value*bps/10000): recovers the
    // whole value from a known bps-portion (`part`) and the same rate, the one inverse
    // bps_of never got even though increase_by_bps/decrease_by_bps both did
    // (original_before_bps_increase/decrease).
    fn step(part: u64, bps: u64) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src("whole_from_bps_of"), "WholeFromBpsOf", None)
            .unwrap_or_else(|e| panic!("bind whole_from_bps_of: {e}"));
        cell.set("part", part).unwrap();
        cell.set("bps", bps).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // Round trip with bps_of: value=1000, bps=500 (5%) -> part=50; recovering:
    // value = 50 * 10000 / 500 = 500000 / 500 = 1000.
    let (_, cell) = step(50, 500);
    assert_eq!(cell.get("value"), Some(1000));

    // part = 0 is a legitimate zero-portion input: product=0, value=0/bps=0.
    let (_, cell) = step(0, 500);
    assert_eq!(cell.get("value"), Some(0));

    // bps == 0 is out of domain regardless of part (dividing by a zero rate is undefined).
    let (report, _) = step(100, 0);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // Overflow: part=429497 -> part*10000 = 4,294,970,000, which exceeds u32::MAX
    // (4,294,967,295) by 2704, so it wraps to 2704 mod 2^32; 2704/10000 = 0 != 429497,
    // tripping the divide-back-and-compare check shared with original_before_bps_increase/decrease.
    let (report, _) = step(429_497, 500);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // Floor division: part=33, bps=700 -> product=330000, 330000/700 = 471.428.. -> 471.
    let (_, cell) = step(33, 700);
    assert_eq!(cell.get("value"), Some(471));
}

#[test]
fn compound_original_before_increase_hand_computed() {
    // Checked against compound_increase_by_bps (the forward loop this reverses) and
    // original_before_bps_increase (the single-step reverse this generalizes to N periods).
    fn step(final_value: u64, bps: u64, periods: u64) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("compound_original_before_increase"),
            "CompoundOriginalBeforeIncrease",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("final_value", final_value).unwrap();
        cell.set("bps", bps).unwrap();
        cell.set("periods", periods).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // 1. Single period, matches original_before_bps_increase exactly:
    // 1050 * 10000 / 10500 = 10500000 / 10500 = 1000.
    let (_, cell) = step(1050, 500, 1);
    assert_eq!(cell.get("original"), Some(1000));

    // 2. Three periods at 10% (1000 -> 1100 -> 1210 -> 1331 forward), reversed exactly:
    // 1331*10000/11000=1210, 1210*10000/11000=1100, 1100*10000/11000=1000.
    let (_, cell) = step(1331, 1000, 3);
    assert_eq!(cell.get("original"), Some(1000));

    // 3. periods=0 is a no-op: the loop never runs, original passes through unchanged.
    let (_, cell) = step(777, 250, 0);
    assert_eq!(cell.get("original"), Some(777));

    // 4. Two periods at 5%, exercising floor division at each step (not an exact
    // round trip of any forward compounding): step1 = 1000*10000/10500 = 952
    // (952*10500=9,996,000, remainder 4000); step2 = 952*10000/10500 = 906
    // (10500*906=9,513,000, remainder 7000, since 9,520,000-9,513,000=7000).
    let (_, cell) = step(1000, 500, 2);
    assert_eq!(cell.get("original"), Some(906));

    // 5. Overflow on the first iteration: 429497 * 10000 = 4,294,970,000, which
    // exceeds u32::MAX (4,294,967,295), wraps, and fails the divide-back check.
    let (report, _) = step(429_497, 500, 1);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn bps_rate_over_2_periods_matches_hand_computed_cases() {
    // Checked against bps_increase_between (recovers a rate over exactly 1 period) and
    // compound_increase_by_bps (applies a KNOWN rate for N periods): this recovers an
    // UNKNOWN constant per-period rate from a before/after pair spanning exactly 2
    // compounding periods, r = sqrt(after/before)-1, via a wide isqrt on the doubly-scaled
    // ratio -- needs isqrt_u32's Nth-root technique, unbuildable before it landed.
    fn step(before: u64, after: u64) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("bps_rate_over_2_periods"),
            "BpsRateOver2Periods",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("before", before).unwrap();
        cell.set("after", after).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // 100 -> 144 over 2 periods: 1.2^2 = 1.44, so each period grew 20% = 2000 bps.
    // temp = 144*10000/100 = 14400; scaled = 14400*10000 = 144_000_000; isqrt = 12000; bps = 2000.
    let (_, cell) = step(100, 144);
    assert_eq!(cell.get("bps"), Some(2000), "100->144");

    // 10000 -> 12100 over 2 periods: 1.1^2 = 1.21, so each period grew 10% = 1000 bps.
    // temp = 12100*10000/10000 = 12100; scaled = 121_000_000; isqrt = 11000; bps = 1000.
    let (_, cell) = step(10000, 12100);
    assert_eq!(cell.get("bps"), Some(1000), "10000->12100");

    // before == after: no growth at all across 2 periods -> 0 bps.
    let (_, cell) = step(777, 777);
    assert_eq!(cell.get("bps"), Some(0), "777->777");

    // before == 0 is out of domain regardless of after.
    let (report, _) = step(0, 100);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06), "before=0");

    // after < before is out of domain (this cell only recovers a growth rate).
    let (report, _) = step(200, 100);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06), "after<before");

    // Overflow: after*10000 alone blows past u32::MAX (500000*10000 = 5_000_000_000).
    let (report, _) = step(1, 500000);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05), "overflow");
}

#[test]
fn combined_bps_increase_composes_two_sequential_markups() {
    // combined_bps_increase folds two sequential bps increases (e.g. a markup then a
    // separately-stated tax) into one equivalent single rate: combined = bps1 + bps2 +
    // bps1*bps2/10000, i.e. (1+r1)(1+r2)-1 expressed in basis points. This is distinct from
    // compound_increase_by_bps, which loops the SAME rate N times rather than composing two
    // different rates.
    let run = |bps1: u16, bps2: u16| {
        let mut r = cell80::Runner::compile(&cell_src("combined_bps_increase"))
            .unwrap_or_else(|e| panic!("compile: {e}"));
        r.run(None, &[bps1, bps2], DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run: {e}"))
    };

    // 5% markup then 3% tax: (1.05)(1.03)-1 = 0.0815 -> 815 bps.
    // cross = 500*300/10000 = 15; total = 500+300+15 = 815.
    assert_eq!(run(500, 300).result, 815);

    // No increase at all composes to no increase.
    assert_eq!(run(0, 0).result, 0);

    // Doubling then doubling again: (2)(2)-1 = 3 -> 30000 bps.
    // cross = 10000*10000/10000 = 10000; total = 10000+10000+10000 = 30000.
    assert_eq!(run(10000, 10000).result, 30000);

    // Arbitrary mid-range rates: cross = 1234*567/10000 = 699678/10000 = 69 (floor).
    // total = 1234 + 567 + 69 = 1870.
    assert_eq!(run(1234, 567).result, 1870);

    // Large rates whose composed total exceeds u16 range escalate rather than wrap:
    // cross = 60000*60000/10000 = 360000; total = 60000+60000+360000 = 480000 > 65535.
    let report = run(60000, 60000);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn combined_bps_decrease_matches_hand_computed() {
    // combined = bps1 + bps2 - bps1*bps2/10000, i.e. 1 - (1-r1)(1-r2) expressed in bps --
    // composing two DIFFERENT successive discount rates into one equivalent rate (distinct
    // from compound_decrease_by_bps, which loops the SAME rate N times).
    fn run_cell(id: &str, args: &[u16]) -> u16 {
        let mut r =
            cell80::Runner::compile(&cell_src(id)).unwrap_or_else(|e| panic!("compile {id}: {e}"));
        r.run(None, args, cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run {id}: {e}"))
            .result
    }

    // 10% then 20%: 0.9*0.8=0.72 remains -> 2800 bps combined discount.
    assert_eq!(run_cell("combined_bps_decrease", &[1000, 2000]), 2800);
    // 0% then 50%: only the 50% survives.
    assert_eq!(run_cell("combined_bps_decrease", &[0, 5000]), 5000);
    // 100% then 100%: correctly caps at 10000 (fully discounted), not 20000.
    assert_eq!(run_cell("combined_bps_decrease", &[10000, 10000]), 10000);
    // 50% then 50%: 0.5*0.5=0.25 remains -> 7500 bps combined discount.
    assert_eq!(run_cell("combined_bps_decrease", &[5000, 5000]), 7500);
    // 25% then 3%: 0.75*0.97=0.7275 remains -> 2725 bps combined discount.
    assert_eq!(run_cell("combined_bps_decrease", &[2500, 300]), 2725);

    // Out-of-domain: bps1 > 10000 halts rather than silently producing nonsense.
    let mut r = cell80::Runner::compile(&cell_src("combined_bps_decrease")).unwrap();
    let report = r.run(None, &[10001, 0], cell80::DEFAULT_CYCLES).unwrap();
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}
