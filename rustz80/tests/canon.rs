//! The canonicalization pass (M2.5) + dialect normalizer (M2.6) at the compiler level:
//! byte-stability when nothing fires, noun independence, statement-order independence,
//! defer-division, exact constant folding with width inference, the unit table, and
//! typed diagnostics. The hash-level acceptance tests (identical *artifact hash*) live
//! in the `cell80` crate; these prove the text layer they stand on.

use rustz80::{
    canonical_unit, canonicalize_source, compile_program, CanonMode, CanonOptions, DiagCode,
    UnitHint,
};

fn full() -> CanonOptions {
    CanonOptions {
        mode: CanonMode::Full,
        ..Default::default()
    }
}

fn full_canon(src: &str) -> rustz80::CanonOutput {
    canonicalize_source(src, &full()).expect("canonicalizes")
}

/// Canonical output must be real dialect Rust: compile it.
fn assert_compiles(src: &str) {
    compile_program(src).unwrap_or_else(|e| panic!("canonical source must compile: {e}\n{src}"));
}

// ---------------------------------------------------------------- light mode

#[test]
fn light_is_byte_stable_on_clean_source() {
    let src = "//! lcm cell\nfn run(a: u16, b: u16) -> u16 { let g = gcd(a, b); if g != 0u16 { a / g * b } else { 0u16 } }\nfn gcd(a: u16, b: u16) -> u16 { let mut x = a; let mut y = b; while y != 0u16 { let t = x % y; x = y; y = t; } x }\n";
    let out = canonicalize_source(src, &CanonOptions::default()).unwrap();
    assert!(!out.changed);
    assert_eq!(out.source, src, "no rule fired ⇒ byte-identical");
}

#[test]
fn light_strips_statement_macros() {
    let src = "fn run(a: u16) -> u16 { println!(\"dbg\"); a + 1u16 }";
    let out = canonicalize_source(src, &CanonOptions::default()).unwrap();
    assert!(out.changed);
    assert!(!out.source.contains("println"));
    assert!(out
        .repairs
        .iter()
        .any(|r| r.code == DiagCode::StatementMacro));
}

#[test]
fn light_rewrites_trailing_let_to_tail() {
    // The row93 class: composed body ended `let jackson = amy - 5;`.
    let src = "fn run(amy: u16) -> u16 { let jackson = amy - 5; }";
    let out = canonicalize_source(src, &CanonOptions::default()).unwrap();
    assert!(out.repairs.iter().any(|r| r.code == DiagCode::TrailingLet));
    assert_compiles(&out.source);
}

#[test]
fn light_rewrites_trailing_return() {
    let src = "fn run(a: u16) -> u16 { let x = a + 2u16; return x; }";
    let out = canonicalize_source(src, &CanonOptions::default()).unwrap();
    assert!(out.repairs.iter().any(|r| r.code == DiagCode::TrailingLet));
    assert_compiles(&out.source);
}

#[test]
fn light_collapses_redundant_parens() {
    let src = "fn run(a: u16) -> u16 { ((a)) + ((1u16)) }";
    let out = canonicalize_source(src, &CanonOptions::default()).unwrap();
    assert!(out
        .repairs
        .iter()
        .any(|r| r.code == DiagCode::RedundantParens));
    assert_compiles(&out.source);
}

// ------------------------------------------------------------ noun independence

#[test]
fn full_same_structure_different_nouns_identical_text() {
    let pencils = "fn run(pencils: u16, boxes: u16) -> u16 { let per_box = pencils * 3; let total = per_box + boxes; total }";
    let notebooks = "fn run(notebooks: u16, crates: u16) -> u16 { let stacked = notebooks * 3; let sum = stacked + crates; sum }";
    let a = full_canon(pencils);
    let b = full_canon(notebooks);
    assert_eq!(
        a.source, b.source,
        "nouns must not reach the canonical text"
    );
    assert!(a.source.contains("q0") && a.source.contains("v0"));
    assert!(!a.source.contains("pencils"));
    // Source names survive as metadata only.
    assert!(a
        .renames
        .iter()
        .any(|r| r.source_name == "pencils" && r.slot == "q0"));
    assert_compiles(&a.source);
}

