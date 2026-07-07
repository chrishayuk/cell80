//! M2.9 — `cell80 compose`: the link loop (unknown call target → search + arity →
//! inline → recompile), the registered agreement gate (unanimous / majority-flagged /
//! escalate), fact provenance, and the precipitation counter on composed schemas.

use cell80::compose::{agreement, compose, run_composed};
use cell80::{run_cli, CellHost, DEFAULT_CYCLES};
use std::path::Path;

fn cells_dir() -> String {
    format!("{}/cells", env!("CARGO_MANIFEST_DIR"))
}

fn library() -> CellHost {
    // Small but real: load the actual library dir the CLI would.
    let out = run_cli(&["index".into(), cells_dir()]).unwrap();
    assert!(out.contains("cells"));
    // host_from_dir is private; go through compose's public path in the tests below.
    CellHost::new()
}

/// Build a host over the real library the way cmd_compose does (via the CLI).
fn tmp(name: &str, src: &str) -> String {
    let dir = std::env::temp_dir().join(format!("cell80-compose-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(name);
    std::fs::write(&p, src).unwrap();
    p.to_str().unwrap().to_string()
}

#[test]
fn compose_links_a_library_call_and_runs() {
    let src = tmp(
        "flag.rs",
        "fn run(a: u16, b: u16) -> u16 { let flag = is_gt(a, b); flag * 100 + 7 }",
    );
    let out = run_cli(&[
        "compose".into(),
        cells_dir(),
        src.clone(),
        "--args".into(),
        "5,3".into(),
    ])
    .unwrap();
    assert!(out.contains("answer: 107"), "{out}");
    assert!(out.contains("`is_gt` -> is_gt"), "resolution named: {out}");
    // And the other branch of the predicate.
    let out = run_cli(&[
        "compose".into(),
        cells_dir(),
        src,
        "--args".into(),
        "2,9".into(),
    ])
    .unwrap();
    assert!(out.contains("answer: 7"), "{out}");
}

#[test]
fn agreement_gate_unanimous_majority_escalate() {
    // The registered rule directly.
    assert_eq!(
        agreement(&[Some(9), Some(9), Some(9)]),
        (Some(9), "unanimous", false)
    );
    assert_eq!(
        agreement(&[Some(9), Some(9), Some(13)]),
        (Some(9), "majority", true),
        "2-of-3 accepts AND flags"
    );
    assert_eq!(agreement(&[Some(9), Some(13)]), (None, "escalate", false));
    assert_eq!(
        agreement(&[Some(9), None, Some(13)]),
        (None, "escalate", false)
    );
    assert_eq!(agreement(&[None, None]), (None, "escalate", false));
    assert_eq!(agreement(&[Some(4)]), (Some(4), "single", false));
}

#[test]
fn zero_guard_registered_amendment() {
    // The row22 class: two broken derivations collapse to 0 and "agree" — the
    // registered zero-guard (2026-07-06) escalates instead of accepting.
    assert_eq!(
        agreement(&[Some(0), Some(0), None]),
        (None, "degenerate_zero", false)
    );
    assert_eq!(
        agreement(&[Some(0), Some(0), Some(0)]),
        (None, "degenerate_zero", false)
    );
    assert_eq!(agreement(&[Some(0)]), (None, "degenerate_zero", false));
    // A nonzero majority over a zero minority is untouched.
    assert_eq!(
        agreement(&[Some(0), Some(14), Some(14)]),
        (Some(14), "majority", true)
    );
}

#[test]
fn cross_check_two_derivations_end_to_end() {
    // An inline derivation (if-value: light fallback, still compiles) and a
    // composed derivation (library `max`) — method diversity, one answer.
    let inline = tmp(
        "inline_max.rs",
        "fn run(a: u16, b: u16) -> u16 { if a > b { a } else { b } }",
    );
    let composed = tmp("lib_max.rs", "fn run(a: u16, b: u16) -> u16 { max(a, b) }");
    let out = run_cli(&[
        "compose".into(),
        cells_dir(),
        inline.clone(),
        composed.clone(),
        "--args".into(),
        "4,9".into(),
        "--json".into(),
    ])
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["answer"], 9, "{out}");
    assert_eq!(v["agreement"], "unanimous", "{out}");
    assert_eq!(v["flagged"], false);
    // A third, disagreeing derivation → majority, accepted but flagged.
    let adder = tmp("adder.rs", "fn run(a: u16, b: u16) -> u16 { a + b }");
    let out = run_cli(&[
        "compose".into(),
        cells_dir(),
        inline,
        composed,
        adder,
        "--args".into(),
        "4,9".into(),
        "--json".into(),
    ])
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["answer"], 9);
    assert_eq!(v["agreement"], "majority");
    assert_eq!(v["flagged"], true);
}

