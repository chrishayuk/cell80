//! Host-oracle tests for the agentic-runtime pack (`cell80/cells/agentic-runtime/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::cell_src;
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn agentic_runtime_state_cells_match_defined_behaviour() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let result = cell.run(DEFAULT_CYCLES).unwrap().result;
        (result, cell)
    }

    // token_bucket_step: refill then spend; still refills (but doesn't go negative) on denial.
    let (allowed, cell) = step(
        "token_bucket_step",
        "TokenBucket",
        &[("tokens", 5), ("capacity", 10), ("refill", 2), ("cost", 3)],
    );
    assert_eq!(allowed, 1);
    assert_eq!(cell.get("tokens"), Some(4)); // (5+2) capped at 10 = 7, minus cost 3
    let (allowed, cell) = step(
        "token_bucket_step",
        "TokenBucket",
        &[("tokens", 1), ("capacity", 10), ("refill", 0), ("cost", 5)],
    );
    assert_eq!(allowed, 0);
    assert_eq!(cell.get("tokens"), Some(1)); // denied — tokens unchanged, not spent

    // backoff_next: doubles, capped, without overflowing past the cap.
    assert_eq!(
        step("backoff_next", "Backoff", &[("current", 0), ("cap", 100)]).0,
        1
    );
    assert_eq!(
        step(
            "backoff_next",
            "Backoff",
            &[("current", 100), ("cap", 10_000)]
        )
        .0,
        200
    );
    assert_eq!(
        step(
            "backoff_next",
            "Backoff",
            &[("current", 40_000), ("cap", 65_535)]
        )
        .0,
        65_535 // would overflow if doubled naively (80,000 wraps to 14,464)
    );

    // circuit_breaker_step: closed -> open -> half-open -> closed/open.
    let (state, _) = step(
        "circuit_breaker_step",
        "CircuitBreaker",
        &[
            ("state", 0),
            ("fail_count", 2),
            ("fail_threshold", 3),
            ("cooldown_elapsed", 0),
            ("success", 0),
        ],
    );
    assert_eq!(state, 1); // 3rd consecutive failure opens the breaker
    let (state, _) = step(
        "circuit_breaker_step",
        "CircuitBreaker",
        &[
            ("state", 1),
            ("fail_count", 3),
            ("fail_threshold", 3),
            ("cooldown_elapsed", 1),
            ("success", 0),
        ],
    );
    assert_eq!(state, 2); // cooldown elapsed -> try half-open
    let (state, cell) = step(
        "circuit_breaker_step",
        "CircuitBreaker",
        &[
            ("state", 2),
            ("fail_count", 3),
            ("fail_threshold", 3),
            ("cooldown_elapsed", 0),
            ("success", 1),
        ],
    );
    assert_eq!(state, 0); // half-open trial succeeded -> closed
    assert_eq!(cell.get("fail_count"), Some(0));
    let (state, _) = step(
        "circuit_breaker_step",
        "CircuitBreaker",
        &[
            ("state", 2),
            ("fail_count", 0),
            ("fail_threshold", 3),
            ("cooldown_elapsed", 0),
            ("success", 0),
        ],
    );
    assert_eq!(state, 1); // half-open trial failed -> back to open

    // debounce_step: three consistent readings needed to confirm a change.
    let (mut count, mut last_stable) = (0u64, 0u64);
    for _ in 0..2 {
        let (out, cell) = step(
            "debounce_step",
            "Debounce",
            &[
                ("input", 1),
                ("last_stable", last_stable),
                ("count", count),
                ("threshold", 3),
            ],
        );
        assert_eq!(out, 0); // not yet confirmed
        count = cell.get("count").unwrap();
        last_stable = cell.get("last_stable").unwrap();
    }
    let (out, _) = step(
        "debounce_step",
        "Debounce",
        &[
            ("input", 1),
            ("last_stable", last_stable),
            ("count", count),
            ("threshold", 3),
        ],
    );
    assert_eq!(out, 1); // 3rd consistent reading confirms the change

    // hysteresis: dead zone holds the prior state.
    assert_eq!(
        step(
            "hysteresis",
            "Hysteresis",
            &[("value", 80), ("low", 20), ("high", 70), ("state", 0)]
        )
        .0,
        1
    );
    assert_eq!(
        step(
            "hysteresis",
            "Hysteresis",
            &[("value", 50), ("low", 20), ("high", 70), ("state", 1)]
        )
        .0,
        1
    ); // dead zone, holds ON
    assert_eq!(
        step(
            "hysteresis",
            "Hysteresis",
            &[("value", 10), ("low", 20), ("high", 70), ("state", 1)]
        )
        .0,
        0
    );
    assert_eq!(
        step(
            "hysteresis",
            "Hysteresis",
            &[("value", 50), ("low", 20), ("high", 70), ("state", 0)]
        )
        .0,
        0
    ); // dead zone, holds OFF
       // Rate-limiting / resilience state machines (wave 3): each call sets fields by name,
       // runs one step, and reads the mutated state back — the host is responsible for
       // re-feeding the updated fields as the next call's inputs.
}

