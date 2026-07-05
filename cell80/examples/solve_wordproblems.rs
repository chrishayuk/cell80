//! **Test-driving `cell_solve` (M2) on hand-crafted word problems** — not the real M3
//! campaign (no model, no GSM8K corpus), but a small, honest check of what the loop
//! actually does with independently-authored problems: does precipitation (same artifact
//! hash ⇒ retrieved, not recompiled) show up across genuinely different problems, or only
//! within repeats of the *same* problem with different numbers (the only case the shipped
//! test suite exercises, `solve_answers_the_lego_problem`)?
//!
//! Answer, found here: precipitation is **literal-field-name-sensitive**. Two problems with
//! the identical underlying structure (multiply a quantity by a unit rate) hash identically
//! only if the extracted quantity *ids* also match — because those ids become literal Rust
//! struct field names in the renderer's output. Two problems using natural,
//! problem-specific names (`notebooks`/`notebook_price` vs `pencils`/`pencil_price`) render
//! to different source and never precipitate against each other, even though a human would
//! call them "the same schema." A real M3 extraction step would need to normalize
//! quantities to canonical role names (`qty`, `unit_price`, `total`, ...), not carry the
//! problem's own nouns forward, for the precipitation story to measure what it claims to.
//!
//! Run: `cargo run --release --example solve_wordproblems -p cell80`.

use cell80::plan::Plan;
use cell80::{CellHost, DEFAULT_CYCLES};

fn plan(json: &str) -> Plan {
    Plan::from_json(json).unwrap_or_else(|e| panic!("bad plan: {e}\n{json}"))
}

fn solve_one(host: &mut CellHost, label: &str, json: &str) {
    let rep = host
        .solve(&[plan(json)], DEFAULT_CYCLES)
        .unwrap_or_else(|e| panic!("solve error for {label}: {e}"));
    let o = &rep.outcomes[0];
    match (&o.answer, &o.kill) {
        (Some(a), _) => println!(
            "  {label:<28} answer={a:<8} artifact={:.12}  retrieved={}",
            o.artifact.as_deref().unwrap_or("-"),
            o.retrieved
        ),
        (None, Some(k)) => println!("  {label:<28} KILLED: {k}"),
        (None, None) => println!("  {label:<28} no answer, no kill (unexpected)"),
    }
}