#[test]
fn composed_schema_precipitates_on_reuse() {
    // The same structure spelled with different nouns composes to the SAME
    // artifact — the second derivation is retrieved, not recompiled (H-M3).
    let a = tmp(
        "pencils_c.rs",
        "fn run(pencils: u16, boxes: u16) -> u16 { let per = pencils * 3; per + boxes }",
    );
    let b = tmp(
        "notebooks_c.rs",
        "fn run(notebooks: u16, crates: u16) -> u16 { let st = notebooks * 3; st + crates }",
    );
    let out = run_cli(&[
        "compose".into(),
        cells_dir(),
        a,
        b,
        "--args".into(),
        "2,5".into(),
        "--json".into(),
    ])
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["answer"], 11);
    assert_eq!(v["agreement"], "unanimous");
    let d = v["derivations"].as_array().unwrap();
    assert_eq!(d[0]["artifact"], d[1]["artifact"], "same schema, same hash");
    assert_eq!(d[0]["retrieved"], false, "first sighting compiles");
    assert_eq!(d[1]["retrieved"], true, "second sighting is retrieved");
}

#[test]
fn typed_failures_escalate_not_guess() {
    // Constant division by a cancelled-to-zero denominator dies at canon with a
    // typed code, and the gate reports escalate — never a made-up number.
    let bad = tmp("div0.rs", "fn run(a: u16, b: u16) -> u16 { a / (b - b) }");
    let out = run_cli(&[
        "compose".into(),
        cells_dir(),
        bad,
        "--args".into(),
        "6,3".into(),
    ])
    .unwrap();
    assert!(out.contains("escalate"), "{out}");
    assert!(out.contains("E0302"), "typed code surfaces: {out}");
    // An unresolvable call target names itself.
    let ghost = tmp("ghost.rs", "fn run(a: u16) -> u16 { zorbulate_qq(a) }");
    let out = run_cli(&["compose".into(), cells_dir(), ghost]).unwrap();
    assert!(out.contains("escalate"), "{out}");
}

#[test]
fn facts_are_left_behind_for_accepted_answers() {
    let src = tmp("fact_src.rs", "fn run(a: u16, b: u16) -> u16 { max(a, b) }");
    let facts = tmp("compose.facts", "");
    let out = run_cli(&[
        "compose".into(),
        cells_dir(),
        src,
        "--args".into(),
        "4,9".into(),
        "--facts".into(),
        facts.clone(),
    ])
    .unwrap();
    assert!(out.contains("answer: 9"), "{out}");
    let text = std::fs::read_to_string(&facts).unwrap();
    assert!(text.contains("\"facts\":1"), "fact header present: {text}");
    assert!(text.contains("compose@cell80"), "{text}");
}

#[test]
fn compose_api_surface_is_usable_directly() {
    // The library API (not just the CLI): compose against a host, run, inspect.
    let mut host = library();
    // Empty host: resolution fails with the typed no-match error.
    let err = match compose(
        &host,
        Path::new(&cells_dir()),
        "fn run(a: u16, b: u16) -> u16 { max(a, b) }",
    ) {
        Err(e) => e,
        Ok(_) => panic!("must fail against an empty host"),
    };
    assert!(err.contains("E0504"), "{err}");
    // A source with no calls composes against an empty host just fine.
    let comp = compose(
        &host,
        Path::new(&cells_dir()),
        "fn run(a: u16, b: u16) -> u16 { a * 2 + b }",
    )
    .unwrap();
    assert!(comp.resolutions.is_empty());
    let out = run_composed(&mut host, comp, &[10, 3], DEFAULT_CYCLES).unwrap();
    assert_eq!(out.answer, Some(23));
    assert!(out.kill.is_none());
}