#[test]
fn full_statement_permutation_identical_text() {
    let one = "fn run(a: u16, b: u16) -> u16 { let x = a * 2; let y = b * 5; x + y }";
    let two = "fn run(a: u16, b: u16) -> u16 { let y = b * 5; let x = a * 2; x + y }";
    assert_eq!(full_canon(one).source, full_canon(two).source);
}

#[test]
fn full_expression_regrouping_identical_text() {
    let one = "fn run(a: u16, b: u16, c: u16) -> u16 { a + b + c }";
    let two = "fn run(a: u16, b: u16, c: u16) -> u16 { c + (a + b) }";
    assert_eq!(full_canon(one).source, full_canon(two).source);
}

#[test]
fn full_is_idempotent() {
    let src = "fn run(a: u16, b: u16) -> u16 { let x = a * 2; let y = b * 5; x + y }";
    let once = full_canon(src);
    let twice = full_canon(&once.source);
    assert_eq!(once.source, twice.source);
}

// ------------------------------------------------------------- defer-division

#[test]
fn defer_division_reorders_div_before_mul() {
    // `a / 100 * 30` (truncates early — the captured failure class) and
    // `a * 30 / 100` must both canonicalize to multiply-then-single-divide.
    let early = "fn run(a: u16) -> u16 { a / 100 * 30 }";
    let late = "fn run(a: u16) -> u16 { a * 30 / 100 }";
    let e = full_canon(early);
    let l = full_canon(late);
    assert_eq!(e.source, l.source);
    // 30/100 reduces to 3/10: multiply by 3 first, divide by 10 last.
    assert!(e.source.contains("q0 * 3u16"), "got:\n{}", e.source);
    assert!(e.source.contains("/ 10u16"));
    assert_compiles(&e.source);
}

#[test]
fn decimal_literal_becomes_exact_fraction() {
    // The registered example: `0.9` → 9/10 — same canonical text as `* 9 / 10`.
    let dec = "fn run(x: u16) -> u16 { x * 0.9 }";
    let frac = "fn run(x: u16) -> u16 { x * 9 / 10 }";
    assert_eq!(full_canon(dec).source, full_canon(frac).source);
}

#[test]
fn division_by_division_normalizes() {
    // (a/b)/(c/d) = a*d / (b*c).
    let nested = "fn run(a: u16, b: u16, c: u16, d: u16) -> u16 { (a / b) / (c / d) }";
    let flat = "fn run(a: u16, b: u16, c: u16, d: u16) -> u16 { a * d / (b * c) }";
    assert_eq!(full_canon(nested).source, full_canon(flat).source);
}

// ------------------------------------------------- constant folding + width

#[test]
fn constant_division_folds_exactly_and_widens() {
    // The row89 class: `88000 / 11` is entirely constant — folded at compile
    // time, and the 88000 literal widens the lane.
    let out = full_canon("fn run() -> u16 { 88000 / 11 }");
    assert!(out.widened);
    assert!(out.source.contains("-> u32"));
    assert!(out.source.contains("8000u32"), "got:\n{}", out.source);
    assert!(out
        .repairs
        .iter()
        .any(|r| r.code == DiagCode::WidthExceedsU16));
    assert_compiles(&out.source);
}

#[test]
fn wide_lane_widens_params_at_use() {
    let out = full_canon("fn run(a: u16) -> u16 { a * 88000 / 11 }");
    assert!(out.widened);
    assert!(
        out.source.contains("(q0 as u32) * 8000u32"),
        "got:\n{}",
        out.source
    );
    assert_compiles(&out.source);
}

#[test]
fn inexact_constant_division_is_a_typed_error() {
    let err = canonicalize_source("fn run() -> u16 { 100 / 3 }", &full()).unwrap_err();
    assert_eq!(err.code, DiagCode::InexactConstDivision);
}

#[test]
fn division_by_constant_zero_is_a_typed_error() {
    let err = canonicalize_source("fn run(a: u16) -> u16 { a / 0 }", &full()).unwrap_err();
    assert_eq!(err.code, DiagCode::InexactConstDivision);
}

#[test]
fn negative_constant_is_a_typed_error() {
    let err = canonicalize_source("fn run(a: u16) -> u16 { a + (0 - 5) * 2 }", &full());
    // 0-5 folds to −5, ×2 = −10, additive constant −10 … emitted as a subtraction —
    // legal. A *standalone* negative constant is the error:
    let hard = canonicalize_source("fn run() -> u16 { 3 - 5 }", &full()).unwrap_err();
    assert_eq!(hard.code, DiagCode::NegativeConst);
    // and the additive form stays legal (a − 10).
    let ok = err.expect("additive negative folds into a subtraction");
    assert!(ok.source.contains("- 10u16"), "got:\n{}", ok.source);
}

