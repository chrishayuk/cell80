//! Host-oracle tests for the first library wave (`cell80/cells/*.rs`). Each cell is
//! compiled straight from its source file and run through the warm `Runner`, then checked
//! against its **defined** behaviour — saturating arithmetic, `div`/`mod` by zero → 0,
//! predicates → `0`/`1`, runtime bit shifts, and the integer (`u16`) envelope. This is the
//! per-cell edge-case guard the contribution rule asks for; it complements the
//! `cell-eval` retrieval/composition datasets (which exercise discovery + chaining).

use cell80::{Runner, StateCell, DEFAULT_CYCLES};
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
        // `wrap` is an alias of `safe_mod` (admission gate: identical for every input) —
        // covered by `safe_mod`'s own rows below, not a separate cell.
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
        // ── percent / ratio (u32-wide internally: the full u16 domain is exact) ──
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
        // ── ranking / extremum / stats ──
        ("min3", &[5, 2, 8], 2),
        ("min3", &[9, 9, 9], 9),
        ("max3", &[5, 2, 8], 8),
        ("max3", &[1, 40000, 2], 40000),
        ("median3", &[5, 2, 8], 5),
        ("median3", &[1, 2, 3], 2),
        ("median3", &[40000, 65535, 1], 40000),
        // `argmax2`/`argmin2` are aliases of `is_lt`/`is_gt` (admission gate: identical for
        // every input) — covered by their rows above, not separate cells.
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
        // ── number theory (second wave) ──
        ("lcm", &[4, 6], 12),
        ("lcm", &[0, 5], 0),
        ("gcd3", &[48, 36, 60], 12),
        ("divides", &[3, 12], 1),
        ("divides", &[5, 12], 0),
        ("divides", &[0, 5], 0),
        ("is_coprime", &[8, 9], 1),
        ("is_coprime", &[8, 12], 0),
        ("is_prime", &[97], 1),
        ("is_prime", &[1], 0),
        ("is_prime", &[2], 1),
        ("is_prime", &[65535], 0),
        ("isqrt", &[16], 4),
        ("isqrt", &[17], 4),
        ("isqrt", &[65535], 255),
        ("is_square", &[65025], 1),
        ("is_square", &[65535], 0),
        ("is_square", &[0], 1),
        ("digit_sum", &[123], 6),
        ("digit_sum", &[65535], 24),
        ("num_digits", &[0], 1),
        ("num_digits", &[65535], 5),
        ("factor_count", &[12], 6),
        ("factor_count", &[36], 9),
        ("factor_count", &[65535], 16),
        ("triangular", &[10], 55),
        ("triangular", &[361], 65341),
        ("next_pow2", &[5], 8),
        ("next_pow2", &[32768], 32768),
        ("next_pow2", &[40000], 0),
        ("is_pow2", &[8], 1),
        ("is_pow2", &[6], 0),
        ("is_pow2", &[0], 0),
        ("pow_small", &[2, 10], 1024),
        ("pow_small", &[2, 16], 65535),
        ("pow_small", &[5, 0], 1),
        ("cube_sat", &[40], 64000),
        ("cube_sat", &[41], 65535),
        ("pow_mod", &[3, 4, 5], 1),
        ("pow_mod", &[7, 2, 5], 4),
        ("pow_mod", &[5, 3, 0], 0),
        // ── bit / encoding ──
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
        ("mask_xor", &[12, 10], 6),
        // ── hashing / checksum (deterministic — these lock the exact outputs) ──
        ("hash_pair", &[1, 2], 49696),
        ("hash_pair", &[0, 0], 0),
        ("fnv1a_step", &[0, 65], 26195),
        ("fnv1a_step", &[0, 256], 0), // byte masked to 0xFF, so == (0, 0)
        ("crc8_step", &[0, 0], 0),
        ("crc8_step", &[0, 1], 94),
        ("mix16", &[0], 0),
        ("mix16", &[1], 10688),
        // ── stats / bucketing / conversion ──
        ("mode3", &[5, 5, 3], 5),
        ("mode3", &[3, 5, 5], 5),
        ("mode3", &[1, 2, 3], 1),
        ("majority3", &[5, 5, 3], 1),
        ("majority3", &[1, 2, 3], 0),
        ("midrange3", &[1, 2, 9], 5),
        ("bucket3", &[5, 10, 20], 0),
        ("bucket3", &[15, 10, 20], 1),
        ("bucket3", &[25, 10, 20], 2),
        // `quantize` is an alias of `safe_div` (admission gate: identical for every input) —
        // covered by `safe_div`'s own rows above, not a separate cell.
        ("percent_to_byte", &[100], 255),
        ("percent_to_byte", &[50], 127),
        ("byte_to_percent", &[255], 100),
        ("byte_to_percent", &[127], 49),
        // ── calendrical / checksum (wave 3) ──
        ("is_leap_year", &[2000], 1), // divisible by 400
        ("is_leap_year", &[1900], 0), // divisible by 100, not 400
        ("is_leap_year", &[2024], 1), // divisible by 4, not 100
        ("is_leap_year", &[2023], 0),
        ("days_in_month", &[2, 1], 29), // Feb, leap
        ("days_in_month", &[2, 0], 28), // Feb, non-leap
        ("days_in_month", &[4, 0], 30),
        ("days_in_month", &[12, 0], 31),
        ("days_in_month", &[13, 0], 0),    // invalid month
        ("day_of_week", &[2000, 1, 1], 0), // Saturday
        ("day_of_week", &[2024, 1, 1], 2), // Monday
        ("luhn_check", &[1230], 1),        // valid Luhn number
        ("luhn_check", &[1231], 0),        // one digit off — invalid
        ("luhn_check", &[0], 1),           // trivial single-zero-digit edge case
        // ── Q8.8 fixed-point (wave 3) ──
        ("q_mul", &[384, 512], 768),       // 1.5 * 2.0 = 3.0
        ("q_mul", &[256, 256], 256),       // 1.0 * 1.0 = 1.0 (identity)
        ("q_div", &[768, 512], 384),       // 3.0 / 2.0 = 1.5
        ("q_div", &[768, 0], 0),           // divide by zero — safe
        ("q_lerp", &[0, 256, 128], 128),   // halfway, forward
        ("q_lerp", &[200, 100, 64], 175),  // t=0.25, b < a (reverse branch)
        ("q_lerp", &[100, 200, 0], 100),   // t=0 → a
        ("q_lerp", &[100, 200, 256], 200), // t=1.0 → b
        // ── spatial / grid (wave 3) ──
        ("grid_index", &[3, 2, 10], 23),
        ("grid_index", &[0, 0, 10], 0),
        // ── packing / BCD, vector (wave 3, pilot batch) ──
        ("pack_u8", &[0x12, 0x34], 0x1234),
        ("pack_u8", &[0x1FF, 0x2FF], 0xFFFF), // out-of-range inputs mask cleanly
        ("pack_nibbles", &[0xA, 0x5], 0xA5),
        ("bcd_encode", &[42], 0x42),
        ("bcd_encode", &[0], 0),
        ("bcd_decode", &[0x42], 42),
        ("bcd_decode", &[0], 0),
        ("norm2_sq", &[3, 4], 25),
        ("norm2_sq", &[0, 0], 0),
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
fn distance_state_cells_match_defined_behaviour() {
    // The 4-point distance cells exceed the 3-arg convention, so they're state cells (a
    // `Pts` struct, like `manhattan`): set the four coordinates by name, run, read the result.
    fn dist(id: &str, x1: u16, y1: u16, x2: u16, y2: u16) -> u16 {
        let mut cell = StateCell::bind(&cell_src(id), "Pts", None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in [("x1", x1), ("y1", y1), ("x2", x2), ("y2", y2)] {
            cell.set(f, v as u64).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap().result
    }
    // chebyshev = max(|dx|, |dy|); euclid_sq = dx² + dy².
    assert_eq!(dist("chebyshev", 3, 4, 10, 9), 7); // max(7, 5)
    assert_eq!(dist("chebyshev", 0, 0, 5, 2), 5);
    assert_eq!(dist("euclid_sq", 0, 0, 3, 4), 25); // 9 + 16
    assert_eq!(dist("euclid_sq", 1, 1, 4, 5), 25); // 9 + 16

    // euclid_sq's `dist` is a wide u32 field: past the u16 ceiling the scalar result
    // saturates but the named field carries the exact value.
    let mut cell = StateCell::bind(&cell_src("euclid_sq"), "Pts", None).unwrap();
    for (f, v) in [("x1", 0u64), ("y1", 0), ("x2", 300), ("y2", 400)] {
        cell.set(f, v).unwrap();
    }
    assert_eq!(cell.run(DEFAULT_CYCLES).unwrap().result, 65535); // saturated scalar
    assert_eq!(cell.get("dist"), Some(250_000)); // 300² + 400², exact and wide
}

#[test]
fn wide_state_cells_carry_exact_u32_results() {
    // The wide siblings of the result-overflow value cells: the u32 output field holds
    // the exact value the u16 return can't (`square(300)`, `weighted_sum` past 65535).
    let mut sq = StateCell::bind(&cell_src("square_wide"), "Sq", None).unwrap();
    sq.set("n", 300).unwrap();
    sq.run(DEFAULT_CYCLES).unwrap();
    assert_eq!(sq.get("sq"), Some(90_000));
    sq.set("n", 65535).unwrap(); // the domain extreme: 65535² needs all 32 bits
    sq.run(DEFAULT_CYCLES).unwrap();
    assert_eq!(sq.get("sq"), Some(65_535u64 * 65_535));

    let mut ws = StateCell::bind(&cell_src("weighted_sum_wide"), "Ws", None).unwrap();
    for (f, v) in [("a", 30_000u64), ("b", 20_000), ("c", 10_000)] {
        ws.set(f, v).unwrap();
    }
    assert_eq!(ws.run(DEFAULT_CYCLES).unwrap().result, 65535); // saturated scalar
    assert_eq!(ws.get("sum"), Some(100_000)); // a + 2b + 3c, exact
}

#[test]
fn agentic_runtime_state_cells_match_defined_behaviour() {
    // Rate-limiting / resilience state machines (wave 3): each call sets fields by name,
    // runs one step, and reads the mutated state back — the host is responsible for
    // re-feeding the updated fields as the next call's inputs.
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
}

#[test]
fn running_stats_state_cells_match_defined_behaviour() {
    // Running-statistics state cells (wave 3), each driven over a short stream: set fields
    // by name, run, feed the updated state back as the next call's input.
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let result = cell.run(DEFAULT_CYCLES).unwrap().result;
        (result, cell)
    }

    // running_min_max_step: self-initializes on the first call (`seen` starts at 0).
    let (mut min, mut max, mut seen) = (0u64, 0u64, 0u64);
    for (value, expect_range) in [(10u64, 0u64), (3, 7), (7, 7), (20, 17), (1, 19)] {
        let (range, cell) = step(
            "running_min_max_step",
            "RunningMinMax",
            &[("value", value), ("min", min), ("max", max), ("seen", seen)],
        );
        assert_eq!(range as u64, expect_range);
        min = cell.get("min").unwrap();
        max = cell.get("max").unwrap();
        seen = cell.get("seen").unwrap();
    }
    assert_eq!((min, max), (1, 20));

    // streak_step: counts consecutive nonzero inputs, resets hard at a 0.
    let mut streak = 0u64;
    for (input, expect) in [(1u64, 1u64), (1, 2), (1, 3), (0, 0), (1, 1)] {
        let (out, cell) = step(
            "streak_step",
            "Streak",
            &[("input", input), ("streak", streak)],
        );
        assert_eq!(out as u64, expect);
        streak = cell.get("streak").unwrap();
    }

    // accumulate_step: running sum + count, saturating; compose with safe_div for a mean.
    let (mut sum, mut count) = (0u64, 0u64);
    for value in [10u64, 20, 30] {
        let (out, cell) = step(
            "accumulate_step",
            "Accumulate",
            &[("value", value), ("sum", sum), ("count", count)],
        );
        sum = cell.get("sum").unwrap();
        count = cell.get("count").unwrap();
        assert_eq!(out as u64, sum);
    }
    assert_eq!((sum, count), (60, 3));
    assert_eq!(run_cell("safe_div", &[sum as u16, count as u16]), 20); // the composed mean
    let (saturated, _) = step(
        "accumulate_step",
        "Accumulate",
        &[("value", 100), ("sum", 65_500), ("count", 5)],
    );
    assert_eq!(saturated, 65535);
}

#[test]
fn spatial_grid_state_cells_match_defined_behaviour() {
    // point_in_rect / aabb_intersect (wave 3): both half-open — edge-touching doesn't count.
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> u16 {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    assert_eq!(
        step(
            "point_in_rect",
            "PointInRect",
            &[
                ("px", 5),
                ("py", 5),
                ("rx", 0),
                ("ry", 0),
                ("rw", 10),
                ("rh", 10)
            ],
        ),
        1
    );
    assert_eq!(
        step(
            "point_in_rect",
            "PointInRect",
            &[
                ("px", 15),
                ("py", 5),
                ("rx", 0),
                ("ry", 0),
                ("rw", 10),
                ("rh", 10)
            ],
        ),
        0
    );
    assert_eq!(
        step(
            "point_in_rect",
            "PointInRect",
            &[
                ("px", 10),
                ("py", 5),
                ("rx", 0),
                ("ry", 0),
                ("rw", 10),
                ("rh", 10)
            ],
        ),
        0 // on the right edge — half-open, doesn't count
    );

    assert_eq!(
        step(
            "aabb_intersect",
            "AabbIntersect",
            &[
                ("x1", 0),
                ("y1", 0),
                ("w1", 10),
                ("h1", 10),
                ("x2", 5),
                ("y2", 5),
                ("w2", 10),
                ("h2", 10),
            ],
        ),
        1
    );
    assert_eq!(
        step(
            "aabb_intersect",
            "AabbIntersect",
            &[
                ("x1", 0),
                ("y1", 0),
                ("w1", 10),
                ("h1", 10),
                ("x2", 20),
                ("y2", 20),
                ("w2", 5),
                ("h2", 5),
            ],
        ),
        0
    );
    assert_eq!(
        step(
            "aabb_intersect",
            "AabbIntersect",
            &[
                ("x1", 0),
                ("y1", 0),
                ("w1", 10),
                ("h1", 10),
                ("x2", 10),
                ("y2", 0),
                ("w2", 5),
                ("h2", 5),
            ],
        ),
        0 // edge-touching, not overlapping
    );
}

#[test]
fn vector_state_cells_match_defined_behaviour() {
    // dot2 (wave 3, pilot batch): a 4-field state cell purely for arg count (2 vectors),
    // not width — mirrors the manhattan/chebyshev shape.
    let mut cell = StateCell::bind(&cell_src("dot2"), "Dot2", None).unwrap();
    for (f, v) in [("ax", 3u64), ("ay", 4), ("bx", 2), ("by", 1)] {
        cell.set(f, v).unwrap();
    }
    assert_eq!(cell.run(DEFAULT_CYCLES).unwrap().result, 10); // 3*2 + 4*1
}

#[test]
fn checked_arithmetic_state_cells_match_defined_behaviour() {
    // The GSM8K math-campaign foundation pack (Phase 2.3): checked u32 arithmetic that
    // escalates (Halt::Escalate(0xFF05), needs_wider_math) instead of silently wrapping —
    // distinct from safe_div/safe_mod's guard-and-sentinel convention, which hides a real
    // error behind an ordinary-looking 0.
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

    // mul_u16_u16_to_u32: always exact, never escalates (max product fits u32 exactly).
    let (_, _, cell) = step(
        "mul_u16_u16_to_u32",
        "MulWide",
        &[("a", 65535), ("b", 65535)],
    );
    assert_eq!(cell.get("product"), Some(65_535u64 * 65_535));

    // add_checked_u32: normal case returns; overflow escalates.
    let (_, report, cell) = step("add_checked_u32", "AddChecked", &[("a", 10), ("b", 20)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("sum"), Some(30));
    let (_, report, _) = step(
        "add_checked_u32",
        "AddChecked",
        &[("a", (u32::MAX - 5) as u64), ("b", 10)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // sub_checked_u32: normal case returns; b > a escalates.
    let (_, report, cell) = step("sub_checked_u32", "SubChecked", &[("a", 30), ("b", 12)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("diff"), Some(18));
    let (_, report, _) = step("sub_checked_u32", "SubChecked", &[("a", 5), ("b", 12)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // div_exact_u32: evenly divisible returns; a remainder escalates (wrong-plan signal).
    let (_, report, cell) = step("div_exact_u32", "DivExact", &[("a", 100), ("b", 25)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("quotient"), Some(4));
    let (_, report, _) = step("div_exact_u32", "DivExact", &[("a", 100), ("b", 30)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // div_floor_u32 / div_ceil_u32 / mod_u32.
    let (_, _, cell) = step("div_floor_u32", "DivFloor", &[("a", 17), ("b", 5)]);
    assert_eq!(cell.get("quotient"), Some(3));
    let (_, _, cell) = step("div_ceil_u32", "DivCeil", &[("a", 17), ("b", 5)]);
    assert_eq!(cell.get("quotient"), Some(4));
    let (_, _, cell) = step("mod_u32", "ModU32", &[("a", 17), ("b", 5)]);
    assert_eq!(cell.get("rem"), Some(2));

    // fits_u16.
    assert_eq!(step("fits_u16", "FitsU16", &[("x", 65535)]).0, 1);
    assert_eq!(step("fits_u16", "FitsU16", &[("x", 65536)]).0, 0);
}

#[test]
fn money_bps_state_cells_match_defined_behaviour() {
    // The GSM8K math-campaign money/basis-points pack (Phase 2.3, M1 pack 2/5) — basis
    // points, never float percentages, per the campaign spec.
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
fn units_free_fn_cells_match_defined_behaviour() {
    // The GSM8K math-campaign units pack (Phase 2.3, M1 pack 3/5) — dimension codes
    // 0=count,1=money,2=time,3=distance,4=area,5=volume,6=rate_money_per_count,
    // 7=rate_distance_per_time (docs/library-growth.md). Free-fn cells (no u32 state
    // needed), escalating via 0xFF06 (out_of_domain) rather than 0xFF05
    // (needs_wider_math) — a mismatched/unmodeled unit pair isn't a wide-math problem.
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
}

#[test]
fn verifier_ranker_cells_match_defined_behaviour() {
    // The GSM8K math-campaign verifier/ranker pack (Phase 2.3, M1 pack 4/5) — each cell
    // re-derives one side of a candidate plan's claimed equation and returns a plain 0/1
    // verdict, never escalating (a verifier always answers; escalation is for the
    // arithmetic packs that *compute* a value). answer_eq is an alias of the predicates
    // pack's `eq`; multi-plan agreement/tie-break are already covered by
    // `majority3`/`mode3` (ranking-stats) — neither needed new code.
    assert_eq!(run_cell("sum_equals", &[3, 4, 7]), 1);
    assert_eq!(run_cell("sum_equals", &[3, 4, 8]), 0);
    // 40000 + 30000 wraps to 4464 in u16; sum_equals must not false-positive on that.
    assert_eq!(run_cell("sum_equals", &[40000, 30000, 4464]), 0);

    assert_eq!(run_cell("diff_equals", &[10, 3, 7]), 1);
    assert_eq!(run_cell("diff_equals", &[10, 3, 6]), 0);
    assert_eq!(run_cell("diff_equals", &[3, 10, 0]), 0); // a < b → 0, not a wrapped u16

    fn verify(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, cell80::Halt) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report.result, report.halt)
    }

    let (ok, halt) = verify(
        "product_equals_u32",
        "ProductEquals",
        &[("a", 12), ("b", 5), ("total", 60)],
    );
    assert_eq!((ok, halt), (1, cell80::Halt::Returned));
    let (ok, _) = verify(
        "product_equals_u32",
        "ProductEquals",
        &[("a", 12), ("b", 5), ("total", 61)],
    );
    assert_eq!(ok, 0);
    // a genuine u32*u32 overflow is a false claim, not an escalation — a verifier always
    // returns a verdict.
    let (ok, halt) = verify(
        "product_equals_u32",
        "ProductEquals",
        &[("a", 4_294_967_295), ("b", 2), ("total", 0)],
    );
    assert_eq!((ok, halt), (0, cell80::Halt::Returned));

    let (ok, _) = verify(
        "quotient_equals_exact_u32",
        "QuotientEqualsExact",
        &[("a", 48), ("b", 12), ("quotient", 4)],
    );
    assert_eq!(ok, 1);
    let (ok, _) = verify(
        "quotient_equals_exact_u32",
        "QuotientEqualsExact",
        &[("a", 50), ("b", 12), ("quotient", 4)],
    );
    assert_eq!(ok, 0); // remainder 2 — inexact
    let (ok, halt) = verify(
        "quotient_equals_exact_u32",
        "QuotientEqualsExact",
        &[("a", 48), ("b", 0), ("quotient", 4)],
    );
    assert_eq!((ok, halt), (0, cell80::Halt::Returned)); // divide-by-zero is a false verdict too
}

#[test]
fn stateful_rng_cells_match_defined_behaviour() {
    // The stateful/RNG pack (library-growth.md "Next waves") — deterministic pseudo-random
    // steps. `StateCell::run` zeros memory the previous run touched (Runner::run's own
    // doc), so the carried field must be re-`set` from the prior `get` before every call —
    // there's no implicit persistence across separate `.run()` invocations.
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> StateCell {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap();
        cell
    }

    // lcg_next: seed = seed * 1664525 + 1013904223 (mod 2^32), top 16 bits returned —
    // matches the classic Numerical Recipes LCG exactly (checked against a reference
    // computation), not just "changes each step."
    let mut seed = 42u64;
    let expect = [1083814273u64, 378494188, 2479403867, 955863294];
    for want_seed in expect {
        let cell = step("lcg_next", "Lcg", &[("seed", seed)]);
        seed = cell.get("seed").unwrap();
        assert_eq!(seed, want_seed);
    }

    // xorshift16: a distinct recurrence from lcg_next (shift/xor, no multiply); a zero
    // seed is a documented fixed point (stays 0 forever).
    let cell = step("xorshift16", "Xorshift16", &[("x", 1)]);
    let x1 = cell.get("x").unwrap();
    assert_ne!(x1, 0);
    let cell = step("xorshift16", "Xorshift16", &[("x", x1)]);
    let x2 = cell.get("x").unwrap();
    assert_ne!(x2, x1); // genuinely advances, not a fixed point at a nonzero seed
    let cell = step("xorshift16", "Xorshift16", &[("x", 0)]);
    assert_eq!(cell.get("x"), Some(0)); // documented zero-seed degeneracy

    // counter_step: wraps to 0 the instant it would reach `limit`; limit 0 disables
    // wrapping (up to the native u16 boundary, which is out of the cell's control).
    let mut count = 0u64;
    for want in [1u64, 2, 0, 1, 2, 0] {
        let cell = step(
            "counter_step",
            "CounterStep",
            &[("count", count), ("limit", 3)],
        );
        count = cell.get("count").unwrap();
        assert_eq!(count, want);
    }
    let cell = step(
        "counter_step",
        "CounterStep",
        &[("count", 65534), ("limit", 0)],
    );
    assert_eq!(cell.get("count"), Some(65535)); // no wrap when limit == 0
}

#[test]
fn signed_delta_free_fn_cells_match_defined_behaviour() {
    // The signed-deltas pack (library-growth.md "Next waves") — the library's first cells
    // over `i16`, now that the dialect supports it. Negative arguments/results are passed
    // and read as their two's-complement `u16` bit pattern (`-5` <-> `65531`), the same
    // convention `run_cell`'s raw-register interface uses throughout this file.
    assert_eq!(run_cell("sign_i16", &[5]), 1);
    assert_eq!(run_cell("sign_i16", &[65531]), 65535); // -5 -> -1
    assert_eq!(run_cell("sign_i16", &[0]), 0);

    assert_eq!(run_cell("abs_i16", &[5]), 5);
    assert_eq!(run_cell("abs_i16", &[65531]), 5); // -5 -> 5
    assert_eq!(run_cell("abs_i16", &[32768]), 32768); // i16::MIN -> 32768 (doesn't fit i16)

    // clamp_i16(x, lo, hi): lo=-10 (65526), hi=10.
    assert_eq!(run_cell("clamp_i16", &[5, 65526, 10]), 5); // within range, unchanged
    assert_eq!(run_cell("clamp_i16", &[65516, 65526, 10]), 65526); // -20 clamped up to -10
    assert_eq!(run_cell("clamp_i16", &[20, 65526, 10]), 10); // 20 clamped down to 10

    // apply_delta_clamped(value, delta, cap): a bounded resource/health adjustment.
    assert_eq!(run_cell("apply_delta_clamped", &[50, 20, 100]), 70);
    assert_eq!(run_cell("apply_delta_clamped", &[90, 20, 100]), 100); // clamped at cap
    assert_eq!(run_cell("apply_delta_clamped", &[50, 65516, 100]), 30); // delta -20
    assert_eq!(run_cell("apply_delta_clamped", &[10, 65516, 100]), 0); // clamped at 0
    assert_eq!(run_cell("apply_delta_clamped", &[100, 0, 100]), 100);
}

#[test]
fn scoring_choice_cells_match_defined_behaviour() {
    // The scoring/choice pack (library-growth.md "Next waves") — weighted_sum2/3
    // generalize the fixed-weight weighted_sum/weighted_sum_wide to caller-supplied
    // weights, so (unlike their fixed-small-weight siblings) a genuine u32 overflow is
    // reachable and escalates rather than silently wrapping.
    fn verify(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }

    let (result, _, cell) = verify(
        "weighted_sum2",
        "WeightedSum2",
        &[("a", 10), ("wa", 3), ("b", 5), ("wb", 2)],
    );
    assert_eq!((result, cell.get("sum")), (40, Some(40)));
    let (_, report, _) = verify(
        "weighted_sum2",
        "WeightedSum2",
        &[("a", 65535), ("wa", 65535), ("b", 65535), ("wb", 65535)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05)); // both products near u32::MAX
    let (result, _, cell) = verify(
        "weighted_sum2",
        "WeightedSum2",
        &[("a", 1000), ("wa", 1000), ("b", 1), ("wb", 1)],
    );
    assert_eq!((result, cell.get("sum")), (65535, Some(1_000_001))); // saturates the u16 return, sum is exact

    let (result, _, cell) = verify(
        "weighted_sum3",
        "WeightedSum3",
        &[
            ("a", 10),
            ("wa", 1),
            ("b", 5),
            ("wb", 2),
            ("c", 3),
            ("wc", 4),
        ],
    );
    assert_eq!((result, cell.get("sum")), (32, Some(32)));
    let (_, report, _) = verify(
        "weighted_sum3",
        "WeightedSum3",
        &[
            ("a", 65535),
            ("wa", 65535),
            ("b", 65535),
            ("wb", 65535),
            ("c", 1),
            ("wc", 1),
        ],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // choose_best3: highest score wins; distinct from argmax3 since value != score here.
    let (result, _, _) = verify(
        "choose_best3",
        "ChooseBest3",
        &[
            ("val_a", 100),
            ("score_a", 5),
            ("val_b", 200),
            ("score_b", 9),
            ("val_c", 300),
            ("score_c", 7),
        ],
    );
    assert_eq!(result, 200);
    let (result, _, _) = verify(
        "choose_best3",
        "ChooseBest3",
        &[
            ("val_a", 100),
            ("score_a", 9),
            ("val_b", 200),
            ("score_b", 9),
            ("val_c", 300),
            ("score_c", 9),
        ],
    );
    assert_eq!(result, 100); // tie -> lowest index (a) wins

    assert_eq!(run_cell("is_clear_winner", &[90, 60, 20]), 1); // margin 30 >= 20
    assert_eq!(run_cell("is_clear_winner", &[70, 60, 20]), 0); // margin 10 < 20
    assert_eq!(run_cell("is_clear_winner", &[60, 90, 20]), 0); // malformed: top < second
}

#[test]
fn fractions_cells_match_defined_behaviour() {
    // The GSM8K math-campaign fractions pack (Phase 2.3, M1 5/5 — the last authored pack)
    // — u32 numerator/denominator, eager reduction via an inline Euclidean GCD in every
    // cell that needs one (no shared gcd_u32 helper: M0's Tier 2 allows at most one u32
    // param per call, still not the two a general gcd_u32(a, b) needs — see
    // docs/library-growth.md). frac_floor/frac_ceil were skipped: they're exact duplicates
    // of the already-shipped div_floor_u32/div_ceil_u32.
    fn verify(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    let (_, c) = verify("frac_reduce", "FracReduce", &[("n", 6), ("d", 8)]);
    assert_eq!((c.get("num"), c.get("den")), (Some(3), Some(4)));
    let (_, c) = verify("frac_reduce", "FracReduce", &[("n", 0), ("d", 5)]);
    assert_eq!((c.get("num"), c.get("den")), (Some(0), Some(1)));
    let (report, _) = verify("frac_reduce", "FracReduce", &[("n", 5), ("d", 0)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    let (_, c) = verify(
        "frac_add",
        "FracAdd",
        &[("na", 1), ("da", 2), ("nb", 1), ("db", 3)],
    );
    assert_eq!((c.get("num"), c.get("den")), (Some(5), Some(6))); // 1/2 + 1/3 = 5/6
    let (_, c) = verify(
        "frac_add",
        "FracAdd",
        &[("na", 1), ("da", 2), ("nb", 1), ("db", 2)],
    );
    assert_eq!((c.get("num"), c.get("den")), (Some(1), Some(1))); // 1/2 + 1/2 = 1
    let (report, _) = verify(
        "frac_add",
        "FracAdd",
        &[("na", 1), ("da", 0), ("nb", 1), ("db", 2)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    let (_, c) = verify(
        "frac_sub",
        "FracSub",
        &[("na", 3), ("da", 4), ("nb", 1), ("db", 4)],
    );
    assert_eq!((c.get("num"), c.get("den")), (Some(1), Some(2))); // 3/4 - 1/4 = 1/2
    let (report, _) = verify(
        "frac_sub",
        "FracSub",
        &[("na", 1), ("da", 4), ("nb", 3), ("db", 4)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05)); // 1/4 - 3/4 is negative

    let (_, c) = verify(
        "frac_mul",
        "FracMul",
        &[("na", 2), ("da", 3), ("nb", 3), ("db", 4)],
    );
    assert_eq!((c.get("num"), c.get("den")), (Some(1), Some(2))); // 2/3 * 3/4 = 1/2

    let (_, c) = verify(
        "frac_div",
        "FracDiv",
        &[("na", 1), ("da", 2), ("nb", 1), ("db", 3)],
    );
    assert_eq!((c.get("num"), c.get("den")), (Some(3), Some(2))); // (1/2) / (1/3) = 3/2
    let (report, _) = verify(
        "frac_div",
        "FracDiv",
        &[("na", 1), ("da", 2), ("nb", 0), ("db", 3)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06)); // dividing by a zero fraction

    let (report, _) = verify(
        "frac_cmp",
        "FracCmp",
        &[("na", 1), ("da", 2), ("nb", 1), ("db", 3)],
    );
    assert_eq!(report.result, 2); // 1/2 > 1/3
    let (report, _) = verify(
        "frac_cmp",
        "FracCmp",
        &[("na", 1), ("da", 2), ("nb", 2), ("db", 4)],
    );
    assert_eq!(report.result, 1); // 1/2 == 2/4
    let (report, _) = verify(
        "frac_cmp",
        "FracCmp",
        &[("na", 1), ("da", 3), ("nb", 1), ("db", 2)],
    );
    assert_eq!(report.result, 0); // 1/3 < 1/2

    let (report, _) = verify(
        "frac_eq",
        "FracEq",
        &[("na", 1), ("da", 2), ("nb", 2), ("db", 4)],
    );
    assert_eq!(report.result, 1); // equal despite unreduced 2/4
    let (report, _) = verify(
        "frac_eq",
        "FracEq",
        &[("na", 1), ("da", 2), ("nb", 1), ("db", 3)],
    );
    assert_eq!(report.result, 0);

    let (report, _) = verify("is_integer", "IsInteger", &[("n", 10), ("d", 5)]);
    assert_eq!(report.result, 1);
    let (report, _) = verify("is_integer", "IsInteger", &[("n", 10), ("d", 3)]);
    assert_eq!(report.result, 0);
    let (report, _) = verify("is_integer", "IsInteger", &[("n", 5), ("d", 0)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    let (_, c) = verify("frac_to_mixed", "FracToMixed", &[("n", 10), ("d", 4)]);
    assert_eq!(
        (c.get("whole"), c.get("num"), c.get("den")),
        (Some(2), Some(1), Some(2))
    ); // 10/4 = 2 1/2
    let (_, c) = verify("frac_to_mixed", "FracToMixed", &[("n", 9), ("d", 3)]);
    assert_eq!(
        (c.get("whole"), c.get("num"), c.get("den")),
        (Some(3), Some(0), Some(1))
    ); // 9/3 = 3 exactly

    let (_, c) = verify(
        "ratio_split2",
        "RatioSplit2",
        &[("total", 100), ("ratio_a", 3), ("ratio_b", 2)],
    );
    assert_eq!((c.get("part_a"), c.get("part_b")), (Some(60), Some(40)));
    let (_, c) = verify(
        "ratio_split2",
        "RatioSplit2",
        &[("total", 10), ("ratio_a", 1), ("ratio_b", 3)],
    );
    // truncated split, but the two parts always sum exactly to total.
    assert_eq!((c.get("part_a"), c.get("part_b")), (Some(2), Some(8)));
}
