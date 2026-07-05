//! **A real M3 smoke test** — 8 genuine problems from GSM8K's public test set (the literal
//! first 8 rows of `openai/grade-school-math`'s `test.jsonl`, fetched fresh, not cherry-picked
//! or written to match a known schema), each hand-extracted into the plan IR by *reading the
//! English and doing the extraction the spec asks a model to do* — not designing the plan
//! around a schema I already knew would compile. This is still not the real M3 (no model in
//! the loop, N=8 not 1,319, no perturbation-battery corpus sweep), but it's the first time this
//! project's `cell_solve` loop has been checked against problems it didn't design, with known
//! ground-truth answers to verify against.
//!
//! Run: `cargo run --release --example m3_gsm8k_smoketest -p cell80`.

use cell80::plan::Plan;
use cell80::{CellHost, DEFAULT_CYCLES};

fn plan(json: &str) -> Plan {
    Plan::from_json(json).unwrap_or_else(|e| panic!("bad plan: {e}\n{json}"))
}

fn check(host: &mut CellHost, label: &str, json: &str, expected: u64, question: &str) {
    let rep = host
        .solve(&[plan(json)], DEFAULT_CYCLES)
        .unwrap_or_else(|e| panic!("solve error for {label}: {e}"));
    let o = &rep.outcomes[0];
    let verdict = match o.answer {
        Some(a) if a == expected => "CORRECT".to_string(),
        Some(a) => format!("WRONG (got {a}, want {expected})"),
        None => format!(
            "ESCALATED: {}",
            o.kill.as_deref().unwrap_or("no answer, no kill")
        ),
    };
    println!("  {label:<12} {verdict:<28} retrieved={:<5} \"{question}\"", o.retrieved);
}