#[test]
fn wide_default_forces_u32_lane() {
    let opts = CanonOptions {
        mode: CanonMode::Full,
        wide_default: true,
        ..Default::default()
    };
    let out = canonicalize_source("fn run(a: u16, b: u16) -> u16 { a * b }", &opts).unwrap();
    assert!(out.widened);
    assert!(out.source.contains("-> u32"));
    assert_compiles(&out.source);
}

// ----------------------------------------------------------------- unit table

#[test]
fn money_hint_scales_decimal_to_cents() {
    let src = "fn run(n: u16) -> u16 { let price = 16.50; price * n }";
    let opts = CanonOptions {
        mode: CanonMode::Full,
        hints: vec![UnitHint {
            ident: "price".into(),
            unit: "dollars".into(),
        }],
        ..Default::default()
    };
    let out = canonicalize_source(src, &opts).unwrap();
    assert!(out.source.contains("1650u16"), "got:\n{}", out.source);
    assert!(out
        .repairs
        .iter()
        .any(|r| r.code == DiagCode::UnitScaled && r.detail.contains("factor=100")));
    assert_compiles(&out.source);
}

#[test]
fn unscaled_decimal_in_additive_position_is_typed() {
    let err = canonicalize_source("fn run(a: u16) -> u16 { a + 16.50 }", &full()).unwrap_err();
    assert_eq!(err.code, DiagCode::RequiresFractionalScale);
}

#[test]
fn unit_table_normalizes_rates_and_unknown_nouns() {
    assert_eq!(
        canonical_unit("dollars_per_egg"),
        ("cents_per_count".into(), 100, 1)
    );
    assert_eq!(
        canonical_unit("miles_per_hour"),
        ("meters_per_seconds".into(), 1609, 3600)
    );
    // Unknown nouns are counts by convention.
    assert_eq!(canonical_unit("sheep"), ("count".into(), 1, 1));
    assert_eq!(canonical_unit("cups"), ("count".into(), 1, 1));
    assert_eq!(canonical_unit("gb"), ("count".into(), 1, 1));
    assert_eq!(canonical_unit("hours"), ("seconds".into(), 3600, 1));
    assert_eq!(canonical_unit("dollars"), ("cents".into(), 100, 1));
}

#[test]
fn param_units_are_metadata_not_rewrites() {
    let src = "fn run(price: u16) -> u16 { price * 2 }";
    let opts = CanonOptions {
        mode: CanonMode::Full,
        hints: vec![UnitHint {
            ident: "price".into(),
            unit: "dollars".into(),
        }],
        ..Default::default()
    };
    let out = canonicalize_source(src, &opts).unwrap();
    let r = out
        .renames
        .iter()
        .find(|r| r.source_name == "price")
        .expect("param rename recorded");
    assert_eq!(r.unit.as_deref(), Some("cents"));
    assert_eq!(
        r.factor, 100,
        "caller-side factor recorded, value untouched"
    );
}

// ------------------------------------------------------------ calls + fallback

#[test]
fn calls_survive_with_slot_args() {
    let src = "fn run(a: u16, b: u16) -> u16 { let g = gcd(a, b); a / g * b }";
    let out = full_canon(src);
    assert!(out.source.contains("gcd(q0, q1)"), "got:\n{}", out.source);
}

#[test]
fn non_straight_line_falls_back_to_light() {
    // if-value is canonical since the select node landed — a loop is the probe now.
    let src = "fn run(a: u16) -> u16 { let mut s = 0u16; while s < a { s = s + 2u16; } s }";
    let out = full_canon(src);
    assert!(!out.changed, "control flow: light fallback, byte-stable");
    assert!(out
        .repairs
        .iter()
        .any(|r| r.code == DiagCode::NonStraightLine));
}

#[test]
fn dead_lets_are_dropped_and_recorded() {
    let src = "fn run(a: u16) -> u16 { let unused = a + 7; a * 2 }";
    let out = full_canon(src);
    assert!(!out.source.contains("7u16"));
    assert!(out.repairs.iter().any(|r| r.code == DiagCode::DeadLet));
}

