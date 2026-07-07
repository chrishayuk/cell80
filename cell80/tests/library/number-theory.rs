//! Host-oracle tests for the number-theory pack (`cell80/cells/number-theory/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::{cell_src, run_cell};
use cell80::{Runner, StateCell, DEFAULT_CYCLES};

#[test]
fn math_wave3_number_theory_slice() {
    // lcm3: lcm(lcm(4,6),10) = lcm(12,10) = 60.
    assert_eq!(run_cell("lcm3", &[4, 6, 10]), 60);
    assert_eq!(run_cell("lcm3", &[2, 3, 5]), 30);
}

#[test]
fn math_aime_pack_number_theory_slice() {
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

    // pow_mod_u32: matches pow_mod at m <= 256 (3^4 mod 5 = 1), then goes past pow_mod's
    // ceiling to the mod-1000 shape AIME finishing moves need; 0 if m == 0; escalates past
    // m = 65536.
    let (_, _, cell) = step(
        "pow_mod_u32",
        "PowModWide",
        &[("base", 3), ("exp", 4), ("m", 5)],
    );
    assert_eq!(cell.get("result"), Some(1));
    let (_, _, cell) = step(
        "pow_mod_u32",
        "PowModWide",
        &[("base", 7), ("exp", 222), ("m", 1000)],
    );
    assert_eq!(cell.get("result"), Some(49)); // 7^222 mod 1000
    let (_, _, cell) = step(
        "pow_mod_u32",
        "PowModWide",
        &[("base", 5), ("exp", 3), ("m", 0)],
    );
    assert_eq!(cell.get("result"), Some(0));
    let (_, report, _) = step(
        "pow_mod_u32",
        "PowModWide",
        &[("base", 2), ("exp", 10), ("m", 65537)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // mod_add_u32 / mod_sub_u32 / mod_mul_u32: reduce operands mod m first, so a and b
    // needn't already be canonical residues; m == 0 escalates (out_of_domain).
    let (_, _, cell) = step(
        "mod_add_u32",
        "ModAddWide",
        &[("a", 7), ("b", 8), ("m", 10)],
    );
    assert_eq!(cell.get("result"), Some(5)); // (7+8) mod 10
    let (_, _, cell) = step(
        "mod_add_u32",
        "ModAddWide",
        &[("a", 23), ("b", 5), ("m", 10)],
    );
    assert_eq!(cell.get("result"), Some(8)); // operands already exceed m
    let (_, report, _) = step("mod_add_u32", "ModAddWide", &[("a", 1), ("b", 1), ("m", 0)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    let (_, _, cell) = step("mod_sub_u32", "ModSubWide", &[("a", 3), ("b", 5), ("m", 7)]);
    assert_eq!(cell.get("result"), Some(5)); // (3-5) mod 7 = -2 mod 7 = 5
    let (_, _, cell) = step(
        "mod_sub_u32",
        "ModSubWide",
        &[("a", 10), ("b", 3), ("m", 7)],
    );
    assert_eq!(cell.get("result"), Some(0));
    let (_, report, _) = step("mod_sub_u32", "ModSubWide", &[("a", 1), ("b", 1), ("m", 0)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    let (_, _, cell) = step(
        "mod_mul_u32",
        "ModMulWide",
        &[("a", 6), ("b", 7), ("m", 10)],
    );
    assert_eq!(cell.get("result"), Some(2)); // 42 mod 10
    let (_, report, _) = step("mod_mul_u32", "ModMulWide", &[("a", 1), ("b", 1), ("m", 0)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
    let (_, report, _) = step(
        "mod_mul_u32",
        "ModMulWide",
        &[("a", 1), ("b", 1), ("m", 65537)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // sum_divisors: sigma(n) — 6 and 28 are perfect numbers (sigma == 2n); n == 0 escalates.
    let (_, _, cell) = step("sum_divisors", "SumDivisors", &[("n", 6)]);
    assert_eq!(cell.get("result"), Some(12));
    let (_, _, cell) = step("sum_divisors", "SumDivisors", &[("n", 28)]);
    assert_eq!(cell.get("result"), Some(56));
    let (_, _, cell) = step("sum_divisors", "SumDivisors", &[("n", 1)]);
    assert_eq!(cell.get("result"), Some(1));
    let (_, report, _) = step("sum_divisors", "SumDivisors", &[("n", 0)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // euler_totient: phi(1) = 1 by convention; phi(prime) = prime - 1; n == 0 escalates.
    assert_eq!(run_cell("euler_totient", &[1]), 1);
    assert_eq!(run_cell("euler_totient", &[9]), 6);
    assert_eq!(run_cell("euler_totient", &[12]), 4);
    assert_eq!(run_cell("euler_totient", &[17]), 16);

    // smallest_prime_factor: the least prime dividing n; n itself if prime; n < 2 escalates.
    assert_eq!(run_cell("smallest_prime_factor", &[15]), 3);
    assert_eq!(run_cell("smallest_prime_factor", &[17]), 17);

    // digit_reverse: trailing zeros drop; escalates past the u16 ceiling.
    assert_eq!(run_cell("digit_reverse", &[123]), 321);
    assert_eq!(run_cell("digit_reverse", &[120]), 21);
    assert_eq!(run_cell("digit_reverse", &[0]), 0);

    // digit_product: a zero digit anywhere makes the whole product 0.
    assert_eq!(run_cell("digit_product", &[234]), 24);
    assert_eq!(run_cell("digit_product", &[105]), 0);
    assert_eq!(run_cell("digit_product", &[0]), 0);

    // MATH/AIME candidate pack (docs/math-campaign-spec.md "MATH/AIME — scoped ahead of
    // the gate"), authored on explicit request ahead of M3's precipitation read-out: wide
    // modular arithmetic (pow_mod_u32 lifts pow_mod's m <= 256 ceiling to 65536, the
    // finishing move AIME's "remainder mod 1000" problems need), number-theory scalars
    // (sum_divisors, euler_totient, smallest_prime_factor, digit_reverse, digit_product),
    // and checked combinatorics (factorial_checked_u32, choose_u32, permute_u32).
    // count_divisors and dist_sq were scoped but not authored — checking docs/cell-index.md
    // first found they're exact duplicates of factor_count and euclid_sq.
}

#[test]
fn math_aime_pack_second_slice_number_theory_slice() {
    fn gcd_u64(a: u64, b: u64) -> u64 {
        let (mut x, mut y) = (a, b);
        while y != 0 {
            let t = x % y;
            x = y;
            y = t;
        }
        x
    }

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

    // is_prime_u32: matches is_prime's own u16-domain answers, then goes past its 65535
    // ceiling; cost scales with sqrt(n), so this stays well under a real prime's practical
    // limit for the default cycle budget (~1.1M cycles at n ~ 2^20, per the cell's own
    // limits note).
    assert_eq!(step("is_prime_u32", "IsPrimeWide", &[("n", 97)]).0, 1);
    assert_eq!(
        step("is_prime_u32", "IsPrimeWide", &[("n", 1_048_573)]).0,
        1
    ); // a genuine prime just under 2^20
    assert_eq!(
        step("is_prime_u32", "IsPrimeWide", &[("n", 1_048_574)]).0,
        0
    );
    assert_eq!(step("is_prime_u32", "IsPrimeWide", &[("n", 0)]).0, 0);
    assert_eq!(step("is_prime_u32", "IsPrimeWide", &[("n", 1)]).0, 0);

    // mod_inverse: fixed cases (a coprime pair, a non-coprime pair, m == 0), then a sweep.
    let (_, _, cell) = step("mod_inverse", "ModInverse", &[("a", 3), ("m", 11)]);
    assert_eq!(cell.get("result"), Some(4)); // 3*4 = 12 == 1 mod 11
    let (_, report, _) = step("mod_inverse", "ModInverse", &[("a", 6), ("m", 9)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06)); // gcd(6,9) = 3, no inverse
    let (_, report, _) = step("mod_inverse", "ModInverse", &[("a", 5), ("m", 0)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    let mut seed = 12345u64;
    let mut checked = 0;
    for _ in 0..300 {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let a = 1 + (seed >> 40) % 1_000_000;
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let m = 1 + (seed >> 40) % 1_000_000;
        if gcd_u64(a, m) != 1 {
            continue;
        }
        let (_, report, cell) = step("mod_inverse", "ModInverse", &[("a", a), ("m", m)]);
        assert_eq!(report.halt, cell80::Halt::Returned, "a={a} m={m}");
        let inv = cell.get("result").unwrap();
        assert_eq!((a * inv) % m, 1, "a={a} m={m} inv={inv}");
        checked += 1;
    }
    assert!(
        checked > 100,
        "expected most sweep pairs coprime, got {checked}"
    );

    // crt_solve_pair: fixed cases (a coprime pair, a non-coprime pair), then a sweep.
    let (_, _, cell) = step(
        "crt_solve_pair",
        "CrtSolvePair",
        &[("r1", 2), ("m1", 3), ("r2", 3), ("m2", 5)],
    );
    assert_eq!(cell.get("result"), Some(8)); // 8 mod 3 = 2, 8 mod 5 = 3
    let (_, report, _) = step(
        "crt_solve_pair",
        "CrtSolvePair",
        &[("r1", 1), ("m1", 2), ("r2", 1), ("m2", 4)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06)); // gcd(2,4) = 2, not coprime

    let mut seed2 = 987_654_321u64;
    let mut checked2 = 0;
    for _ in 0..300 {
        seed2 = seed2
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let m1 = 1 + (seed2 >> 44) % 1000;
        seed2 = seed2
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let m2 = 1 + (seed2 >> 44) % 1000;
        if gcd_u64(m1, m2) != 1 {
            continue;
        }
        seed2 = seed2
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let r1 = seed2 % m1;
        seed2 = seed2
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let r2 = seed2 % m2;
        let (_, report, cell) = step(
            "crt_solve_pair",
            "CrtSolvePair",
            &[("r1", r1), ("m1", m1), ("r2", r2), ("m2", m2)],
        );
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "r1={r1} m1={m1} r2={r2} m2={m2}"
        );
        let x = cell.get("result").unwrap();
        assert_eq!(x % m1, r1, "r1={r1} m1={m1} r2={r2} m2={m2} x={x}");
        assert_eq!(x % m2, r2, "r1={r1} m1={m1} r2={r2} m2={m2} x={x}");
        assert!(x < m1 * m2);
        checked2 += 1;
    }
    assert!(
        checked2 > 100,
        "expected most sweep pairs coprime, got {checked2}"
    );
    // Second slice of the MATH/AIME pack (docs/math-campaign-spec.md) — the four items
    // originally scoped but deferred: is_prime_u32, shoelace_area_x2, mod_inverse,
    // crt_solve_pair. mod_inverse and crt_solve_pair's input space is too large to usefully
    // hand-pick expected constants for, so alongside fixed cases they get a deterministic
    // pseudo-random property sweep (checking the defining equation itself — a*inverse == 1
    // mod m, or the CRT result satisfies both congruences — rather than a golden value).
}

#[test]
fn wave4_sequences_nth_term_number_theory_slice() {
    // triangular_inverse_exact: T(5)=15, T(361)=65341 (triangular's own domain max), T(0)=0,
    // and 14 (between T(4)=10 and T(5)=15) isn't triangular.
    assert_eq!(run_cell("triangular_inverse_exact", &[15]), 5);
    assert_eq!(run_cell("triangular_inverse_exact", &[65341]), 361);
    assert_eq!(run_cell("triangular_inverse_exact", &[0]), 0);
    {
        let mut r = Runner::compile(&cell_src("triangular_inverse_exact")).unwrap();
        let report = r.run(None, &[14], DEFAULT_CYCLES).unwrap();
        assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
    }
}

#[test]
fn math_server_number_theory_family_cells_match_defined_behaviour() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    fn free_report(id: &str, args: &[u16]) -> cell80::Report {
        let mut r = Runner::compile(&cell_src(id)).unwrap_or_else(|e| panic!("compile {id}: {e}"));
        r.run(None, args, DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run {id}: {e}"))
    }

    // little_omega: count of distinct prime factors; omega(1) = 0 by convention; n == 0 escalates.
    assert_eq!(run_cell("little_omega", &[1]), 0);
    assert_eq!(run_cell("little_omega", &[12]), 2); // 2, 3
    assert_eq!(run_cell("little_omega", &[2]), 1);
    assert_eq!(run_cell("little_omega", &[30]), 3); // 2, 3, 5
    assert_eq!(
        free_report("little_omega", &[0]).halt,
        cell80::Halt::Escalate(0xFF06)
    );

    // big_omega: total prime factors with multiplicity; Omega(1) = 0; n == 0 escalates.
    assert_eq!(run_cell("big_omega", &[1]), 0);
    assert_eq!(run_cell("big_omega", &[12]), 3); // 2, 2, 3
    assert_eq!(run_cell("big_omega", &[8]), 3); // 2, 2, 2
    assert_eq!(run_cell("big_omega", &[30]), 3); // 2, 3, 5 (squarefree, matches little_omega)
    assert_eq!(
        free_report("big_omega", &[0]).halt,
        cell80::Halt::Escalate(0xFF06)
    );

    // mobius_function: 1 at n=1; 0 whenever n has a squared prime factor; else (-1)^omega(n);
    // n == 0 escalates. Negative results are read as their two's-complement u16 bit pattern
    // (-1 -> 65535), the same convention signed_delta_free_fn_cells_match_defined_behaviour uses.
    assert_eq!(run_cell("mobius_function", &[1]), 1);
    assert_eq!(run_cell("mobius_function", &[2]), 65535); // mu(2) = -1
    assert_eq!(run_cell("mobius_function", &[4]), 0); // 2^2 | 4
    assert_eq!(run_cell("mobius_function", &[6]), 1); // 2*3, two distinct primes -> +1
    assert_eq!(run_cell("mobius_function", &[30]), 65535); // 2*3*5, three primes -> -1
    assert_eq!(
        free_report("mobius_function", &[0]).halt,
        cell80::Halt::Escalate(0xFF06)
    );

    // divisor_power_sum (sigma_k): k=0 is factor_count, k=1 is sum_divisors.
    let (_, cell) = step(
        "divisor_power_sum",
        "DivisorPowerSum",
        &[("n", 6), ("k", 0)],
    );
    assert_eq!(cell.get("result"), Some(4)); // tau(6) = 4
    let (_, cell) = step(
        "divisor_power_sum",
        "DivisorPowerSum",
        &[("n", 6), ("k", 1)],
    );
    assert_eq!(cell.get("result"), Some(12)); // sigma(6) = 12
    let (_, cell) = step(
        "divisor_power_sum",
        "DivisorPowerSum",
        &[("n", 6), ("k", 2)],
    );
    assert_eq!(cell.get("result"), Some(50)); // 1+4+9+36
    let (_, cell) = step(
        "divisor_power_sum",
        "DivisorPowerSum",
        &[("n", 1), ("k", 5)],
    );
    assert_eq!(cell.get("result"), Some(1));
    let (report, _) = step(
        "divisor_power_sum",
        "DivisorPowerSum",
        &[("n", 0), ("k", 1)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
    // 65535 = 3*5*17*257 has 16 divisors; sigma_2(65535) = 4,980,170,000, past u32::MAX.
    let (report, _) = step(
        "divisor_power_sum",
        "DivisorPowerSum",
        &[("n", 65535), ("k", 2)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // jordan_totient (J_k): J_1 matches euler_totient; J_1(1) = 1 (empty product).
    let (_, cell) = step("jordan_totient", "JordanTotient", &[("n", 6), ("k", 1)]);
    assert_eq!(cell.get("result"), Some(2)); // euler_totient(6) = 2
    let (_, cell) = step("jordan_totient", "JordanTotient", &[("n", 1), ("k", 1)]);
    assert_eq!(cell.get("result"), Some(1));
    let (_, cell) = step("jordan_totient", "JordanTotient", &[("n", 9), ("k", 2)]);
    assert_eq!(cell.get("result"), Some(72)); // J_2(9) = 9^2 * (1 - 1/9) = 72
    let (_, cell) = step("jordan_totient", "JordanTotient", &[("n", 6), ("k", 12)]);
    assert_eq!(cell.get("result"), Some(2_176_246_800)); // fits u32, just under the ceiling
    let (report, _) = step("jordan_totient", "JordanTotient", &[("n", 6), ("k", 13)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05)); // J_13(6) ~ 1.3e10, overflows u32
    let (report, _) = step("jordan_totient", "JordanTotient", &[("n", 0), ("k", 1)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // carmichael_lambda: the reduced totient, lambda(n) | euler_totient(n) and lambda(n) <= n.
    assert_eq!(run_cell("carmichael_lambda", &[1]), 1);
    assert_eq!(run_cell("carmichael_lambda", &[8]), 2); // lambda(2^3) = 2^(3-2) = 2
    assert_eq!(run_cell("carmichael_lambda", &[15]), 4); // lcm(lambda(3)=2, lambda(5)=4) = 4
    assert_eq!(run_cell("carmichael_lambda", &[20]), 4); // lcm(lambda(4)=2, lambda(5)=4) = 4
    assert_eq!(
        free_report("carmichael_lambda", &[0]).halt,
        cell80::Halt::Escalate(0xFF06)
    );
    // The math-server-mined number-theory family (docs/math-server-map.md's 77-candidate
    // coverage map, `docs/real-valued-cells-spec.md` Wave 1): little_omega/big_omega (distinct
    // vs. multiplicity-counted prime factors), mobius_function (the classic sign/squarefree
    // function), divisor_power_sum (sigma_k, generalizing factor_count/sum_divisors with an
    // exponent), jordan_totient (generalizing euler_totient with an exponent), and
    // carmichael_lambda (the reduced totient). Every expected value below was cross-checked
    // against an independent Python reference implementation before being hand-transcribed
    // here, the same discipline mod_inverse/crt_solve_pair/shoelace_area_x2 used.
    // Free-fn variant (little_omega/big_omega/mobius_function take no state), matching
    // units_free_fn_cells_match_defined_behaviour's own local helper.
}

#[test]
fn first_wave_number_theory_cells_match_defined_behaviour() {
    let cases: &[(&str, &[u16], u16)] = &[
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