fn main() {
    let mut host = CellHost::new();
    host.set_cache(true);

    println!("=== 1. Baseline: the spec's own lego example (mul, cents_per_count) ===");
    solve_one(
        &mut host,
        "lego (13 sets @ 1500c)",
        r#"{"quantities":[{"id":"lego_sets","value":13,"unit":"count"},
                          {"id":"lego_price","value":1500,"unit":"cents_per_count"}],
            "ops":[["mul","lego_sets","lego_price","lego_money"]],
            "target":"lego_money"}"#,
    );

    println!("\n=== 2. Same *schema*, generic field names, across two different word problems ===");
    // "Maya buys 6 notebooks at 45 cents each."
    solve_one(
        &mut host,
        "notebooks (generic ids)",
        r#"{"quantities":[{"id":"qty","value":6,"unit":"count"},
                          {"id":"unit_price","value":45,"unit":"cents_per_count"}],
            "ops":[["mul","qty","unit_price","total"]],
            "target":"total"}"#,
    );
    // "Jordan buys 9 pencils at 20 cents each." — same generic ids, different problem.
    solve_one(
        &mut host,
        "pencils (generic ids)",
        r#"{"quantities":[{"id":"qty","value":9,"unit":"count"},
                          {"id":"unit_price","value":20,"unit":"cents_per_count"}],
            "ops":[["mul","qty","unit_price","total"]],
            "target":"total"}"#,
    );

    println!("\n=== 3. Same schema, but *natural* problem-specific field names ===");
    // The same pencil problem, but the quantities keep the problem's own nouns —
    // this is what a naive "extract the actual names" model would likely produce.
    solve_one(
        &mut host,
        "pencils (natural ids)",
        r#"{"quantities":[{"id":"pencils","value":9,"unit":"count"},
                          {"id":"pencil_price","value":20,"unit":"cents_per_count"}],
            "ops":[["mul","pencils","pencil_price","pencil_total"]],
            "target":"pencil_total"}"#,
    );

    println!("\n=== 4. A two-op chain (mul then add), generic ids across two problems ===");
    // "4 shirts at 800 cents each, plus a flat 200-cent shipping fee."
    solve_one(
        &mut host,
        "shirts+shipping (generic)",
        r#"{"quantities":[{"id":"qty","value":4,"unit":"count"},
                          {"id":"unit_price","value":800,"unit":"cents_per_count"},
                          {"id":"fee","value":200,"unit":"cents"}],
            "ops":[["mul","qty","unit_price","subtotal"],["add","subtotal","fee","total"]],
            "target":"total"}"#,
    );
    // "7 books at 500 cents each, plus a flat 150-cent tax." — same schema, generic ids.
    solve_one(
        &mut host,
        "books+tax (generic)",
        r#"{"quantities":[{"id":"qty","value":7,"unit":"count"},
                          {"id":"unit_price","value":500,"unit":"cents_per_count"},
                          {"id":"fee","value":150,"unit":"cents"}],
            "ops":[["mul","qty","unit_price","subtotal"],["add","subtotal","fee","total"]],
            "target":"total"}"#,
    );

    println!("\n=== 5. Wage-rate word problem (cents_per_time × time, the units-pack flow) ===");
    // "Alex earns 1500 cents an hour and works 6 hours. How much does he earn?"
    solve_one(
        &mut host,
        "wage (alex, 6h)",
        r#"{"quantities":[{"id":"rate","value":1500,"unit":"cents_per_time"},
                          {"id":"worked","value":6,"unit":"time"}],
            "ops":[["mul","rate","worked","pay"]],
            "target":"pay"}"#,
    );
    // Different worker, same schema/ids — should retrieve.
    solve_one(
        &mut host,
        "wage (priya, 9h)",
        r#"{"quantities":[{"id":"rate","value":1200,"unit":"cents_per_time"},
                          {"id":"worked","value":9,"unit":"time"}],
            "ops":[["mul","rate","worked","pay"]],
            "target":"pay"}"#,
    );

    println!("\n=== 6. A wrong plan: exact_div violated (48 cookies into bags of 5) ===");
    solve_one(
        &mut host,
        "cookies (48 / 5, exact)",
        r#"{"quantities":[{"id":"cookies","value":48,"unit":"count"},
                          {"id":"bag_size","value":5,"unit":"count"}],
            "ops":[["div","cookies","bag_size","bags"]],
            "target":"bags",
            "constraints":[["exact_div","cookies","bag_size"]]}"#,
    );

    println!("\n=== 7. Multi-plan consensus + counterfactual battery (constructed) ===");
    // A deliberately constructed disagreement: three candidate extractions for one
    // "problem". Two share the (correct) mul schema; one is a coincidental agreement
    // that only matches at the original numbers (mirroring the project's own documented
    // min/median3 register-0 coincidence) — the battery's +1 perturbation sweep should
    // separate them and side with the 2-plan majority.
    let candidates = vec![
        plan(
            r#"{"quantities":[{"id":"a","value":4,"unit":"count"},{"id":"b","value":4,"unit":"count"}],
                "ops":[["mul","a","b","out"]], "target":"out"}"#,
        ),
        plan(
            r#"{"quantities":[{"id":"a","value":4,"unit":"count"},{"id":"b","value":4,"unit":"count"}],
                "ops":[["mul","b","a","out"]], "target":"out"}"#,
        ),
        plan(
            // add(4,4) == mul(4,4) == 8 -- wait, 4*4=16, 4+4=8. Not a coincidence at these
            // numbers; picked deliberately so the "wrong" plan is visibly wrong even
            // before perturbation, alongside a genuine coincidence at a=2,b=2 below.
            r#"{"quantities":[{"id":"a","value":4,"unit":"count"},{"id":"b","value":4,"unit":"count"}],
                "ops":[["add","a","b","out"]], "target":"out"}"#,
        ),
    ];
    let rep = host
        .solve(&candidates, DEFAULT_CYCLES)
        .expect("battery solve");
    println!(
        "  3 candidates (mul, mul-swapped, add) at a=b=4: consensus={:?} battery_ran={}",
        rep.answer, rep.battery_ran
    );
    for (i, o) in rep.outcomes.iter().enumerate() {
        println!("    plan {i}: answer={:?} kill={:?}", o.answer, o.kill);
    }

    // The genuine coincidence: a=2,b=2 -> mul=4, add=4 (equal at THESE numbers only).
    let coincidence = vec![
        plan(
            r#"{"quantities":[{"id":"a","value":2,"unit":"count"},{"id":"b","value":2,"unit":"count"}],
                "ops":[["mul","a","b","out"]], "target":"out"}"#,
        ),
        plan(
            r#"{"quantities":[{"id":"a","value":2,"unit":"count"},{"id":"b","value":2,"unit":"count"}],
                "ops":[["add","a","b","out"]], "target":"out"}"#,
        ),
    ];
    let rep2 = host
        .solve(&coincidence, DEFAULT_CYCLES)
        .expect("coincidence solve");
    println!(
        "\n  2 candidates (mul, add) at a=b=2 (agree at 4 before perturbing): consensus={:?} battery_ran={}",
        rep2.answer, rep2.battery_ran
    );
    for (i, o) in rep2.outcomes.iter().enumerate() {
        println!("    plan {i}: answer={:?} kill={:?}", o.answer, o.kill);
    }
}