// ------------------------------------------------------------------ diagnostics

#[test]
fn legacy_error_strings_classify() {
    use rustz80::classify_error;
    assert_eq!(
        classify_error("kill: render/compile: unknown call target `choose_best3`"),
        Some(DiagCode::UnknownCallTarget)
    );
    assert_eq!(
        compile_program("fn run() -> u16 { 1.5 }")
            .err()
            .and_then(|e| classify_error(&e)),
        Some(DiagCode::RequiresFractionalScale)
    );
}

#[test]
fn method_calls_rewrite_to_kernels() {
    // Registered amendment 2026-07-06 (E0205): granite's `.max(0)` habit becomes the
    // prelude kernel call — same canonical text as writing the kernel directly.
    let method = "fn run(a: u16, b: u16) -> u16 { let bigger = a.max(b); bigger * 2 }";
    let kernel = "fn run(a: u16, b: u16) -> u16 { let bigger = imax(a, b); bigger * 2 }";
    let m = full_canon(method);
    assert_eq!(m.source, full_canon(kernel).source);
    assert!(m.source.contains("imax(q0, q1)"), "got:\n{}", m.source);
    assert!(m.repairs.iter().any(|r| r.code == DiagCode::MethodToKernel));
    // Unknown methods stay a soft fallback, never a guess.
    let out = full_canon("fn run(a: u16) -> u16 { a.checked_mul(2).unwrap_or(0) }");
    assert!(out
        .repairs
        .iter()
        .any(|r| r.code == DiagCode::NonStraightLine));
}

#[test]
fn literal_lifting_generalizes_the_schema() {
    let opts = CanonOptions {
        mode: CanonMode::Full,
        lift_literals: true,
        ..Default::default()
    };
    // Same structure, different numbers ⇒ ONE canonical schema; the values move
    // to arguments (the H-M3 shape: precipitation across problem instances).
    let a = canonicalize_source(
        "fn run() -> u16 { let amy = 30; let jake = 5; amy * jake }",
        &opts,
    )
    .unwrap();
    let b = canonicalize_source(
        "fn run() -> u16 { let pens = 12; let cost = 7; pens * cost }",
        &opts,
    )
    .unwrap();
    assert_eq!(a.source, b.source);
    assert!(
        a.source.contains("fn run(q0: u16, q1: u16)"),
        "{}",
        a.source
    );
    assert_eq!(a.lifted, vec![("q0".into(), 30), ("q1".into(), 5)]);
    assert_eq!(b.lifted, vec![("q0".into(), 12), ("q1".into(), 7)]);
    assert_eq!(
        a.repairs
            .iter()
            .filter(|r| r.code == DiagCode::QuantityLifted)
            .count(),
        2
    );
    assert_compiles(&a.source);
}

#[test]
fn lifting_keeps_structural_constants_baked() {
    let opts = CanonOptions {
        mode: CanonMode::Full,
        lift_literals: true,
        ..Default::default()
    };
    // The let-bound 250 is a quantity (lifted); the inline 30/100 is structure (baked).
    let out = canonicalize_source("fn run() -> u16 { let x = 250; x * 30 / 100 }", &opts).unwrap();
    assert_eq!(out.lifted, vec![("q0".into(), 250)]);
    assert!(out.source.contains("* 3u16"), "{}", out.source);
    assert!(out.source.contains("/ 10u16"));
}

// ------------------------------------------------- casts + if-value (select)

#[test]
fn casts_are_transparent_and_unblock_rewrites() {
    // The granite row22 shape: a cast tail used to soft-fail the whole fn, blocking
    // both E0205 and lifting. Now `as u16` is the identity in the narrow lane and
    // `as u32` just commits the wide lane.
    let opts = CanonOptions {
        mode: CanonMode::Full,
        lift_literals: true,
        ..Default::default()
    };
    let src = "fn run() -> u16 { let sams = 31; let ray = sams - 6; let son = ray - 23; (son.max(0)) as u16 }";
    let out = canonicalize_source(src, &opts).unwrap();
    assert!(out
        .repairs
        .iter()
        .any(|r| r.code == DiagCode::MethodToKernel));
    assert!(!out
        .repairs
        .iter()
        .any(|r| r.code == DiagCode::NonStraightLine));
    assert!(out.source.contains("imax("), "{}", out.source);
    // `imax` is a cell-prelude kernel; append a stub so the bare compiler links it.
    let with_kernel = format!(
        "{}\nfn imax(a: u16, b: u16) -> u16 {{ let mut m = a; if b > a {{ m = b; }} m }}\n",
        out.source
    );
    assert_compiles(&with_kernel);
    // `as u32` forces the wide lane.
    let wide = full_canon("fn run(a: u16, b: u16) -> u16 { (a as u32) * (b as u32) }");
    assert!(wide.widened);
    assert!(wide.source.contains("-> u32"));
    assert_compiles(&wide.source);
}