#[test]
fn battery_kills_coincidental_agreement_on_composed_cells() {
    // The plan-solve battery, ported to the composed path via literal lifting:
    // a+b and a*b agree at (2,2) — 4 == 4 — but shatter under perturbation.
    let adder = tmp(
        "bat_add.rs",
        "fn run() -> u16 { let a = 2; let b = 2; a + b }",
    );
    let muler = tmp(
        "bat_mul.rs",
        "fn run() -> u16 { let a = 2; let b = 2; a * b }",
    );
    let out = run_cli(&["compose".into(), cells_dir(), adder, muler, "--json".into()]).unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["agreement"], "battery_escalate", "{out}");
    assert!(v["answer"].is_null(), "no confident 4: {out}");
}

#[test]
fn battery_passes_real_agreement_and_lifted_args_auto_run() {
    // Two genuinely different structures computing the same function: max+min == a+b.
    // Lifted quantities supply the arguments (no --args), and the agreement must
    // survive perturbation of both values.
    let direct = tmp(
        "bat_sum.rs",
        "fn run() -> u16 { let a = 4; let b = 9; a + b }",
    );
    let split = tmp(
        "bat_maxmin.rs",
        "fn run() -> u16 { let a = 4; let b = 9; let hi = imax(a, b); let lo = imin(a, b); hi + lo }",
    );
    let out = run_cli(&[
        "compose".into(),
        cells_dir(),
        direct,
        split,
        "--json".into(),
    ])
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["answer"], 13, "{out}");
    assert_eq!(v["agreement"], "unanimous");
    let battery = v["battery"].as_str().unwrap();
    assert!(battery.contains("survived 2 perturbation"), "{out}");
    // Distinct artifacts — this is two schemas agreeing, not one schema twice.
    let d = v["derivations"].as_array().unwrap();
    assert_ne!(d[0]["artifact"], d[1]["artifact"]);
}

#[test]
fn lifting_precipitates_across_problem_instances() {
    // Same structure, different numbers ⇒ the SAME composed artifact, retrieved on
    // the second sighting — precipitation across problem instances, not just nouns.
    let one = tmp(
        "lift_a.rs",
        "fn run() -> u16 { let x = 30; let y = 5; x * y }",
    );
    let two = tmp(
        "lift_b.rs",
        "fn run() -> u16 { let p = 12; let q = 7; p * q }",
    );
    let out = run_cli(&["compose".into(), cells_dir(), one, two, "--json".into()]).unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let d = v["derivations"].as_array().unwrap();
    assert_eq!(
        d[0]["artifact"], d[1]["artifact"],
        "one schema, two instances"
    );
    assert_eq!(d[1]["retrieved"], true, "{out}");
    // Different numbers ⇒ different answers ⇒ the gate correctly escalates; the
    // point here is the schema economy, not agreement.
    assert_eq!(d[0]["answer"], 150);
    assert_eq!(d[1]["answer"], 84);
}

#[test]
fn guarded_division_survives_canonicalization_end_to_end() {
    // The safe-div idiom with the divisor at zero: the canonical select must keep
    // the division lazy in its arm — answer 0, never a div_by_zero kill.
    let src = tmp(
        "guard.rs",
        "fn run(a: u16, b: u16) -> u16 { if b != 0 { a / b } else { 0 } }",
    );
    let out = run_cli(&[
        "compose".into(),
        cells_dir(),
        src.clone(),
        "--args".into(),
        "5,0".into(),
        "--json".into(),
    ])
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["derivations"][0]["kill"].is_null(), "{out}");
    // answer 0 → the zero-guard reports degenerate_zero rather than accepting a
    // lone zero — correct: a legit zero answer escalates by registered rule.
    assert_eq!(v["agreement"], "degenerate_zero", "{out}");
    // And with a nonzero divisor the value flows.
    let out = run_cli(&[
        "compose".into(),
        cells_dir(),
        src,
        "--args".into(),
        "12,3".into(),
    ])
    .unwrap();
    assert!(out.contains("answer: 4"), "{out}");
}

