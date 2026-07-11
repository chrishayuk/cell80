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
fn figurate_number_family_cells_match_defined_behaviour() {
    // Wave 7 (docs/math-server-map.md's figurate_numbers category): polygonal_number
    // generalizes the s-gonal formula (s=3 reproduces triangular, s=4 is the perfect
    // squares, s=5 is pentagonal, s=6 is hexagonal — folding what would otherwise be a
    // differently-named cell per side count), is_polygonal_number is its membership
    // predicate, centered_polygonal_number folds star_number in as its s=12 case, and
    // square_pyramidal_number is the checked-u32 sum-of-squares sequence. Every expected
    // value below was cross-checked against an independent hand derivation of the closed
    // forms (P(s,n) = n + (s-2)*n*(n-1)/2, C(s,n) = 1 + s*n*(n+1)/2,
    // S(n) = n*(n+1)*(2n+1)/6) before being transcribed here.

    fn free_report(id: &str, args: &[u16]) -> cell80::Report {
        let mut r = Runner::compile(&cell_src(id)).unwrap_or_else(|e| panic!("compile {id}: {e}"));
        r.run(None, args, DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run {id}: {e}"))
    }

    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // polygonal_number: P(s, n) = n + (s-2)*n*(n-1)/2.
    assert_eq!(run_cell("polygonal_number", &[3, 10]), 55); // triangular(10)
    assert_eq!(run_cell("polygonal_number", &[4, 5]), 25); // perfect square 5^2
    assert_eq!(run_cell("polygonal_number", &[5, 4]), 22); // pentagonal: 1,5,12,22,35
    assert_eq!(run_cell("polygonal_number", &[6, 3]), 15); // hexagonal: 1,6,15,28,45
    assert_eq!(run_cell("polygonal_number", &[3, 0]), 0);
    assert_eq!(
        free_report("polygonal_number", &[2, 5]).halt,
        cell80::Halt::Escalate(0xFF06)
    ); // s < 3 is not a polygon
    assert_eq!(
        free_report("polygonal_number", &[65535, 1000]).halt,
        cell80::Halt::Escalate(0xFF05)
    ); // genuinely doesn't fit u16

    // is_polygonal_number: membership test via the same closed form, bounded by x.
    assert_eq!(run_cell("is_polygonal_number", &[5, 22]), 1); // pentagonal
    assert_eq!(run_cell("is_polygonal_number", &[5, 23]), 0);
    assert_eq!(run_cell("is_polygonal_number", &[6, 28]), 1); // hexagonal
    assert_eq!(run_cell("is_polygonal_number", &[3, 100]), 0); // 100 is not triangular
    assert_eq!(run_cell("is_polygonal_number", &[3, 0]), 1); // the degenerate n=0 term
    assert_eq!(
        free_report("is_polygonal_number", &[2, 5]).halt,
        cell80::Halt::Escalate(0xFF06)
    );

    // centered_polygonal_number: C(s, n) = 1 + s*n*(n+1)/2.
    assert_eq!(run_cell("centered_polygonal_number", &[6, 0]), 1);
    assert_eq!(run_cell("centered_polygonal_number", &[6, 1]), 7);
    assert_eq!(run_cell("centered_polygonal_number", &[6, 3]), 37); // centered hexagonal
    assert_eq!(run_cell("centered_polygonal_number", &[3, 2]), 10); // centered triangular
    assert_eq!(run_cell("centered_polygonal_number", &[12, 0]), 1); // star_number(1)
    assert_eq!(run_cell("centered_polygonal_number", &[12, 1]), 13); // star_number(2)
    assert_eq!(run_cell("centered_polygonal_number", &[12, 2]), 37); // star_number(3)
    assert_eq!(
        free_report("centered_polygonal_number", &[2, 5]).halt,
        cell80::Halt::Escalate(0xFF06)
    );
    assert_eq!(
        free_report("centered_polygonal_number", &[65535, 1000]).halt,
        cell80::Halt::Escalate(0xFF05)
    );

    // square_pyramidal_number: S(n) = 1^2 + 2^2 + ... + n^2, checked u32 state cell.
    let (_, cell) = step("square_pyramidal_number", "SquarePyramidal", &[("n", 1)]);
    assert_eq!(cell.get("result"), Some(1));
    let (_, cell) = step("square_pyramidal_number", "SquarePyramidal", &[("n", 4)]);
    assert_eq!(cell.get("result"), Some(30));
    let (_, cell) = step("square_pyramidal_number", "SquarePyramidal", &[("n", 10)]);
    assert_eq!(cell.get("result"), Some(385));
    let (_, cell) = step("square_pyramidal_number", "SquarePyramidal", &[("n", 0)]);
    assert_eq!(cell.get("result"), Some(0));
    {
        // The overflow point (sum > u32::MAX around n ~ 2350) needs more iterations than
        // DEFAULT_CYCLES affords for this pack's checked-arithmetic-call-per-iteration
        // cost — budget a larger cycle count explicitly, per the cell's own doc note.
        let mut cell = StateCell::bind(
            &cell_src("square_pyramidal_number"),
            "SquarePyramidal",
            None,
        )
        .unwrap();
        cell.set("n", 100_000).unwrap();
        let report = cell.run(20_000_000).unwrap();
        assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
    }
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

#[test]
fn digit_operations_wave8_cells_match_defined_behaviour() {
    // Wave 8 (docs/math-server-map.md's digital_operations category). Every expected
    // value below was cross-checked against an independent Python reference
    // implementation before being transcribed here.

    fn free_report(id: &str, args: &[u16]) -> cell80::Report {
        let mut r = Runner::compile(&cell_src(id)).unwrap_or_else(|e| panic!("compile {id}: {e}"));
        r.run(None, args, DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run {id}: {e}"))
    }

    let cases: &[(&str, &[u16], u16)] = &[
        // digital_root: closed form 1 + (n-1) mod 9, 0 for n == 0.
        ("digital_root", &[0], 0),
        ("digital_root", &[9], 9),
        ("digital_root", &[18], 9),
        ("digital_root", &[123], 6),
        ("digital_root", &[65535], 6),
        // persistent_digital_root: additive persistence (step count).
        ("persistent_digital_root", &[0], 0),
        ("persistent_digital_root", &[5], 0),
        ("persistent_digital_root", &[10], 1),
        ("persistent_digital_root", &[19], 2),
        ("persistent_digital_root", &[9875], 3),
        // is_palindromic_number: digit-reversal comparison, any base >= 2.
        ("is_palindromic_number", &[121, 10], 1),
        ("is_palindromic_number", &[123, 10], 0),
        ("is_palindromic_number", &[5, 2], 1), // 101 in binary
        ("is_palindromic_number", &[6, 2], 0), // 110 in binary
        ("is_palindromic_number", &[0, 10], 1),
        // next_palindrome: smallest decimal palindrome strictly greater than n.
        ("next_palindrome", &[1001], 1111),
        ("next_palindrome", &[9], 11),
        ("next_palindrome", &[65455], 65456),
        // is_repdigit: every decimal digit identical.
        ("is_repdigit", &[0], 1),
        ("is_repdigit", &[5], 1),
        ("is_repdigit", &[55], 1),
        ("is_repdigit", &[54], 0),
        ("is_repdigit", &[9999], 1),
        // is_automorphic_number: n^2 ends with n.
        ("is_automorphic_number", &[0], 1),
        ("is_automorphic_number", &[5], 1),
        ("is_automorphic_number", &[6], 1),
        ("is_automorphic_number", &[25], 1),
        ("is_automorphic_number", &[76], 1),
        ("is_automorphic_number", &[376], 1),
        ("is_automorphic_number", &[12], 0),
        ("is_automorphic_number", &[100], 0),
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

    // Domain/overflow escalations.
    assert_eq!(
        free_report("is_palindromic_number", &[5, 1]).halt,
        cell80::Halt::Escalate(0xFF06)
    ); // base < 2
       // next_palindrome: no palindrome fits u16 for n in [65456, 65535].
    assert_eq!(
        free_report("next_palindrome", &[65456]).halt,
        cell80::Halt::Escalate(0xFF05)
    );
    assert_eq!(
        free_report("next_palindrome", &[65535]).halt,
        cell80::Halt::Escalate(0xFF05)
    );
}

#[test]
fn modular_classic_number_theory_wave9_cells_match_defined_behaviour() {
    // Wave 9 (docs/math-server-map.md's modular_number_theory category): extended_gcd is
    // the standalone two-Bezout-chain version mod_inverse/crt_solve_pair only inline half
    // of; the other four are independent bounded modular searches. Every expected value
    // below was cross-checked against an independent Python reference implementation
    // before being transcribed here.

    fn free_report(id: &str, args: &[u16]) -> cell80::Report {
        let mut r = Runner::compile(&cell_src(id)).unwrap_or_else(|e| panic!("compile {id}: {e}"));
        r.run(None, args, DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run {id}: {e}"))
    }

    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // extended_gcd: gcd(a,b) plus Bezout x,y with a*x + b*y == gcd. Sign-magnitude output
    // fields (mag, neg): neg == 1 means negative, mag == 0 is always neg == 0.
    let (_, cell) = step("extended_gcd", "ExtendedGcd", &[("a", 240), ("b", 46)]);
    assert_eq!(cell.get("gcd"), Some(2));
    assert_eq!((cell.get("x_mag"), cell.get("x_neg")), (Some(9), Some(1))); // x = -9
    assert_eq!((cell.get("y_mag"), cell.get("y_neg")), (Some(47), Some(0))); // y = 47
    let (_, cell) = step("extended_gcd", "ExtendedGcd", &[("a", 0), ("b", 5)]);
    assert_eq!(cell.get("gcd"), Some(5));
    assert_eq!((cell.get("x_mag"), cell.get("x_neg")), (Some(0), Some(0)));
    assert_eq!((cell.get("y_mag"), cell.get("y_neg")), (Some(1), Some(0)));
    let (_, cell) = step("extended_gcd", "ExtendedGcd", &[("a", 5), ("b", 0)]);
    assert_eq!(cell.get("gcd"), Some(5));
    assert_eq!((cell.get("x_mag"), cell.get("x_neg")), (Some(1), Some(0)));
    assert_eq!((cell.get("y_mag"), cell.get("y_neg")), (Some(0), Some(0)));
    let (_, cell) = step("extended_gcd", "ExtendedGcd", &[("a", 17), ("b", 5)]);
    assert_eq!(cell.get("gcd"), Some(1));
    assert_eq!((cell.get("x_mag"), cell.get("x_neg")), (Some(2), Some(1))); // x = -2
    assert_eq!((cell.get("y_mag"), cell.get("y_neg")), (Some(7), Some(0))); // y = 7
    let (_, cell) = step("extended_gcd", "ExtendedGcd", &[("a", 48), ("b", 18)]);
    assert_eq!(cell.get("gcd"), Some(6));
    assert_eq!((cell.get("x_mag"), cell.get("x_neg")), (Some(1), Some(1))); // x = -1
    assert_eq!((cell.get("y_mag"), cell.get("y_neg")), (Some(3), Some(0))); // y = 3

    // jacobi_symbol: -1/0/1, i16-typed. 65535 is the u16 bit pattern for -1i16 (the
    // same convention mobius_function/sign_i16 already use).
    assert_eq!(run_cell("jacobi_symbol", &[1001, 9907]), 65535); // -1
    assert_eq!(run_cell("jacobi_symbol", &[19, 45]), 1);
    assert_eq!(run_cell("jacobi_symbol", &[8, 21]), 65535); // -1
    assert_eq!(run_cell("jacobi_symbol", &[5, 21]), 1);
    assert_eq!(run_cell("jacobi_symbol", &[3, 9]), 0); // gcd(3,9) = 3 != 1
    assert_eq!(run_cell("jacobi_symbol", &[1, 1]), 1);
    assert_eq!(run_cell("jacobi_symbol", &[0, 5]), 0);
    assert_eq!(
        free_report("jacobi_symbol", &[1, 4]).halt,
        cell80::Halt::Escalate(0xFF06)
    ); // n even

    // order_modulo: smallest k >= 1 with a^k == 1 (mod n).
    assert_eq!(run_cell("order_modulo", &[3, 7]), 6); // 3 is a primitive root mod 7
    assert_eq!(run_cell("order_modulo", &[2, 7]), 3);
    assert_eq!(run_cell("order_modulo", &[1, 5]), 1);
    assert_eq!(
        free_report("order_modulo", &[2, 4]).halt,
        cell80::Halt::Escalate(0xFF06)
    ); // gcd(2,4) = 2 != 1

    // is_quadratic_residue: works for any modulus, not just primes.
    assert_eq!(run_cell("is_quadratic_residue", &[4, 7]), 1); // 2^2 = 4
    assert_eq!(run_cell("is_quadratic_residue", &[5, 7]), 0);
    assert_eq!(run_cell("is_quadratic_residue", &[0, 7]), 1); // 0^2 = 0
    assert_eq!(
        free_report("is_quadratic_residue", &[1, 1]).halt,
        cell80::Halt::Escalate(0xFF06)
    ); // p < 2

    // discrete_log_naive: smallest k in [0, max_exp) with base^k == target (mod m).
    let (_, cell) = step(
        "discrete_log_naive",
        "DiscreteLogNaive",
        &[("base", 3), ("target", 13), ("m", 17), ("max_exp", 20)],
    );
    assert_eq!(cell.get("k"), Some(4));
    let (_, cell) = step(
        "discrete_log_naive",
        "DiscreteLogNaive",
        &[("base", 2), ("target", 1), ("m", 5), ("max_exp", 10)],
    );
    assert_eq!(cell.get("k"), Some(0));
    let (report, _) = step(
        "discrete_log_naive",
        "DiscreteLogNaive",
        &[("base", 2), ("target", 3), ("m", 5), ("max_exp", 2)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06)); // not found within the bound
}

#[test]
fn number_theory_isqrt_u32_wide_sibling() {
    // isqrt_u32: the wide (u32-domain) sibling of isqrt, largest r with r*r <= n.
    // Mirrors is_prime/is_prime_u32's same-pack wide-sibling shape: a state cell
    // (n: u32, r: u32) since u32 can't be a free-fn param/return. run() returns
    // the u16-truncated result (always safe here — max r for u32::MAX is 65535,
    // which fits u16 exactly), and the u32 field `r` carries the same value.
    fn isqrt_wide(n: u64) -> u64 {
        let mut cell = StateCell::bind(&cell_src("isqrt_u32"), "IsqrtWide", None)
            .unwrap_or_else(|e| panic!("bind isqrt_u32: {e}"));
        cell.set("n", n).unwrap();
        cell.run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run isqrt_u32: {e}"));
        cell.get("r").unwrap()
    }

    assert_eq!(isqrt_wide(0), 0);
    assert_eq!(isqrt_wide(1), 1);
    assert_eq!(isqrt_wide(15), 3); // 3*3=9 <= 15 < 16=4*4
    assert_eq!(isqrt_wide(1_000_000), 1000); // perfect square
    assert_eq!(isqrt_wide(4_294_967_295), 65535); // u32::MAX: 65535^2 <= n < 65536^2
    assert_eq!(isqrt_wide(4_294_836_225), 65535); // 65535^2 exactly, near top of domain
}

#[test]
fn number_theory_is_square_u32_wide_sibling() {
    // is_square_u32: matches is_square's own u16-domain answers, then goes past its 65535
    // ceiling into the full u32 domain. An inlined binary search over r in [0, 65535] finds
    // the largest r with r*r <= n, then compares r*r to n -- cheap (~17 steps) across the
    // whole domain, so no larger --cycles budget is ever needed (unlike is_prime_u32's
    // linear trial division).
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

    // Small values, matching is_square's own u16-domain behaviour.
    assert_eq!(step("is_square_u32", "IsSquareWide", &[("n", 0)]).0, 1); // 0*0 == 0
    assert_eq!(step("is_square_u32", "IsSquareWide", &[("n", 100)]).0, 1); // 10^2
    assert_eq!(step("is_square_u32", "IsSquareWide", &[("n", 99)]).0, 0); // between 9^2 and 10^2

    // Past u16::MAX -- exercises the width is_square can't reach at all.
    assert_eq!(step("is_square_u32", "IsSquareWide", &[("n", 65536)]).0, 1); // 256^2

    // The extremes of the u32 domain: the largest perfect square that fits (65535^2), and
    // u32::MAX itself, which is one short of being a perfect square.
    assert_eq!(
        step("is_square_u32", "IsSquareWide", &[("n", 4_294_836_225)]).0,
        1
    ); // 65535^2, the largest square representable in u32
    assert_eq!(
        step("is_square_u32", "IsSquareWide", &[("n", 4_294_967_295)]).0,
        0
    ); // u32::MAX
}

#[test]
fn liouville_function_matches_defined_behaviour() {
    fn free_report(id: &str, args: &[u16]) -> cell80::Report {
        let mut r = Runner::compile(&cell_src(id)).unwrap_or_else(|e| panic!("compile {id}: {e}"));
        r.run(None, args, DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run {id}: {e}"))
    }

    // liouville_function: lambda(n) = (-1)^Omega(n), where Omega counts prime factors with
    // multiplicity (big_omega's own loop). Unlike mobius_function (0 for any non-squarefree
    // n), lambda is always +-1 and defined for every n >= 1. 65535 is the u16 bit pattern
    // for -1i16 (the same convention jacobi_symbol/mobius_function already use).
    assert_eq!(run_cell("liouville_function", &[1]), 1); // Omega(1) = 0 -> +1
    assert_eq!(run_cell("liouville_function", &[2]), 65535); // Omega(2) = 1 -> -1
    assert_eq!(run_cell("liouville_function", &[4]), 1); // 2^2: Omega = 2 -> +1 (mobius(4) = 0)
    assert_eq!(run_cell("liouville_function", &[12]), 65535); // 2^2*3: Omega = 3 -> -1
    assert_eq!(run_cell("liouville_function", &[30]), 65535); // 2*3*5: Omega = 3 -> -1
    assert_eq!(run_cell("liouville_function", &[60]), 1); // 2^2*3*5: Omega = 4 -> +1
    assert_eq!(
        free_report("liouville_function", &[0]).halt,
        cell80::Halt::Escalate(0xFF06)
    ); // n == 0 is out of domain
}

#[test]
fn solve_linear_diophantine_hand_computed_cases() {
    // Every case below was hand-traced through the dialect's exact iterative extended
    // Euclidean algorithm (old_r/r, old_s/s, old_t/t with q = old_r / r) and independently
    // cross-checked with a standalone Python re-implementation of that same iteration
    // before being transcribed here -- never taken from the compiled cell's own output.
    fn step(fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("solve_linear_diophantine"),
            "LinearDiophantine",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // 3x + 5y = 1: gcd(3,5)=1, divides 1. This algorithm's particular Bezout chain gives
    // x0=2, y0=-1 (3*2 + 5*(-1) = 6-5 = 1), scale k=1/1=1 -> x=2, y=-1.
    let (report, cell) = step(&[("a", 3), ("b", 5), ("c", 1)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!((cell.get("x_mag"), cell.get("x_neg")), (Some(2), Some(0))); // x = 2
    assert_eq!((cell.get("y_mag"), cell.get("y_neg")), (Some(1), Some(1))); // y = -1

    // 6x + 9y = 3: gcd(6,9)=3, divides 3. Bezout chain gives x0=-1, y0=1
    // (6*(-1) + 9*1 = -6+9 = 3), scale k=3/3=1 -> x=-1, y=1.
    let (report, cell) = step(&[("a", 6), ("b", 9), ("c", 3)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!((cell.get("x_mag"), cell.get("x_neg")), (Some(1), Some(1))); // x = -1
    assert_eq!((cell.get("y_mag"), cell.get("y_neg")), (Some(1), Some(0))); // y = 1

    // 4x + 6y = 5: gcd(4,6)=2, does not divide 5 -- no integer solution, escalate.
    let (report, _cell) = step(&[("a", 4), ("b", 6), ("c", 5)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // 0x + 7y = 14 (a=0 edge case): gcd(0,7)=7, divides 14. Bezout chain gives x0=0, y0=1
    // (0*0 + 7*1 = 7), scale k=14/7=2 -> x=0, y=2 (0*0 + 7*2 = 14).
    let (report, cell) = step(&[("a", 0), ("b", 7), ("c", 14)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!((cell.get("x_mag"), cell.get("x_neg")), (Some(0), Some(0))); // x = 0
    assert_eq!((cell.get("y_mag"), cell.get("y_neg")), (Some(2), Some(0))); // y = 2

    // 0x + 0y = 0 (a=b=0, c=0 edge case): gcd(0,0)=0, and c==0 too -- trivial solution
    // x=0, y=0 rather than an escalation (0*0 + 0*0 = 0 == c).
    let (report, cell) = step(&[("a", 0), ("b", 0), ("c", 0)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!((cell.get("x_mag"), cell.get("x_neg")), (Some(0), Some(0)));
    assert_eq!((cell.get("y_mag"), cell.get("y_neg")), (Some(0), Some(0)));

    // 0x + 0y = 5 (a=b=0, c!=0 edge case): gcd(0,0)=0 cannot divide a nonzero target --
    // no solution exists, escalate.
    let (report, _cell) = step(&[("a", 0), ("b", 0), ("c", 5)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // 25x + 15y = 10: gcd(25,15)=5, divides 10. Bezout chain gives x0=-1, y0=2
    // (25*(-1) + 15*2 = -25+30 = 5), scale k=10/5=2 -> x=-2, y=4
    // (25*(-2) + 15*4 = -50+60 = 10).
    let (report, cell) = step(&[("a", 25), ("b", 15), ("c", 10)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!((cell.get("x_mag"), cell.get("x_neg")), (Some(2), Some(1))); // x = -2
    assert_eq!((cell.get("y_mag"), cell.get("y_neg")), (Some(4), Some(0))); // y = 4
}


#[test]
fn number_theory_is_pronic_number_oblong_via_4n_plus_1_square() {
    // is_pronic_number: n = k*(k+1) for some k >= 0 (0, 2, 6, 12, 20, 30, ...), checked via
    // the 4n+1-is-a-perfect-square equivalence, using the same branch-free bitwise
    // integer-sqrt loop isqrt_u32/cosine_score_approx use, inlined as a u32 local since
    // cells can't call each other.
    fn is_pronic(n: u64) -> bool {
        // Host-side oracle: find k with k*(k+1) == n by walking k up from 0.
        let mut k: u64 = 0;
        loop {
            let v = k * (k + 1);
            if v == n {
                break true;
            }
            if v > n {
                break false;
            }
            k += 1;
        }
    }

    for n in [0u16, 1, 2, 3, 6, 12, 20, 30, 42, 56, 65280, 65535] {
        let expected = is_pronic(n as u64) as u16;
        let got = run_cell("is_pronic_number", &[n]);
        assert_eq!(got, expected, "is_pronic_number({n}) expected {expected}, got {got}");
    }
}

#[test]
fn number_theory_sum_of_two_squares() {
    // sum_of_two_squares: predicate -- does n == a^2 + b^2 for some a,b >= 0?
    // n=0: 0 = 0^2 + 0^2 -> 1
    assert_eq!(run_cell("sum_of_two_squares", &[0]), 1);
    // n=3: only squares <= 3 are 0 and 1; neither 3-0=3 nor 3-1=2 is a perfect square -> 0
    assert_eq!(run_cell("sum_of_two_squares", &[3]), 0);
    // n=25: 0^2 + 5^2 = 25 -> 1
    assert_eq!(run_cell("sum_of_two_squares", &[25]), 1);
    // n=50: 1^2 + 7^2 = 50 -> 1
    assert_eq!(run_cell("sum_of_two_squares", &[50]), 1);
    // n=2023 = 7 * 17^2; the prime 7 (== 4k+3) divides n to an odd power, so by the
    // sum-of-two-squares theorem n is not expressible -> 0
    assert_eq!(run_cell("sum_of_two_squares", &[2023]), 0);
    // n=65535 (u16::MAX) = 3 * 5 * 17 * 257; the prime 3 (== 4k+3) divides to an odd
    // power -> not expressible; also exercises the top of the u16 domain -> 0
    assert_eq!(run_cell("sum_of_two_squares", &[65535]), 0);
}

#[test]
fn verify_is_carmichael_number() {
    // 561 = 3 * 11 * 17, the smallest Carmichael number. n-1 = 560; (3-1)=2 | 560,
    // (11-1)=10 | 560, (17-1)=16 | 560 (560 / 16 = 35). Korselt holds -> 1.
    assert_eq!(run_cell("is_carmichael_number", &[561]), 1);
    // 1105 = 5 * 13 * 17, the 2nd Carmichael number. n-1 = 1104; 4 | 1104 (276),
    // 12 | 1104 (92), 16 | 1104 (69). Korselt holds -> 1.
    assert_eq!(run_cell("is_carmichael_number", &[1105]), 1);
    // 15 = 3 * 5, squarefree and composite but NOT Carmichael: n-1 = 14, and
    // (5-1)=4 does not divide 14 -> Korselt fails -> 0.
    assert_eq!(run_cell("is_carmichael_number", &[15]), 0);
    // 4 = 2^2 is not squarefree (fails the squareful check before Korselt is even tried) -> 0.
    assert_eq!(run_cell("is_carmichael_number", &[4]), 0);
    // 13 is prime, not composite (only one distinct prime factor: itself) -> 0.
    assert_eq!(run_cell("is_carmichael_number", &[13]), 0);
}

#[test]
fn wilson_theorem_check_hand_computed_cases() {
    // Wilson's theorem: n is prime iff (n-1)! == -1 (mod n), i.e. (n-1)! mod n == n-1.
    // Every expected value below is a hand-computed factorial-mod-n, not taken from the
    // compiled cell's own output.
    // n = 2: 1! = 1, 1 mod 2 = 1 = n-1 -> prime (1).
    assert_eq!(run_cell("wilson_theorem_check", &[2]), 1);
    // n = 3: 2! = 2, 2 mod 3 = 2 = n-1 -> prime (1).
    assert_eq!(run_cell("wilson_theorem_check", &[3]), 1);
    // n = 4: 3! = 6, 6 mod 4 = 2, n-1 = 3, 2 != 3 -> composite (0).
    assert_eq!(run_cell("wilson_theorem_check", &[4]), 0);
    // n = 5: 4! = 24, 24 mod 5 = 4 = n-1 -> prime (1).
    assert_eq!(run_cell("wilson_theorem_check", &[5]), 1);
    // n = 7: 6! = 720, 720 mod 7 = 6 = n-1 -> prime (1).
    assert_eq!(run_cell("wilson_theorem_check", &[7]), 1);
    // n = 9: 8! = 40320, divisible by 9 (3*6 appear as factors) so 40320 mod 9 = 0,
    // which != n-1 = 8 -> composite (0).
    assert_eq!(run_cell("wilson_theorem_check", &[9]), 0);
    // n = 1: below the n >= 2 domain, returns 0 (matches is_prime's convention for n < 2).
    assert_eq!(run_cell("wilson_theorem_check", &[1]), 0);
    // n = 0: below the n >= 2 domain, returns 0.
    assert_eq!(run_cell("wilson_theorem_check", &[0]), 0);
}

#[test]
fn number_theory_wilson_factorial_mod() {
    // wilson_factorial_mod: k! mod m as a running product at u32 width per step,
    // distinct from pow_mod (exponentiation, not a factorial) and wilson_theorem_check
    // (which fixes k = n-1 and compares to n-1) -- this is the general (k, m) utility.
    fn factorial_mod(n: u64, m: u64) -> u64 {
        if m == 0 { return 0; }
        let mut r = 1u64 % m;
        let mut i = 1u64;
        while i <= n {
            r = (r * (i % m)) % m;
            i += 1;
        }
        r
    }

    // 0! mod 5 = 1.
    assert_eq!(run_cell("wilson_factorial_mod", &[0, 5]), 1);
    // 6! mod 7 = 720 mod 7 = 6 -- Wilson's theorem instance ((p-1)! == p-1 mod p, p=7 prime).
    assert_eq!(run_cell("wilson_factorial_mod", &[6, 7]), 6);
    assert_eq!(run_cell("wilson_factorial_mod", &[6, 7]), factorial_mod(6, 7) as u16);
    // 10! mod 6 = 0, since 6 = 2*3 and both factors already appear by i=3 (6 | 10!).
    assert_eq!(run_cell("wilson_factorial_mod", &[10, 6]), 0);
    // m == 0 guard: always 0 regardless of k, matching pow_mod's own m == 0 convention.
    assert_eq!(run_cell("wilson_factorial_mod", &[5, 0]), 0);
    // 10! mod 65521 (65521 is prime, the largest prime below 65536): 3628800 mod 65521 = 25145 --
    // exercises the u32-width running product past pow_mod's own m <= 256 domain.
    assert_eq!(run_cell("wilson_factorial_mod", &[10, 65521]), 25145);
    assert_eq!(run_cell("wilson_factorial_mod", &[10, 65521]), factorial_mod(10, 65521) as u16);
    // 4! mod 4 = 0 -- boundary case k == m, the modulus itself appears as the final factor.
    assert_eq!(run_cell("wilson_factorial_mod", &[4, 4]), 0);
}

#[test]
fn number_theory_is_pow2_u32_wide_sibling() {
    // is_pow2_u32: the wide (u32-domain) sibling of is_pow2, same x != 0 && (x & (x-1)) == 0
    // bit trick as is_pow2 but at u32 width -- mirrors is_prime/is_prime_u32's and
    // is_square/is_square_u32's same-pack wide-sibling shape. A state cell (x: u32, result: u16)
    // since u32 can't be a free-fn param/return; the && short-circuits so x - 1u32 is never
    // evaluated when x == 0 (which would otherwise underflow).
    fn is_pow2_wide(x: u64) -> u64 {
        let mut cell = StateCell::bind(&cell_src("is_pow2_u32"), "IsPow2Wide", None)
            .unwrap_or_else(|e| panic!("bind is_pow2_u32: {e}"));
        cell.set("x", x).unwrap();
        cell.run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run is_pow2_u32: {e}"));
        cell.get("result").unwrap()
    }

    assert_eq!(is_pow2_wide(0), 0); // 0 is not a power of two
    assert_eq!(is_pow2_wide(1), 1); // 2^0
    assert_eq!(is_pow2_wide(2), 1); // 2^1
    assert_eq!(is_pow2_wide(3), 0); // not a power of two
    assert_eq!(is_pow2_wide(65536), 1); // 2^16, beyond is_pow2's u16 ceiling (65535)
    assert_eq!(is_pow2_wide(2_147_483_648), 1); // 2^31, the largest power of two fitting u32
    assert_eq!(is_pow2_wide(4_294_967_295), 0); // u32::MAX = 2^32 - 1, not a power of two
    assert_eq!(is_pow2_wide(100_000), 0); // between 65536 and 131072, not a power of two
}

#[test]
fn number_theory_next_pow2_u32_wide_sibling() {
    // next_pow2_u32: the wide (u32-domain) sibling of next_pow2, smallest power of two >= n.
    // Mirrors is_pow2/is_pow2_u32's same-pack wide-sibling shape: a state cell
    // (n: u32, result: u32) since u32 can't be a free-fn param/return. run() returns
    // a status flag (1u16); the u32 field `result` carries the actual answer.
    fn next_pow2_wide(n: u64) -> u64 {
        let mut cell = StateCell::bind(&cell_src("next_pow2_u32"), "NextPow2Wide", None)
            .unwrap_or_else(|e| panic!("bind next_pow2_u32: {e}"));
        cell.set("n", n).unwrap();
        cell.run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run next_pow2_u32: {e}"));
        cell.get("result").unwrap()
    }

    // next_pow2_u32(0) = 1, matching next_pow2's own zero convention.
    assert_eq!(next_pow2_wide(0), 1);
    // 1 is already a power of two.
    assert_eq!(next_pow2_wide(1), 1);
    // 5 rounds up to 8 (2^2=4 < 5 <= 8=2^3), matching next_pow2's own small-input behaviour.
    assert_eq!(next_pow2_wide(5), 8);
    // 65536 == 2^16, beyond next_pow2's u16 ceiling (65535) -- exact power stays itself.
    assert_eq!(next_pow2_wide(65536), 65536);
    // 2147483648 == 2^31, the largest power of two that fits in u32 -- stays itself.
    assert_eq!(next_pow2_wide(2_147_483_648), 2_147_483_648);
    // 2147483649 == 2^31 + 1: the next power of two would be 2^32, which overflows u32,
    // so the cell reports 0 (matching next_pow2's own past-ceiling convention).
    assert_eq!(next_pow2_wide(2_147_483_649), 0);
    // u32::MAX also overflows past the next power of two.
    assert_eq!(next_pow2_wide(4_294_967_295), 0);
}

// magic_constants: M(n) = n*(n^2+1)/2, the row/column/diagonal sum for an n x n magic
// square filled with 1..n^2. Cubic growth means it escalates (0xFF05) well before n
// reaches u16::MAX; n=51 is the first value that overflows the u16 result.
#[test]
fn magic_constants_closed_form() {
    fn free_report(id: &str, args: &[u16]) -> cell80::Report {
        let mut r = Runner::compile(&cell_src(id)).unwrap_or_else(|e| panic!("compile {id}: {e}"));
        r.run(None, args, DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run {id}: {e}"))
    }

    // n=1: M(1) = 1*2/2 = 1 (trivial 1x1 square).
    assert_eq!(run_cell("magic_constants", &[1]), 1);
    // n=3: M(3) = 3*10/2 = 15 (the classic Lo Shu 3x3 square).
    assert_eq!(run_cell("magic_constants", &[3]), 15);
    // n=4: M(4) = 4*17/2 = 34 (Durer's 4x4 magic square).
    assert_eq!(run_cell("magic_constants", &[4]), 34);
    // n=5: M(5) = 5*26/2 = 65.
    assert_eq!(run_cell("magic_constants", &[5]), 65);
    // n=50: M(50) = 50*2501/2 = 62525, still <= 65535, no halt.
    assert_eq!(run_cell("magic_constants", &[50]), 62525);
    // n=51: M(51) = 51*2602/2 = 66351, exceeds 65535 -> escalates.
    assert_eq!(
        free_report("magic_constants", &[51]).halt,
        cell80::Halt::Escalate(0xFF05)
    );
}

#[test]
fn number_theory_digit_sort_asc() {
    // digit_sort_asc: reassemble n's decimal digits sorted ascending (smallest digit most
    // significant); a sorted-in leading zero just contributes a value-0 place, so it drops
    // naturally on reconstruction. Distinct from digit_reverse (positional reverse, not a sort).
    assert_eq!(run_cell("digit_sort_asc", &[4213]), 1234); // digits 4,2,1,3 -> 1,2,3,4
    assert_eq!(run_cell("digit_sort_asc", &[120]), 12); // digits 1,2,0 -> 0,1,2 -> leading zero drops
    assert_eq!(run_cell("digit_sort_asc", &[0]), 0); // single digit 0
    assert_eq!(run_cell("digit_sort_asc", &[7]), 7); // single digit passes through unchanged
    assert_eq!(run_cell("digit_sort_asc", &[1000]), 1); // digits 1,0,0,0 -> 0,0,0,1 -> leading zeros drop
    assert_eq!(run_cell("digit_sort_asc", &[65535]), 35556); // digits 6,5,5,3,5 -> 3,5,5,5,6
}

#[test]
fn digit_sort_desc_matches_hand_computed_cases() {
    // digit_sort_desc: reassemble n's decimal digits sorted descending (largest digit
    // most significant). Same extraction/local-array/bubble-sort technique as
    // digit_sort_asc, comparison direction flipped. Escalates (0xFF05) when
    // rearranging digits pushes the value past u16::MAX (e.g. 59999 -> 99995).

    // 4213 -> digits {4,2,1,3} sorted descending -> 4321 (spec example).
    assert_eq!(run_cell("digit_sort_desc", &[4213]), 4321);
    // 0 has no digits to extract; result is 0.
    assert_eq!(run_cell("digit_sort_desc", &[0]), 0);
    // Single digit is a no-op.
    assert_eq!(run_cell("digit_sort_desc", &[7]), 7);
    // 40 -> digits {4,0} already descending -> 40 (unchanged).
    assert_eq!(run_cell("digit_sort_desc", &[40]), 40);
    // 1999 -> digits {1,9,9,9} sorted descending -> 9991.
    assert_eq!(run_cell("digit_sort_desc", &[1999]), 9991);

    // 59999 -> digits {5,9,9,9,9} sorted descending -> 99995, which exceeds u16::MAX,
    // so the cell must escalate rather than silently wrap.
    let src = cell_src("digit_sort_desc");
    let mut r = Runner::compile(&src).unwrap();
    let report = r.run(None, &[59999], DEFAULT_CYCLES).unwrap();
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn verify_num_digits_base_generalizes_num_digits_to_arbitrary_base() {
    // num_digits_base(n, base): divide-until-zero digit count at an arbitrary base
    // (base >= 2), generalizing the decimal-only num_digits the same way
    // is_palindromic_number generalized palindrome-checking with a base parameter.
    fn cell_src(id: &str) -> String {
        let cells_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cells");
        let p = cell80::find_cell_file(&cells_dir, id).unwrap_or_else(|e| panic!("{e}"));
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
    }
    fn run_cell(id: &str, args: &[u16]) -> u16 {
        let mut r = Runner::compile(&cell_src(id)).unwrap_or_else(|e| panic!("compile {id}: {e}"));
        r.run(None, args, DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run {id}: {e}"))
            .result
    }

    // 0 has 1 digit regardless of base, matching num_digits's convention.
    assert_eq!(run_cell("num_digits_base", &[0, 10]), 1);
    // 255 = 0xFF -> 2 hex digits; 255 = 0b11111111 -> 8 binary digits.
    assert_eq!(run_cell("num_digits_base", &[255, 16]), 2);
    assert_eq!(run_cell("num_digits_base", &[255, 2]), 8);
    // 8 written in base 8 is "10" -> 2 digits.
    assert_eq!(run_cell("num_digits_base", &[8, 8]), 2);
    // 65535 = 2^16 - 1 is sixteen 1-bits in binary.
    assert_eq!(run_cell("num_digits_base", &[65535, 2]), 16);

    // base < 2 escalates (halt 0xFF06, out_of_domain).
    let mut r = Runner::compile(&cell_src("num_digits_base")).unwrap();
    let report = r.run(None, &[42, 1], DEFAULT_CYCLES).unwrap();
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}

#[test]
fn sum_digit_powers_hand_computed() {
    // sum_digit_powers: sum of each decimal digit of n raised to power p, sum(digit_i^p).
    // Generalizes digit_sum (p=1) with an explicit exponent, using a u32 running
    // accumulator with an overflow guard on both the per-digit term and the running sum.
    fn check(n: u16, p: u16) -> (u16, cell80::Halt) {
        let mut r = Runner::compile(&cell_src("sum_digit_powers")).unwrap();
        let out = r.run(None, &[n, p], DEFAULT_CYCLES).unwrap();
        (out.result, out.halt)
    }

    // 123, p=2: 1^2 + 2^2 + 3^2 = 1 + 4 + 9 = 14
    assert_eq!(check(123, 2).0, 14);
    // 0, p=5: no digits looped, sum stays 0 (matches digit_sum's convention for n=0)
    assert_eq!(check(0, 5).0, 0);
    // 9, p=0: single digit 9, 9^0 = 1 (0^0=1 convention, matches pow_small.rs)
    assert_eq!(check(9, 0).0, 1);
    // 99, p=3: 9^3 + 9^3 = 729 + 729 = 1458
    assert_eq!(check(99, 3).0, 1458);
    // 9, p=6: 9^6 = 531441, exceeds u16 -> escalates (halt 0xFF05, needs_wider_math)
    assert_eq!(check(9, 6).1, cell80::Halt::Escalate(0xFF05));
}