#[test]
fn library_growth_backlog_agentic_runtime_slice() {
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

    // rate_window_update: limit 2 per 100-tick window; 3rd request in-window denied;
    // a request past the window rolls over and is allowed again.
    let (allowed, _, cell) = step(
        "rate_window_update",
        "RateWindowUpdate",
        &[
            ("now", 10),
            ("window_start", 0),
            ("window_size", 100),
            ("count", 0),
            ("limit", 2),
        ],
    );
    assert_eq!(allowed, 1);
    let (allowed, _, cell) = step(
        "rate_window_update",
        "RateWindowUpdate",
        &[
            ("now", 20),
            ("window_start", cell.get("window_start").unwrap()),
            ("window_size", 100),
            ("count", cell.get("count").unwrap()),
            ("limit", 2),
        ],
    );
    assert_eq!(allowed, 1);
    let (allowed, _, cell) = step(
        "rate_window_update",
        "RateWindowUpdate",
        &[
            ("now", 30),
            ("window_start", cell.get("window_start").unwrap()),
            ("window_size", 100),
            ("count", cell.get("count").unwrap()),
            ("limit", 2),
        ],
    );
    assert_eq!(allowed, 0); // 3rd request this window, denied
    let (allowed, _, cell) = step(
        "rate_window_update",
        "RateWindowUpdate",
        &[
            ("now", 200), // past window_start(0) + window_size(100): rolls over
            ("window_start", cell.get("window_start").unwrap()),
            ("window_size", 100),
            ("count", cell.get("count").unwrap()),
            ("limit", 2),
        ],
    );
    assert_eq!(allowed, 1);
    assert_eq!(cell.get("window_start"), Some(200));
    assert_eq!(cell.get("count"), Some(1));
    let (_, report, _) = step(
        "rate_window_update",
        "RateWindowUpdate",
        &[
            ("now", 5),
            ("window_start", 10),
            ("window_size", 100),
            ("count", 0),
            ("limit", 2),
        ],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06)); // time moved backward
}

#[test]
fn wave4_agentic_runtime_reflexes_agentic_runtime_slice() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> u16 {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    fn step_cell(id: &str, strct: &str, fields: &[(&str, u64)]) -> StateCell {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap();
        cell
    }

    // cooldown_step: still counting down; reaches ready exactly at 0; already-ready stays 0.
    let cell = step_cell("cooldown_step", "CooldownStep", &[("cooldown", 3)]);
    assert_eq!(cell.get("cooldown"), Some(2));
    assert_eq!(cell.get("ready"), Some(0));
    let cell = step_cell("cooldown_step", "CooldownStep", &[("cooldown", 1)]);
    assert_eq!(cell.get("cooldown"), Some(0));
    assert_eq!(cell.get("ready"), Some(1));
    let cell = step_cell("cooldown_step", "CooldownStep", &[("cooldown", 0)]);
    assert_eq!(cell.get("cooldown"), Some(0));
    assert_eq!(cell.get("ready"), Some(1));

    // epsilon_greedy_pick3: below the exploration threshold -> alt_idx; at/above -> best_idx.
    assert_eq!(
        step(
            "epsilon_greedy_pick3",
            "EpsilonGreedyPick3",
            &[
                ("rand_bps", 500),
                ("epsilon_bps", 1000),
                ("best_idx", 7),
                ("alt_idx", 2)
            ]
        ),
        2
    );
    assert_eq!(
        step(
            "epsilon_greedy_pick3",
            "EpsilonGreedyPick3",
            &[
                ("rand_bps", 1500),
                ("epsilon_bps", 1000),
                ("best_idx", 7),
                ("alt_idx", 2)
            ]
        ),
        7
    );

    // Equivalence check backing the decision not to ship retry_budget_step/
    // budget_spend_step: token_bucket_step with refill=0 and capacity >= tokens IS a
    // plain "spend from a budget, report allowed" cell — confirmed directly rather than
    // assumed, the same discipline the admission gate automates.
    let cell = step_cell(
        "token_bucket_step",
        "TokenBucket",
        &[
            ("tokens", 100),
            ("capacity", 100),
            ("refill", 0),
            ("cost", 30),
        ],
    );
    assert_eq!(cell.get("tokens"), Some(70)); // spent
    assert_eq!(cell.get("allowed"), Some(1));
    let cell = step_cell(
        "token_bucket_step",
        "TokenBucket",
        &[
            ("tokens", 20),
            ("capacity", 100),
            ("refill", 0),
            ("cost", 30),
        ],
    );
    assert_eq!(cell.get("tokens"), Some(20)); // unchanged, denied
    assert_eq!(cell.get("allowed"), Some(0));
    // Wave 4, slice 5: agentic-runtime reflexes — cell80's standing "agent reflexes, not
    // just math" priority, independent of the GSM8K campaign. retry_budget_step and
    // budget_spend_step from the original ~100-cell proposal were verified (not
    // assumed) to be behaviourally identical to the already-shipped token_bucket_step
    // called with refill=0 and a capacity >= tokens — see the equivalence check below —
    // so neither was shipped as a separate cell. ucb1_score_q8 was not attempted: UCB1
    // needs a fixed-point ln the dialect has no primitive for (the same class of gap
    // cosine_score_approx is still blocked on).
}