#[test]
fn if_value_canonicalizes_and_normalizes_comparisons() {
    // `a > b` and `b < a` are one comparison; both spellings reach one schema.
    let gt = full_canon("fn run(a: u16, b: u16) -> u16 { if a > b { a } else { b } }");
    let lt = full_canon("fn run(a: u16, b: u16) -> u16 { if b < a { a } else { b } }");
    assert_eq!(gt.source, lt.source);
    assert!(gt.source.contains("if "), "{}", gt.source);
    assert!(!gt
        .repairs
        .iter()
        .any(|r| r.code == DiagCode::NonStraightLine));
    assert_compiles(&gt.source);
    // Symmetric comparisons sort operands.
    let a = full_canon("fn run(x: u16) -> u16 { if x == 5 { 1 } else { 2 } }");
    let b = full_canon("fn run(x: u16) -> u16 { if 5 == x { 1 } else { 2 } }");
    assert_eq!(a.source, b.source);
}

#[test]
fn guarded_division_stays_lazy() {
    // THE correctness constraint: the guard idiom must not evaluate its division
    // eagerly — `a / b` renders inside the arm, not hoisted above the `if`.
    let out = full_canon("fn run(a: u16, b: u16) -> u16 { if b != 0 { a / b } else { 0 } }");
    let src = &out.source;
    let if_pos = src.find("if ").expect("select emitted");
    let div_pos = src.find(" / ").expect("division emitted");
    assert!(
        div_pos > if_pos,
        "division must be inside the if-arm, not hoisted:\n{src}"
    );
    assert_compiles(src);
}

#[test]
fn shared_subexpressions_hoist_above_the_select() {
    // `s` feeds the condition and both arms — it hoists as a normal op; only the
    // arm-exclusive work stays inline.
    let out = full_canon(
        "fn run(a: u16, b: u16) -> u16 { let s = a + b; if s > 10 { s * 2 } else { s / 2 } }",
    );
    let src = &out.source;
    let sum_pos = src.find(" + ").expect("shared sum emitted");
    let if_pos = src.find("if ").expect("select emitted");
    assert!(sum_pos < if_pos, "shared node hoists above the if:\n{src}");
    assert_compiles(src);
}

#[test]
fn else_if_chains_nest_as_selects() {
    let out =
        full_canon("fn run(x: u16) -> u16 { if x > 100 { 3 } else if x > 10 { 2 } else { 1 } }");
    assert!(out.source.matches("if ").count() >= 2, "{}", out.source);
    assert!(!out
        .repairs
        .iter()
        .any(|r| r.code == DiagCode::NonStraightLine));
    assert_compiles(&out.source);
}

#[test]
fn verify_if_rewrites_to_computed_side() {
    // Registered amendment 2026-07-07 (E0207): granite's verify-not-compute shape
    // returns the computed side; the stated literal and zero arm are noise.
    let out = full_canon("fn run(a: u16) -> u16 { if a * 3 == 12 { 12 } else { 0 } }");
    assert!(out
        .repairs
        .iter()
        .any(|r| r.code == DiagCode::VerifyRewrite));
    assert!(out.source.contains("q0 * 3u16"), "{}", out.source);
    assert!(!out.source.contains("if "), "select gone:\n{}", out.source);
    assert_compiles(&out.source);
    // Same schema as writing the arithmetic directly.
    assert_eq!(
        out.source,
        full_canon("fn run(a: u16) -> u16 { a * 3 }").source
    );
    // A non-zero else arm is a real choice — no rewrite.
    let keep = full_canon("fn run(a: u16) -> u16 { if a * 3 == 12 { 12 } else { 5 } }");
    assert!(!keep
        .repairs
        .iter()
        .any(|r| r.code == DiagCode::VerifyRewrite));
    assert!(keep.source.contains("if "), "{}", keep.source);
}