// ---------------------------------------------------------- coverage: edges

#[test]
fn battery_reports_skipped_values_for_partial_lifts() {
    // d0 lifts {6, 2}; d1 bakes the 2 inline (expression constant) — only the
    // common value 6 is perturbable; 2 is skipped and reported, never guessed at.
    let a = tmp(
        "skip_a.rs",
        "fn run() -> u16 { let x = 6; let y = 2; x * y }",
    );
    let b = tmp("skip_b.rs", "fn run() -> u16 { let x = 6; x + x }");
    let out = run_cli(&["compose".into(), cells_dir(), a, b, "--json".into()]).unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["answer"], 12);
    let battery = v["battery"].as_str().unwrap();
    assert!(battery.contains("skipped"), "{out}");
}

#[test]
fn linker_refuses_state_cell_only_matches_with_a_typed_error() {
    // `agree3_u32` exists only as a state cell — naming it as a call must produce
    // the typed E0503 refusal, not an inline of something un-callable.
    let src = tmp(
        "statecall.rs",
        "fn run(a: u16, b: u16) -> u16 { agree3_u32(a, b) }",
    );
    let out = run_cli(&["compose".into(), cells_dir(), src]).unwrap();
    assert!(out.contains("escalate"), "{out}");
    assert!(out.contains("E0503") || out.contains("E0504"), "{out}");
}

#[test]
fn compose_cli_flag_errors_are_named() {
    assert!(run_cli(&["compose".into(), cells_dir()]).is_err());
    assert!(run_cli(&[
        "compose".into(),
        cells_dir(),
        "x.rs".into(),
        "--bogus".into()
    ])
    .is_err());
    assert!(run_cli(&["compose".into(), cells_dir(), "/nonexistent-file.rs".into()]).is_err());
}

#[test]
fn link_budget_exhausts_with_a_named_error_and_battery_needs_handles() {
    // Thirteen distinct library calls exceed the 12-iteration link budget.
    let calls: Vec<String> = [
        "is_gt", "is_lt", "is_le", "is_ge", "min", "max", "lcm", "safe_div", "safe_mod", "percent",
        "gcd3", "min3", "divides",
    ]
    .iter()
    .map(|n| format!("{n}(a, b)"))
    .collect();
    let src = tmp(
        "budget.rs",
        &format!("fn run(a: u16, b: u16) -> u16 {{ {} }}", calls.join(" + ")),
    );
    let out = run_cli(&["compose".into(), cells_dir(), src]).unwrap();
    assert!(out.contains("link budget exhausted"), "{out}");
    // battery() on outcomes without warm handles is a named error.
    let mut host = CellHost::new();
    let orphan = cell80::compose::DerivationOutcome {
        answer: Some(1),
        kill: None,
        artifact: None,
        resolutions: vec![],
        repairs: vec![],
        retrieved: false,
        handle: None,
        base_args: vec![],
        lifted: vec![("q0".into(), 1)],
        wide_ret: false,
    };
    let err = cell80::compose::battery(&mut host, &[&orphan], DEFAULT_CYCLES).unwrap_err();
    assert!(err.contains("warm handles"), "{err}");
}

#[test]
fn runtime_halts_are_named_kills() {
    // Unguarded division by a zero argument: div_by_zero, never a number.
    let src = tmp("rawdiv.rs", "fn run(a: u16, b: u16) -> u16 { a / b }");
    let out = run_cli(&[
        "compose".into(),
        cells_dir(),
        src.clone(),
        "--args".into(),
        "5,0".into(),
    ])
    .unwrap();
    assert!(out.contains("div_by_zero"), "{out}");
    // A one-cycle budget starves the run: cycle_budget, never a number.
    let out = run_cli(&[
        "compose".into(),
        cells_dir(),
        src,
        "--args".into(),
        "6,3".into(),
        "--cycles".into(),
        "1".into(),
    ])
    .unwrap();
    assert!(out.contains("cycle_budget"), "{out}");
    // A lone dead derivation is `single` with no answer.
    assert_eq!(agreement(&[None]), (None, "single", false));
}
