//! M2.5 acceptance tests — the seven registered assertions from
//! `docs/math-campaign-amendment.md` (all must be green before M3 spends compute):
//!
//! 1. pencils/notebooks — same structure, different nouns → identical artifact hash
//! 2. slot-order stability — permuted-but-equivalent op order → identical hash
//! 3. dollars / $16.50 mixed — canonical plan is cents-only, factor recorded
//! 4. reserved identifiers (`final`, `try`, `union`) — render cleanly via slots
//! 5. rate nouns — `numerator_per_denominator` unit emitted
//! 6. unknown nouns (sheep, cups, GB) — `count` convention applied
//! 7. defer-division parity — both spellings produce the identical canonical plan
//!    (cross-language parity against the Python `ast` arm runs in the M2.8 harness)

use cell80::plan::Plan;
use cell80::{Cartridge, CartridgeOpts, CellConfig, CellHost, DEFAULT_CYCLES};
use rustz80::{CanonMode, UnitHint};

fn plan(json: &str) -> Plan {
    Plan::from_json(json).expect("plan parses")
}

/// Compile direct-Rust through the campaign path: Full canonicalization.
fn compile_full(id: &str, src: &str, hints: Vec<UnitHint>) -> Cartridge {
    Cartridge::compile(
        src,
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some(id.into()),
            canon: CanonMode::Full,
            canon_hints: hints,
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("compiles: {e}\n{src}"))
}

fn hash_full(src: &str) -> [u8; 32] {
    compile_full("t", src, Vec::new()).artifact_hash()
}

// 1 ─────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_1_same_structure_different_nouns_identical_hash() {
    // Direct-Rust extraction path.
    let pencils = "fn run(pencils: u16, boxes: u16) -> u16 { let per_box = pencils * 3; let total = per_box + boxes; total }";
    let notebooks = "fn run(notebooks: u16, crates: u16) -> u16 { let stack = notebooks * 3; let sum = stack + crates; sum }";
    assert_eq!(
        hash_full(pencils),
        hash_full(notebooks),
        "identical structure must hash identically whatever the nouns"
    );
    // Plan-IR path: same ops, different quantity ids ⇒ byte-identical render.
    let a = plan(
        r#"{"quantities":[{"id":"pencils","value":13,"unit":"count"},{"id":"boxes","value":4,"unit":"count"}],
            "ops":[["mul","pencils","boxes","total"]], "target":"total"}"#,
    );
    let b = plan(
        r#"{"quantities":[{"id":"notebooks","value":13,"unit":"count"},{"id":"crates","value":4,"unit":"count"}],
            "ops":[["mul","notebooks","crates","stacked"]], "target":"stacked"}"#,
    );
    assert_eq!(a.render().unwrap(), b.render().unwrap());
}

// 2 ─────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_2_slot_order_stability() {
    // Plan-IR: two independent ops emitted in either order → identical render
    // (the add ranks before the mul in the tie-break, whatever the model's order).
    let one = plan(
        r#"{"quantities":[{"id":"a","value":2,"unit":"count"},{"id":"s","value":3,"unit":"scalar"}],
            "ops":[["mul","a","s","x"],["add","a","a","y"],["add","x","y","z"]], "target":"z"}"#,
    );
    let two = plan(
        r#"{"quantities":[{"id":"a","value":2,"unit":"count"},{"id":"s","value":3,"unit":"scalar"}],
            "ops":[["add","a","a","y"],["mul","a","s","x"],["add","x","y","z"]], "target":"z"}"#,
    );
    assert_eq!(one.render().unwrap(), two.render().unwrap());
    // Direct-Rust: permuted independent lets → identical artifact hash.
    let p = "fn run(a: u16, b: u16) -> u16 { let x = a * 2; let y = b * 5; x + y }";
    let q = "fn run(a: u16, b: u16) -> u16 { let y = b * 5; let x = a * 2; x + y }";
    assert_eq!(hash_full(p), hash_full(q));
}

// 3 ─────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_3_mixed_money_canonicalizes_to_cents_with_factor_recorded() {
    // Plan-IR: dollars and cents mixed — the base-scale table makes it cents-only.
    let mut p = plan(
        r#"{"quantities":[{"id":"bill","value":16,"unit":"dollars"},{"id":"tip","value":50,"unit":"cents"}],
            "ops":[["add","bill","tip","paid"]], "target":"paid"}"#,
    );
    let repairs = p.normalize_units().unwrap();
    assert!(p.quantities.iter().all(|q| q.unit == "cents"), "{p:?}");
    assert_eq!(p.quantities[0].value, 1600);
    assert!(
        repairs.iter().any(|r| r.contains("factor=100")),
        "{repairs:?}"
    );
    // And the whole solve path agrees end-to-end.
    let mut host = CellHost::new();
    host.set_cache(true);
    let rep = host
        .solve(
            &[plan(
                r#"{"quantities":[{"id":"bill","value":16,"unit":"dollars"},{"id":"tip","value":50,"unit":"cents"}],
                    "ops":[["add","bill","tip","paid"]], "target":"paid"}"#,
            )],
            DEFAULT_CYCLES,
        )
        .unwrap();
    assert_eq!(rep.answer, Some(1650));
    assert!(rep.outcomes[0]
        .repairs
        .iter()
        .any(|r| r.contains("unit_scaled") && r.contains("factor=100")));
    // Direct-Rust: a hinted $16.50 literal hashes identically to its pre-scaled
    // cents spelling — the scale is applied before anything downstream sees it.
    let decimal = compile_full(
        "t",
        "fn run(n: u16) -> u16 { let price = 16.50; price * n }",
        vec![UnitHint {
            ident: "price".into(),
            unit: "dollars".into(),
        }],
    );
    let cents = compile_full("t", "fn run(n: u16) -> u16 { n * 1650 }", Vec::new());
    assert_eq!(decimal.artifact_hash(), cents.artifact_hash());
    assert!(decimal
        .canon_repairs
        .iter()
        .any(|r| r.to_string().contains("factor=100")));
}

