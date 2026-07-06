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
