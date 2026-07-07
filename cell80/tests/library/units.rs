//! Host-oracle tests for the units pack (`cell80/cells/units/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::cell_src;
use cell80::{Runner, DEFAULT_CYCLES};

#[test]
fn units_free_fn_cells_match_defined_behaviour() {
    fn report(id: &str, args: &[u16]) -> cell80::Report {
        let mut r = Runner::compile(&cell_src(id)).unwrap_or_else(|e| panic!("compile {id}: {e}"));
        r.run(None, args, DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run {id}: {e}"))
    }

    // same_unit_check: matching units echo the shared code; mismatched units escalate.
    assert_eq!(report("same_unit_check", &[1, 1]).result, 1); // money == money
    assert_eq!(
        report("same_unit_check", &[1, 2]).halt,
        cell80::Halt::Escalate(0xFF06)
    );

    // unit_mul: count*money=money, distance*distance=area, area*distance=volume,
    // rate_money_per_count*count=money, rate_distance_per_time*time=distance.
    assert_eq!(report("unit_mul", &[0, 1]).result, 1);
    assert_eq!(report("unit_mul", &[3, 3]).result, 4);
    assert_eq!(report("unit_mul", &[4, 3]).result, 5);
    assert_eq!(report("unit_mul", &[6, 0]).result, 1);
    assert_eq!(report("unit_mul", &[7, 2]).result, 3);
    assert_eq!(
        report("unit_mul", &[1, 1]).halt,
        cell80::Halt::Escalate(0xFF06)
    ); // money*money is unmodeled

    // unit_div: money/count=rate_money_per_count, distance/time=rate_distance_per_time,
    // and same/same always cancels to a plain count.
    assert_eq!(report("unit_div", &[1, 0]).result, 6);
    assert_eq!(report("unit_div", &[3, 2]).result, 7);
    assert_eq!(report("unit_div", &[5, 5]).result, 0);
    assert_eq!(
        report("unit_div", &[2, 1]).halt,
        cell80::Halt::Escalate(0xFF06)
    ); // time/money is unmodeled

    // unit_cancel_check: a non-escalating boolean mirror of unit_div's domain table.
    assert_eq!(report("unit_cancel_check", &[1, 0]).result, 1);
    assert_eq!(report("unit_cancel_check", &[2, 1]).result, 0);
    assert_eq!(report("unit_cancel_check", &[100, 4]).result, 0); // out-of-domain codes too

    // Wage-rate pair (code 8, rate_money_per_time): money/time=rate, rate*time=money.
    assert_eq!(report("unit_div", &[1, 2]).result, 8);
    assert_eq!(report("unit_mul", &[8, 2]).result, 1);
    assert_eq!(report("unit_mul", &[2, 8]).result, 1);
    assert_eq!(report("unit_cancel_check", &[1, 2]).result, 1);
    assert_eq!(report("same_unit_check", &[8, 8]).result, 8);

    // Production-rate pair (code 9, rate_count_per_time): count/time=rate, rate*time=count.
    assert_eq!(report("unit_div", &[0, 2]).result, 9);
    assert_eq!(report("unit_mul", &[9, 2]).result, 0);
    assert_eq!(report("unit_mul", &[2, 9]).result, 0);
    assert_eq!(report("unit_cancel_check", &[0, 2]).result, 1);
    assert_eq!(report("same_unit_check", &[9, 9]).result, 9);

    // The bound check now rejects codes > 9, not > 8.
    assert_eq!(
        report("same_unit_check", &[10, 10]).halt,
        cell80::Halt::Escalate(0xFF06)
    );
    // The GSM8K math-campaign units pack (Phase 2.3, M1 pack 3/5) — dimension codes
    // 0=count,1=money,2=time,3=distance,4=area,5=volume,6=rate_money_per_count,
    // 7=rate_distance_per_time,8=rate_money_per_time,9=rate_count_per_time
    // (docs/library-growth.md; codes 8/9 added later to cover wage-rate — "$ per hour" —
    // and production-rate — "N per hour" — word problems, both previously unmodeled).
    // Free-fn cells (no u32 state needed), escalating via 0xFF06 (out_of_domain) rather
    // than 0xFF05 (needs_wider_math) — a mismatched/unmodeled unit pair isn't a wide-math
    // problem.
}