fn main() {
    let mut host = CellHost::new();
    host.set_cache(true);

    println!("=== 8 real GSM8K test-set problems (openai/grade-school-math, rows 1-8) ===\n");

    // 1. Janet's ducks (GSM8K #1). "16 eggs, eats 3, bakes 4, sells rest at $2." Answer: 18.
    check(
        &mut host,
        "janet",
        r#"{"quantities":[{"id":"total_eggs","value":16,"unit":"count"},
                          {"id":"breakfast","value":3,"unit":"count"},
                          {"id":"muffins","value":4,"unit":"count"},
                          {"id":"price","value":2,"unit":"money_per_count"}],
            "ops":[["sub","total_eggs","breakfast","after_breakfast"],
                   ["sub","after_breakfast","muffins","sold"],
                   ["mul","sold","price","dollars"]],
            "target":"dollars"}"#,
        18,
        "Janet's ducks lay 16 eggs/day; eats 3, bakes 4 into muffins, sells rest at $2/egg.",
    );

    // 2. Robe fiber (GSM8K #2). "2 bolts blue, half that white." Answer: 3.
    check(
        &mut host,
        "robe",
        r#"{"quantities":[{"id":"blue","value":2,"unit":"count"},
                          {"id":"two","value":2,"unit":"scalar"}],
            "ops":[["div","blue","two","white"],["add","blue","white","total"]],
            "target":"total"}"#,
        3,
        "A robe takes 2 bolts of blue fiber and half that much white fiber. Total bolts?",
    );

    // 3. Josh house flip (GSM8K #3). "$80k house + $50k repairs, value +150%." Answer: 70000.
    check(
        &mut host,
        "josh",
        r#"{"quantities":[{"id":"house_price","value":80000,"unit":"money"},
                          {"id":"repairs","value":50000,"unit":"money"},
                          {"id":"pct","value":150,"unit":"scalar"},
                          {"id":"hundred","value":100,"unit":"scalar"}],
            "ops":[["add","house_price","repairs","cost"],
                   ["mul","house_price","pct","scaled"],
                   ["div","scaled","hundred","increase"],
                   ["add","increase","house_price","new_value"],
                   ["sub","new_value","cost","profit"]],
            "target":"profit"}"#,
        70000,
        "Josh buys an $80,000 house, $50,000 repairs, value increases 150%. Profit?",
    );

    // 4. James sprints (GSM8K #4). "3 sprints x3/week, 60m each." Answer: 540.
    check(
        &mut host,
        "james",
        r#"{"quantities":[{"id":"sprints_per_session","value":3,"unit":"count"},
                          {"id":"sessions_per_week","value":3,"unit":"count"},
                          {"id":"meters_per_sprint","value":60,"unit":"distance_per_count"}],
            "ops":[["mul","sprints_per_session","sessions_per_week","sprints_per_week"],
                   ["mul","sprints_per_week","meters_per_sprint","total_meters"]],
            "target":"total_meters"}"#,
        540,
        "James runs 3 sprints 3 times a week, 60 meters each. Total meters/week?",
    );

    // 5. Wendi's chickens (GSM8K #5). "20 chickens x3 cups, minus 15+25 fed so far." Answer: 20.
    check(
        &mut host,
        "wendi",
        r#"{"quantities":[{"id":"chickens","value":20,"unit":"count"},
                          {"id":"cups_per_chicken","value":3,"unit":"count_per_count"},
                          {"id":"morning","value":15,"unit":"count"},
                          {"id":"afternoon","value":25,"unit":"count"}],
            "ops":[["mul","chickens","cups_per_chicken","total_needed"],
                   ["add","morning","afternoon","given"],
                   ["sub","total_needed","given","final_meal"]],
            "target":"final_meal"}"#,
        20,
        "20 chickens need 3 cups each/day; fed 15 (AM) + 25 (PM). Final meal?",
    );

    // 6. Kylar's glasses (GSM8K #6). "$5 each, every 2nd is 60% price, buys 16." Answer: 64.
    check(
        &mut host,
        "kylar",
        r#"{"quantities":[{"id":"price","value":5,"unit":"money"},
                          {"id":"disc_pct","value":60,"unit":"scalar"},
                          {"id":"hundred","value":100,"unit":"scalar"},
                          {"id":"qty","value":16,"unit":"count"},
                          {"id":"two","value":2,"unit":"scalar"}],
            "ops":[["mul","price","disc_pct","scaled"],
                   ["div","scaled","hundred","discount_price"],
                   ["div","qty","two","half_count"],
                   ["mul","half_count","discount_price","cheaper_total"],
                   ["mul","half_count","price","regular_total"],
                   ["add","cheaper_total","regular_total","total"]],
            "target":"total"}"#,
        64,
        "Glasses $5 each, every 2nd glass 60% price, buys 16. Total cost?",
    );

    // 7. Toulouse/Charleston/Seattle sheep (GSM8K #7). Answer: 260.
    check(
        &mut host,
        "sheep",
        r#"{"quantities":[{"id":"seattle","value":20,"unit":"count"},
                          {"id":"four","value":4,"unit":"scalar"},
                          {"id":"two","value":2,"unit":"scalar"}],
            "ops":[["mul","seattle","four","charleston"],
                   ["mul","charleston","two","toulouse"],
                   ["add","seattle","charleston","partial"],
                   ["add","partial","toulouse","total"]],
            "target":"total"}"#,
        260,
        "Toulouse has 2x Charleston's sheep, Charleston has 4x Seattle's (20). Total?",
    );

    // 8. Carla's download (GSM8K #8). "200GB @ 2GB/min, restart at 40%, +20min wait." Answer: 160.
    check(
        &mut host,
        "carla",
        r#"{"quantities":[{"id":"file_size","value":200,"unit":"count"},
                          {"id":"rate","value":2,"unit":"count_per_time"},
                          {"id":"pct","value":40,"unit":"scalar"},
                          {"id":"hundred","value":100,"unit":"scalar"},
                          {"id":"restart_wait","value":20,"unit":"time"}],
            "ops":[["mul","file_size","pct","scaled"],
                   ["div","scaled","hundred","partial_size"],
                   ["div","partial_size","rate","time_to_restart"],
                   ["div","file_size","rate","time_full"],
                   ["add","time_to_restart","time_full","partial_time"],
                   ["add","partial_time","restart_wait","total_time"]],
            "target":"total_time"}"#,
        160,
        "200GB @ 2GB/min, restarts (loses progress) at 40%, restart itself takes 20min.",
    );

    println!("\n=== Mini robustness check: same schema, different numbers (GSM-Symbolic-style) ===\n");
    // James's problem, same ids/structure, new numbers: 4 sprints x5/week, 70m each -> 1400.
    check(
        &mut host,
        "james-v2",
        r#"{"quantities":[{"id":"sprints_per_session","value":4,"unit":"count"},
                          {"id":"sessions_per_week","value":5,"unit":"count"},
                          {"id":"meters_per_sprint","value":70,"unit":"distance_per_count"}],
            "ops":[["mul","sprints_per_session","sessions_per_week","sprints_per_week"],
                   ["mul","sprints_per_week","meters_per_sprint","total_meters"]],
            "target":"total_meters"}"#,
        1400,
        "(perturbed James: 4 sprints x5/week, 70m each)",
    );
    // Sheep problem, same ids/structure, new numbers: seattle=15 -> charleston=60, toulouse=120, total=195.
    check(
        &mut host,
        "sheep-v2",
        r#"{"quantities":[{"id":"seattle","value":15,"unit":"count"},
                          {"id":"four","value":4,"unit":"scalar"},
                          {"id":"two","value":2,"unit":"scalar"}],
            "ops":[["mul","seattle","four","charleston"],
                   ["mul","charleston","two","toulouse"],
                   ["add","seattle","charleston","partial"],
                   ["add","partial","toulouse","total"]],
            "target":"total"}"#,
        195,
        "(perturbed sheep: Seattle=15)",
    );
}
