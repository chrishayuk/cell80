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
        prev = cell.get("prev").unwrap() as u64;
        assert_eq!(prev, input); // prev always tracks the last input after run()
    }
}
