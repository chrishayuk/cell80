//! Demo of the **`cell_solve` loop** — M2 of the math campaign
//! (`docs/math-campaign-spec.md`).
//!
//! Run: `cargo run --release --example solve_demo -p cell80`
//!
//! The story in five beats: a model-extracted plan renders to canonical dialect
//! Rust and **the plan is a cell** (beat 1); the same schema with new numbers is
//! *retrieved by artifact hash*, not recompiled — deliberative once, reflexive
//! after (beat 2); candidate plans that disagree face the **counterfactual
//! battery** and the consistent majority wins (beat 3); bad plans are *killed with
//! a named reason*, never a wrong number (beat 4); and everything the loop ran is
//! exportable as a **`.facts` file** — claims anyone can re-verify by execution
//! (beat 5).
use cell80::plan::{plans_from_json, Plan};
use cell80::{CellHost, DEFAULT_CYCLES};

fn plan(json: &str) -> Plan {
    plans_from_json(json)
        .unwrap_or_else(|e| panic!("plan: {e}"))
        .remove(0)
}

fn main() {
    let mut host = CellHost::new();
    host.set_cache(true);

    // ── beat 1: a plan compiles to a cell ────────────────────────────────────
    // "13 lego sets at $15.00 each — how much money?" A model extracts this
    // (cents, never decimals; units tagged so the algebra is checkable):
    let lego = r#"{
        "quantities": [ {"id":"lego_sets","value":13,"unit":"count"},
                        {"id":"lego_price","value":1500,"unit":"cents_per_count"} ],
        "ops":        [ ["mul","lego_sets","lego_price","lego_money"] ],
        "target":     "lego_money" }"#;
    println!("── the rendered cell (canonical, deterministic) ──");
    println!("{}", plan(lego).render().unwrap());
    let rep = host.solve(&[plan(lego)], DEFAULT_CYCLES).unwrap();
    println!(
        "solve #1  answer: {:?} cents   retrieved: {}   (compiled fresh)",
        rep.answer.unwrap(),
        rep.outcomes[0].retrieved
    );

    // ── beat 2: precipitation — same schema, new numbers ─────────────────────
    let lego7 = lego.replace("\"value\":13", "\"value\":7");
    let rep = host.solve(&[plan(&lego7)], DEFAULT_CYCLES).unwrap();
    println!(
        "solve #2  answer: {:?} cents   retrieved: {}   (same schema ⇒ same hash ⇒ no compile)",
        rep.answer.unwrap(),
        rep.outcomes[0].retrieved
    );

    // ── beat 3: disagreeing candidates face the battery ──────────────────────
    // Two plans for "2 boxes and 3 boxes": one adds, one multiplies. On these
    // numbers they even *disagree* (5 vs 6); a third plan agrees with the adder
    // via a different route. The +1 perturbation sweep keeps the consistent
    // majority.
    let mk = |ops: &str, target: &str| {
        plan(&format!(
            r#"{{"quantities":[{{"id":"a","value":2,"unit":"count"}},{{"id":"b","value":3,"unit":"count"}}],
                "ops":[{ops}], "target":"{target}"}}"#
        ))
    };
    let adder = mk(r#"["add","a","b","c"]"#, "c");
    let muler = mk(r#"["mul","a","b","c"]"#, "c");
    let adder2 = mk(
        r#"["add","a","b","c"], ["add","c","b","d"], ["sub","d","b","e"]"#,
        "e",
    );
    let rep = host.solve(&[adder, muler, adder2], DEFAULT_CYCLES).unwrap();
    println!(
        "solve #3  answer: {:?}   battery ran: {}   (adders 2 v multiplier 1)",
        rep.answer.unwrap(),
        rep.battery_ran
    );

    // ── beat 4: bad plans die with the reason named ───────────────────────────
    let broke = plan(
        r#"{"quantities":[{"id":"have","value":300,"unit":"cents"},{"id":"spend","value":500,"unit":"cents"}],
            "ops":[["sub","have","spend","left"]], "target":"left"}"#,
    );
    let inexact = plan(
        r#"{"quantities":[{"id":"money","value":1000,"unit":"cents"},{"id":"price","value":300,"unit":"cents"}],
            "ops":[["div","money","price","n"]], "target":"n",
            "constraints":[["exact_div","money","price"]]}"#,
    );
    let rep = host.solve(&[broke, inexact], DEFAULT_CYCLES).unwrap();
    println!(
        "solve #4  answer: {:?}   (both plans killed — escalate)",
        rep.answer
    );
    for (i, o) in rep.outcomes.iter().enumerate() {
        println!("          plan {i} killed: {}", o.kill.as_deref().unwrap());
    }
    // Unit mismatches die even earlier — before compilation:
    let confused = plan(
        r#"{"quantities":[{"id":"money","value":5,"unit":"cents"},{"id":"wait","value":2,"unit":"hours"}],
            "ops":[["add","money","wait","x"]], "target":"x"}"#,
    );
    println!(
        "          cents + hours: {}",
        confused.render().unwrap_err()
    );

    // ── beat 5: the residue ───────────────────────────────────────────────────
    let mut buf = Vec::new();
    let n = host.export_facts(&mut buf, "solve-demo").unwrap();
    println!("\n── the residue: {n} verified facts (spot-checkable by re-execution) ──");
    for line in String::from_utf8(buf).unwrap().lines().skip(1).take(3) {
        println!("{line}");
    }
    println!("...");
}