// 4 ─────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_4_reserved_identifiers_render_via_slots() {
    for id in ["final", "try", "union"] {
        let json = format!(
            r#"{{"quantities":[{{"id":"{id}","value":3,"unit":"count"}},{{"id":"n","value":4,"unit":"count"}}],
                "ops":[["mul","{id}","n","out"]], "target":"out"}}"#
        );
        let p = plan(&json);
        let rendered = p
            .render_canonical()
            .unwrap_or_else(|e| panic!("`{id}` must render cleanly via slots: {e}"));
        assert!(
            !rendered.src.contains(id),
            "`{id}` must never reach the Rust:\n{}",
            rendered.src
        );
        // …and it compiles all the way to a cartridge (no rustc-keyword parse trap).
        Cartridge::compile(
            &rendered.src,
            CellConfig::sandboxed(),
            CartridgeOpts {
                entry: Some("P::run".into()),
                ..Default::default()
            },
        )
        .unwrap_or_else(|e| panic!("`{id}` cell must compile: {e}"));
        // The noun survives as metadata.
        assert!(rendered.renames.iter().any(|(n, s)| n == id && s == "q0"));
    }
}

// 5 ─────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_5_rate_nouns_emit_numerator_per_denominator() {
    let mut p = plan(
        r#"{"quantities":[{"id":"price","value":2,"unit":"dollars_per_egg"},{"id":"eggs","value":12,"unit":"eggs"}],
            "ops":[["mul","price","eggs","cost"]], "target":"cost"}"#,
    );
    let repairs = p.normalize_units().unwrap();
    assert_eq!(p.quantities[0].unit, "cents_per_count", "{repairs:?}");
    assert_eq!(p.quantities[0].value, 200, "dollars→cents ×100 on the rate");
    assert_eq!(p.quantities[1].unit, "count");
    // The canonical unit is the explicit numerator_per_denominator form, and the
    // downstream unit algebra accepts it: cents_per_count × count → cents.
    let rendered = p.render_canonical().unwrap();
    assert!(rendered.src.contains("struct P"));
}

// 6 ─────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_6_unknown_nouns_take_the_count_convention() {
    let mut p = plan(
        r#"{"quantities":[{"id":"flock","value":30,"unit":"sheep"},{"id":"pens","value":5,"unit":"cups"},{"id":"disk","value":2,"unit":"gb"}],
            "ops":[["div","flock","pens","per"],["mul","per","disk","out"]], "target":"out"}"#,
    );
    p.normalize_units().unwrap();
    assert!(
        p.quantities.iter().all(|q| q.unit == "count"),
        "unknown nouns are counts: {:?}",
        p.quantities
    );
    // And solve accepts what used to be an unknown-unit render reject.
    let mut host = CellHost::new();
    host.set_cache(true);
    let rep = host
        .solve(
            &[plan(
                r#"{"quantities":[{"id":"flock","value":30,"unit":"sheep"},{"id":"pens","value":5,"unit":"cups"}],
                    "ops":[["div","flock","pens","per"]], "target":"per"}"#,
            )],
            DEFAULT_CYCLES,
        )
        .unwrap();
    assert_eq!(rep.answer, Some(6));
}

// 7 ─────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_7_defer_division_parity() {
    // Both spellings of "30% of a" — early-truncating and late-dividing — reach
    // one canonical plan: multiply first, divide once at the end.
    let early = "fn run(a: u16) -> u16 { a / 100 * 30 }";
    let late = "fn run(a: u16) -> u16 { a * 30 / 100 }";
    // The manifest id is part of the artifact hash — compare under one id.
    assert_eq!(hash_full(early), hash_full(late));
    let ce = compile_full("early", early, Vec::new());
    let cl = compile_full("late", late, Vec::new());
    // Semantics on the emulator: a = 250 → 75 (the deferred order), not the
    // early-truncation 60. The repair is deterministic and recorded.
    let mut host = CellHost::new();
    host.add(ce);
    let h = host.handle_for("early").unwrap();
    let fast = host.run_fast(h, &[250], DEFAULT_CYCLES).unwrap();
    assert_eq!(fast.result, 75, "defer-division fixes the precision loss");
    assert!(cl
        .canon_repairs
        .iter()
        .any(|r| r.to_string().contains("defer_division")));
}
