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