#[test]
fn difficulty_zone_step_matches_defined_behaviour() {
    // Mined from chuk-math-gym's curriculum-scheduling strategy: a 3-way advance/hold/
    // retreat decision from an accuracy tally against a target+-tolerance band, gated by
    // a minimum sample count — exact via cross-multiplication, no accuracy ratio computed.
    // A state cell purely for arg count (5 named fields), not because it remembers
    // anything itself — distinct from hysteresis's raw single-value 2-state latch.
    fn step(correct: u16, total: u16, target: u16, tolerance: u16, min_problems: u16) -> u16 {
        let mut cell = StateCell::bind(
            &cell_src("difficulty_zone_step"),
            "DifficultyZoneStep",
            None,
        )
        .unwrap();
        cell.set("correct", correct as u64).unwrap();
        cell.set("total", total as u64).unwrap();
        cell.set("target_pct", target as u64).unwrap();
        cell.set("tolerance_pct", tolerance as u64).unwrap();
        cell.set("min_problems", min_problems as u64).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    assert_eq!(step(5, 5, 75, 10, 10), 1); // not enough samples yet -> hold
    assert_eq!(step(9, 10, 75, 10, 5), 2); // 90% vs band 65-85 -> advance
    assert_eq!(step(5, 10, 75, 10, 5), 0); // 50% vs band 65-85 -> retreat
    assert_eq!(step(8, 10, 75, 10, 5), 1); // 80% vs band 65-85 -> hold
    assert_eq!(step(17, 20, 75, 10, 5), 1); // exactly 85% is not > 85 (strict) -> hold
}

#[test]
fn token_bucket_step_u32_matches_hand_computed() {
    // Wide/checked sibling of token_bucket_step: same refill-cap-spend formula but at u32
    // width with a checked refill add (escalates 0xFF05 on overflow instead of wrapping).
    fn step(fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src("token_bucket_step_u32"), "TokenBucketU32", None)
            .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // Sanity vs. the u16 sibling: identical numbers, identical result.
    // refilled = 5+2=7, capped at 10 -> 7, ok = 7>=3 true, tokens = 7-3=4, allowed=1
    let (report, cell) = step(&[("tokens", 5), ("capacity", 10), ("refill", 2), ("cost", 3)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(report.result, 1);
    assert_eq!(cell.get("tokens"), Some(4));

    // Denied: tokens still refill (up to cap) even when the spend is rejected.
    // refilled = 1+0=1, capped=1, ok=1>=5 false, tokens stays at capped=1, allowed=0
    let (report, cell) = step(&[("tokens", 1), ("capacity", 10), ("refill", 0), ("cost", 5)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(report.result, 0);
    assert_eq!(cell.get("tokens"), Some(1));

    // Wide-only values (beyond u16::MAX), cap kicks in before the spend.
    // refilled = 100_000+150_000=250_000, capped at 200_000 -> 200_000,
    // ok = 200_000>=50_000 true, tokens = 200_000-50_000=150_000, allowed=1
    let (report, cell) = step(&[
        ("tokens", 100_000),
        ("capacity", 200_000),
        ("refill", 150_000),
        ("cost", 50_000),
    ]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(report.result, 1);
    assert_eq!(cell.get("tokens"), Some(150_000));

    // Denied even after refilling to the cap.
    // refilled = 0+1000=1000, capped at 1000 (== capacity) -> 1000,
    // ok = 1000>=2000 false, tokens = capped = 1000, allowed=0
    let (report, cell) = step(&[
        ("tokens", 0),
        ("capacity", 1000),
        ("refill", 1000),
        ("cost", 2000),
    ]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(report.result, 0);
    assert_eq!(cell.get("tokens"), Some(1000));

    // Refill overflow escalates (checked add) instead of silently wrapping.
    // tokens = u32::MAX, refill = 1 -> add_checked_u32 halts 0xFF05 before capping/spending.
    let (report, _cell) = step(&[
        ("tokens", u32::MAX as u64),
        ("capacity", u32::MAX as u64),
        ("refill", 1),
        ("cost", 0),
    ]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn linear_backoff_next_matches_defined_behaviour() {
    // Additive-growth backoff: next = min(current + step, cap), starting at `step` when
    // current is 0 (no special-case needed — 0 + step already is step). Distinct from
    // backoff_next's doubling growth. Also checks saturating_add guards against u16
    // wraparound when current + step would overflow before the cap is applied.
    fn step_it(current: u16, step: u16, cap: u16) -> u16 {
        let mut cell = StateCell::bind(&cell_src("linear_backoff_next"), "LinearBackoff", None)
            .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("current", current as u64).unwrap();
        cell.set("step", step as u64).unwrap();
        cell.set("cap", cap as u64).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    assert_eq!(step_it(0, 50, 1000), 50); // starts at step when current is 0
    assert_eq!(step_it(50, 50, 1000), 100); // plain additive growth
    assert_eq!(step_it(980, 50, 1000), 1000); // 980+50=1030, capped to 1000
    assert_eq!(step_it(1000, 50, 1000), 1000); // already at cap, stays at cap
    assert_eq!(step_it(65_530, 100, 65_535), 65_535); // 65530+100 overflows u16;
                                                      // saturating_add clamps before the
                                                      // cap check, no silent wraparound
}

#[test]
fn jittered_backoff_next_matches_defined_behaviour() {
    // jittered_backoff_next: capped-exponential ceiling (same rule as backoff_next),
    // then scaled down into [0, ceiling] by rand_bps/10000 via a u32 intermediate.
    fn step(current: u16, cap: u16, rand_bps: u16) -> u16 {
        let mut cell = StateCell::bind(&cell_src("jittered_backoff_next"), "JitteredBackoff", None)
            .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("current", current as u64).unwrap();
        cell.set("cap", cap as u64).unwrap();
        cell.set("rand_bps", rand_bps as u64).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // Bootstrap: current=0, cap=100 -> ceiling=1; rand_bps=5000 (50%) floors to 0.
    assert_eq!(step(0, 100, 5000), 0);
    // Mid-range growth: current=100, cap=10_000 -> ceiling=200 (100*2, under half the cap);
    // rand_bps=9999 (near-max) -> 200*9999/10000 = 199.
    assert_eq!(step(100, 10_000, 9999), 199);
    // Cap saturation: current=40_000 > cap/2, so ceiling clamps to cap=65_535;
    // rand_bps=5000 (50%) -> 65_535*5000/10000 = 32_767 (half the ceiling).
    assert_eq!(step(40_000, 65_535, 5000), 32_767);
    // rand_bps=0 always collapses to 0, regardless of how large the ceiling is.
    assert_eq!(step(100, 1000, 0), 0);
    // Same 65_535 ceiling as above but rand_bps=9999: 65_535*9999 = 655_284_465, which
    // overflows u16 (max 65_535) and would silently wrap if multiplied at 16-bit width --
    // this proves the u32 intermediate is load-bearing. 655_284_465/10000 = 65_528.
    assert_eq!(step(40_000, 65_535, 9999), 65_528);
}

// rising_edge_step: reports 1 only on the exact step the signal goes 0 -> 1; 0 on hold-high,
// hold-low, or the falling edge. prev is threaded step-to-step to simulate a live time series.
#[test]
fn rising_edge_step_fires_only_on_the_transition_to_one() {
    fn step(input: u64, prev: u64) -> (u16, StateCell) {
        let mut cell = StateCell::bind(&cell_src("rising_edge_step"), "RisingEdge", None)
            .unwrap_or_else(|e| panic!("bind rising_edge_step: {e}"));
        cell.set("input", input).unwrap();
        cell.set("prev", prev).unwrap();
        let result = cell.run(DEFAULT_CYCLES).unwrap().result;
        (result, cell)
    }

    // signal sequence: 0, 1, 1, 0, 1 -> edges fire only at the two 0->1 transitions.
    let mut prev = 0u64;
    let expected = [(0u64, 0u16), (1, 1), (1, 0), (0, 0), (1, 1)];
    for (input, want_edge) in expected {
        let (edge, cell) = step(input, prev);
        assert_eq!(edge, want_edge, "input={input} prev={prev}");
        prev = cell.get("prev").unwrap();
        assert_eq!(prev, input); // prev always tracks the last input after run()
    }
}

// falling_edge_step: reports 1 only on the exact step the signal goes 1 -> 0; 0 on hold-low,
// hold-high, or the rising edge. prev is threaded step-to-step to simulate a live time series,
// mirroring the rising_edge_step test above but for the opposite transition.
#[test]
fn falling_edge_step_fires_only_on_the_transition_to_zero() {
    fn step(input: u64, prev: u64) -> (u16, StateCell) {
        let mut cell = StateCell::bind(&cell_src("falling_edge_step"), "FallingEdge", None)
            .unwrap_or_else(|e| panic!("bind falling_edge_step: {e}"));
        cell.set("input", input).unwrap();
        cell.set("prev", prev).unwrap();
        let result = cell.run(DEFAULT_CYCLES).unwrap().result;
        (result, cell)
    }

    // signal sequence: 1, 1, 0, 0, 1, 0 -> edges fire only at the two 1->0 transitions.
    let mut prev = 0u64;
    let expected = [(1u64, 0u16), (1, 0), (0, 1), (0, 0), (1, 0), (0, 1)];
    for (input, want_edge) in expected {
        let (edge, cell) = step(input, prev);
        assert_eq!(edge, want_edge, "input={input} prev={prev}");
        prev = cell.get("prev").unwrap();
        assert_eq!(prev, input); // prev always tracks the last input after run()
    }
}

#[test]
fn jittered_linear_backoff_next_matches_defined_behaviour() {
    // jittered_linear_backoff_next: capped-additive ceiling (same rule as linear_backoff_next:
    // min(current+step, cap), starting at step when current is 0), then scaled down into
    // [0, ceiling] by rand_bps/10000 via a u32 intermediate -- the additive-growth dual of
    // jittered_backoff_next's exponential-ceiling scaling.
    fn step(current: u16, step: u16, cap: u16, rand_bps: u16) -> u16 {
        let mut cell = StateCell::bind(
            &cell_src("jittered_linear_backoff_next"),
            "JitteredLinearBackoff",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("current", current as u64).unwrap();
        cell.set("step", step as u64).unwrap();
        cell.set("cap", cap as u64).unwrap();
        cell.set("rand_bps", rand_bps as u64).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // Bootstrap: current=0 -> grown=step=50, under cap=1000 -> ceiling=50;
    // rand_bps=5000 (50%) -> 50*5000/10000 = 25.
    assert_eq!(step(0, 50, 1000, 5000), 25);
    // Plain additive growth: current=50, +step=50 -> ceiling=100, under cap=1000;
    // rand_bps=9999 (near-max) -> 100*9999/10000 = 99.
    assert_eq!(step(50, 50, 1000, 9999), 99);
    // Cap saturation: current=980, +step=50 = 1030 > cap=1000 -> ceiling clamps to 1000;
    // rand_bps=5000 (50%) -> 1000*5000/10000 = 500 (half the ceiling).
    assert_eq!(step(980, 50, 1000, 5000), 500);
    // rand_bps=0 always collapses to 0, regardless of how large the ceiling is.
    assert_eq!(step(100, 50, 1000, 0), 0);
    // current+step overflows u16 (65530+100=65630 > 65535) so saturating_add clamps to
    // 65535 before the cap check; ceiling = min(65535, cap=65535) = 65535. rand_bps=9999 ->
    // 65535*9999 = 655_284_465, which overflows u16 (max 65_535) and would silently wrap if
    // multiplied at 16-bit width -- proves the u32 intermediate is load-bearing.
    // 655_284_465/10000 = 65_528.
    assert_eq!(step(65_530, 100, 65_535, 9999), 65_528);
}

#[test]
fn agentic_runtime_watchdog_step_matches_defined_behaviour() {
    // Local helper: bind watchdog_step, set the given fields, run one step, return the cell.
    fn step_cell(fields: &[(&str, u64)]) -> StateCell {
        let mut cell = StateCell::bind(&cell_src("watchdog_step"), "WatchdogStep", None)
            .unwrap_or_else(|e| panic!("bind watchdog_step: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap();
        cell
    }

    // Missed heartbeats climb toward timeout without tripping yet.
    let cell = step_cell(&[("ticks", 0), ("timeout", 3), ("pet", 0), ("tripped", 0)]);
    assert_eq!(cell.get("ticks"), Some(1));
    assert_eq!(cell.get("tripped"), Some(0));

    // Reaching timeout exactly sets the sticky trip.
    let cell = step_cell(&[("ticks", 2), ("timeout", 3), ("pet", 0), ("tripped", 0)]);
    assert_eq!(cell.get("ticks"), Some(3));
    assert_eq!(cell.get("tripped"), Some(1));

    // Once at timeout, ticks is floored there (no wraparound) and the trip stays sticky
    // across further missed heartbeats.
    let cell = step_cell(&[("ticks", 3), ("timeout", 3), ("pet", 0), ("tripped", 1)]);
    assert_eq!(cell.get("ticks"), Some(3));
    assert_eq!(cell.get("tripped"), Some(1));

    // A pet signal always resets ticks to 0 and clears the trip, even mid-alarm.
    let cell = step_cell(&[("ticks", 3), ("timeout", 3), ("pet", 1), ("tripped", 1)]);
    assert_eq!(cell.get("ticks"), Some(0));
    assert_eq!(cell.get("tripped"), Some(0));

    // Edge case: timeout=0 trips immediately even with zero elapsed ticks.
    let cell = step_cell(&[("ticks", 0), ("timeout", 0), ("pet", 0), ("tripped", 0)]);
    assert_eq!(cell.get("tripped"), Some(1));
}

#[test]
fn circuit_breaker_trials_step_matches_defined_behaviour() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let result = cell.run(DEFAULT_CYCLES).unwrap().result;
        (result, cell)
    }

    // circuit_breaker_trials_step: like circuit_breaker_step but half-open needs N CONSECUTIVE
    // successes (any failure resets the tally and drops straight back to open) before closing.
    let (state, cell) = step(
        "circuit_breaker_trials_step",
        "CircuitBreakerTrials",
        &[
            ("state", 2),
            ("fail_count", 3),
            ("fail_threshold", 3),
            ("cooldown_elapsed", 0),
            ("success", 1),
            ("success_count", 0),
            ("success_threshold", 2),
        ],
    );
    assert_eq!(state, 2); // 1st of 2 required consecutive successes: stays half-open
    assert_eq!(cell.get("success_count"), Some(1));
    let (state, cell) = step(
        "circuit_breaker_trials_step",
        "CircuitBreakerTrials",
        &[
            ("state", 2),
            ("fail_count", 3),
            ("fail_threshold", 3),
            ("cooldown_elapsed", 0),
            ("success", 1),
            ("success_count", 1),
            ("success_threshold", 2),
        ],
    );
    assert_eq!(state, 0); // 2nd consecutive success closes the breaker
    assert_eq!(cell.get("fail_count"), Some(0));
    assert_eq!(cell.get("success_count"), Some(0));
    let (state, cell) = step(
        "circuit_breaker_trials_step",
        "CircuitBreakerTrials",
        &[
            ("state", 2),
            ("fail_count", 3),
            ("fail_threshold", 3),
            ("cooldown_elapsed", 0),
            ("success", 0),
            ("success_count", 1), // one success already banked
            ("success_threshold", 2),
        ],
    );
    assert_eq!(state, 1); // a single failure wipes the tally and reopens (not a duplicate of
    assert_eq!(cell.get("success_count"), Some(0)); // circuit_breaker_step's fixed single-success half-open)
}

#[test]
fn toggle_step_flips_on_each_rising_edge_and_holds_between() {
    // Uses the same cell_src/StateCell pattern as the other agentic-runtime pack tests
    // (see rising_edge_step_fires_only_on_the_transition_to_one above).
    fn step(trigger: u64, prev: u64, state: u64) -> (u16, StateCell) {
        let mut cell = StateCell::bind(&cell_src("toggle_step"), "ToggleStep", None)
            .unwrap_or_else(|e| panic!("bind toggle_step: {e}"));
        cell.set("trigger", trigger).unwrap();
        cell.set("prev", prev).unwrap();
        cell.set("state", state).unwrap();
        let result = cell.run(DEFAULT_CYCLES).unwrap().result;
        (result, cell)
    }

    // trigger sequence: 0, 1, 1, 0, 1 -- state starts at 0. Only the two 0->1 transitions
    // (steps 2 and 5) flip the sticky state; holding high (step 3) or going low (step 4)
    // leaves it unchanged, unlike rising_edge_step which would only ever report a pulse.
    let (result, cell) = step(0, 0, 0);
    assert_eq!(result, 0); // no edge yet, state stays 0
    let (mut prev, mut state) = (cell.get("prev").unwrap(), cell.get("state").unwrap());

    let (result, cell) = step(1, prev, state);
    assert_eq!(result, 1); // rising edge -> state flips 0 -> 1
    prev = cell.get("prev").unwrap();
    state = cell.get("state").unwrap();

    let (result, cell) = step(1, prev, state);
    assert_eq!(result, 1); // holding high, not an edge -> state stays 1
    prev = cell.get("prev").unwrap();
    state = cell.get("state").unwrap();

    let (result, cell) = step(0, prev, state);
    assert_eq!(result, 1); // falling, not a rising edge -> state stays 1
    prev = cell.get("prev").unwrap();
    state = cell.get("state").unwrap();

    let (result, cell) = step(1, prev, state);
    assert_eq!(result, 0); // rising edge again -> state flips back 1 -> 0
    prev = cell.get("prev").unwrap();
    state = cell.get("state").unwrap();
    assert_eq!(prev, 1);
    assert_eq!(state, 0);
}

#[test]
fn hysteresis_u32_matches_defined_behaviour() {
    // hysteresis_u32: wide (u32 value/low/high) sibling of hysteresis -- same Schmitt-trigger
    // dead-zone latch semantics (turn ON at value>=high, turn OFF at value<=low, else hold prior
    // state), but exercised with values above u16::MAX to prove the comparisons are genuinely
    // u32-wide, not a truncated u16 comparison.
    fn step(value: u64, low: u64, high: u64, state: u64) -> u16 {
        let mut cell = StateCell::bind(&cell_src("hysteresis_u32"), "HysteresisU32", None)
            .unwrap_or_else(|e| panic!("bind hysteresis_u32: {e}"));
        cell.set("value", value).unwrap();
        cell.set("low", low).unwrap();
        cell.set("high", high).unwrap();
        cell.set("state", state).unwrap();
        cell.run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run hysteresis_u32: {e}"))
            .result
    }

    // value=80000 >= high=70000 -> turns ON. 80000 exceeds u16::MAX (65535).
    assert_eq!(step(80_000, 20_000, 70_000, 0), 1);
    // value=50000 is in the dead zone (20000 < 50000 < 70000), prior state=1 -> holds ON.
    assert_eq!(step(50_000, 20_000, 70_000, 1), 1);
    // value=10000 <= low=20000 -> turns OFF.
    assert_eq!(step(10_000, 20_000, 70_000, 1), 0);
    // value=50000 dead zone again, prior state=0 -> holds OFF.
    assert_eq!(step(50_000, 20_000, 70_000, 0), 0);
    // value==high exactly (100000 == 100000, above u16::MAX) -> turns ON: boundary is
    // inclusive (>= high), and the equality itself proves full u32-width comparison.
    assert_eq!(step(100_000, 5_000, 100_000, 0), 1);
}

// decorrelated_jitter_backoff_next: range walks off max(current, base), not a fixed ceiling --
// distinct from jittered_backoff_next/jittered_linear_backoff_next which both scale a
// deterministic ceiling down to [0, ceiling]. Here the low end is always `base`, and the
// high end is min(cap, max(current, base) * 3).
#[test]
fn decorrelated_jitter_backoff_next_matches_defined_behaviour() {
    fn step(current: u16, base: u16, cap: u16, rand_bps: u16) -> u16 {
        let mut cell = StateCell::bind(
            &cell_src("decorrelated_jitter_backoff_next"),
            "DecorrelatedJitterBackoff",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("current", current as u64).unwrap();
        cell.set("base", base as u64).unwrap();
        cell.set("cap", cap as u64).unwrap();
        cell.set("rand_bps", rand_bps as u64).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // Bootstrap: current=0, base=10 -> temp=max(0,10)=10, ceiling=min(10*3,1000)=30,
    // range=30-10=20; rand_bps=5000 (50%) -> 20*5000/10000=10, next=base+10=20.
    assert_eq!(step(0, 10, 1000, 5000), 20);
    // rand_bps=0 always collapses to the low end of the range, i.e. exactly `base`,
    // regardless of how large current or the ceiling is.
    assert_eq!(step(20, 10, 1000, 0), 10);
    // Near-max rand_bps=9999: current=5 < base=10, so temp=base=10, ceiling=30, range=20;
    // 20*9999/10000 = 19 (floor), next=10+19=29 -- proves current below base doesn't shrink
    // the walk below the base-anchored floor.
    assert_eq!(step(5, 10, 1000, 9999), 29);
    // Cap saturation: current=100 -> temp*3=300, but cap=50 clamps the ceiling to 50;
    // range=50-10=40; rand_bps=5000 -> 40*5000/10000=20, next=10+20=30.
    assert_eq!(step(100, 10, 50, 5000), 30);
    // Degenerate cap < base: cap=50 clamps the raw ceiling to 50, which is still below
    // base=100, so the defensive floor forces ceiling back up to base -- range collapses to
    // 0 and next is exactly base, never halting or wrapping on the inverted bound.
    assert_eq!(step(5, 100, 50, 5000), 100);
}

#[test]
fn concurrency_gate_step_matches_defined_behaviour() {
    // Counting-semaphore gate: `release`!=0 always decrements in_flight (floored at 0) and
    // reports allowed=1; `release`==0 admits (increments in_flight) only while strictly
    // under max_concurrent, else denies and leaves in_flight untouched.
    fn step(in_flight: u64, max_concurrent: u64, release: u64) -> (u16, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("concurrency_gate_step"),
            "ConcurrencyGateStep",
            None,
        )
        .unwrap_or_else(|e| panic!("bind concurrency_gate_step: {e}"));
        cell.set("in_flight", in_flight).unwrap();
        cell.set("max_concurrent", max_concurrent).unwrap();
        cell.set("release", release).unwrap();
        let result = cell.run(DEFAULT_CYCLES).unwrap().result;
        (result, cell)
    }

    // Acquire under the limit (2 < 5): admitted, in_flight increments to 3.
    let (allowed, cell) = step(2, 5, 0);
    assert_eq!(allowed, 1);
    assert_eq!(cell.get("in_flight"), Some(3));

    // Acquire exactly at the limit (5 < 5 is false): denied, in_flight unchanged.
    let (allowed, cell) = step(5, 5, 0);
    assert_eq!(allowed, 0);
    assert_eq!(cell.get("in_flight"), Some(5));

    // Release with in_flight>0: decrements to 2, always reports allowed=1.
    let (allowed, cell) = step(3, 5, 1);
    assert_eq!(allowed, 1);
    assert_eq!(cell.get("in_flight"), Some(2));

    // Release when already at 0: floors at 0 (no underflow), still allowed=1.
    let (allowed, cell) = step(0, 5, 1);
    assert_eq!(allowed, 1);
    assert_eq!(cell.get("in_flight"), Some(0));

    // Acquire against a zero-capacity gate (0 < 0 is false): always denied.
    let (allowed, cell) = step(0, 0, 0);
    assert_eq!(allowed, 0);
    assert_eq!(cell.get("in_flight"), Some(0));
}

#[test]
fn sliding_window_counter_step_matches_hand_computed() {
    // Sliding-window-counter: blends the previous window's count (weighted by how much
    // of it still overlaps the sliding lookback) with the current window's count, fixing
    // the boundary-burst gap that a hard fixed-window reset (rate_window_update) allows.
    fn step(fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("sliding_window_counter_step"),
            "SlidingWindowCounterStep",
            None,
        )
        .unwrap_or_else(|e| panic!("bind sliding_window_counter_step: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // Case 1: first-ever call, brand-new window, no prior count -> admitted trivially.
    let (report, cell) = step(&[
        ("now", 0),
        ("window_start", 0),
        ("window_size", 100),
        ("prev_count", 0),
        ("curr_count", 0),
        ("limit", 5),
    ]);
    assert_eq!(report.result, 1);
    assert_eq!(cell.get("curr_count"), Some(1));
    assert_eq!(cell.get("prev_count"), Some(0));

    // Case 2: mid-window, no rollover (elapsed 33 < window_size 100). Weighted prev
    // contribution floors: prev_count(7)*remaining(67)/window_size(100) = 469/100 = 4.
    // estimate = curr_count(1) + 4 = 5 < limit(10) -> admitted, curr_count -> 2.
    let (report, cell) = step(&[
        ("now", 133),
        ("window_start", 100),
        ("window_size", 100),
        ("prev_count", 7),
        ("curr_count", 1),
        ("limit", 10),
    ]);
    assert_eq!(report.result, 1);
    assert_eq!(cell.get("curr_count"), Some(2));
    assert_eq!(cell.get("prev_count"), Some(7)); // untouched, no rollover

    // Case 3: same shape but limit lowered so the weighted estimate (6) is not strictly
    // under the limit (6 < 6 is false) -> denied, curr_count NOT incremented.
    // elapsed=50, remaining=50, weighted_prev = 8*50/100 = 4, estimate = 2+4 = 6.
    let (report, cell) = step(&[
        ("now", 150),
        ("window_start", 100),
        ("window_size", 100),
        ("prev_count", 8),
        ("curr_count", 2),
        ("limit", 6),
    ]);
    assert_eq!(report.result, 0);
    assert_eq!(cell.get("curr_count"), Some(2)); // unchanged, not spent

    // Case 4: rollover fires (elapsed_before 110 >= window_size 100): curr_count(5) is
    // carried into prev_count, curr_count resets, window_start snaps to now(210).
    // Immediately after rollover elapsed=0 so remaining=window_size, so the carried
    // prev_count counts in full: weighted_prev = 5*100/100 = 5, estimate = 0+5 = 5 < 10
    // -> admitted, curr_count -> 1.
    let (report, cell) = step(&[
        ("now", 210),
        ("window_start", 100),
        ("window_size", 100),
        ("prev_count", 99),
        ("curr_count", 5),
        ("limit", 10),
    ]);
    assert_eq!(report.result, 1);
    assert_eq!(cell.get("window_start"), Some(210));
    assert_eq!(cell.get("prev_count"), Some(5));
    assert_eq!(cell.get("curr_count"), Some(1));

    // Case 5: time moving backward relative to window_start is a caller bug, not a rate
    // decision -> escalates rather than returning a value.
    let (report, _cell) = step(&[
        ("now", 5),
        ("window_start", 10),
        ("window_size", 100),
        ("prev_count", 0),
        ("curr_count", 0),
        ("limit", 5),
    ]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}
