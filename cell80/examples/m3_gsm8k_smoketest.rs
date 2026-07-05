//! **A real M3 smoke test** — 25 genuine problems from GSM8K's public test set (the literal
//! first 27 rows of `openai/grade-school-math`'s `test.jsonl`, fetched fresh, minus 2 that
//! don't fit the current plan IR at all — noted below, not silently dropped), each
//! hand-extracted into the plan IR by *reading the English and doing the extraction the spec
//! asks a model to do* — not designing the plan around a schema I already knew would compile.
//! This is still not the real M3 (no model in the loop, N=25 not 1,319, no distractor plans,
//! no cost measurement), but it's the first time this project's `cell_solve` loop has been
//! checked against a real, unfiltered slice of the benchmark, with known ground-truth answers
//! to verify against.
//!
//! **2 of the first 27 rows don't fit the current plan IR — a real, not a hand-waved, gap:**
//! - Row 9 (John's drive) needs fractional time (a half-hour) — the dialect is u32-only, and
//!   rescaling every time quantity to minutes to dodge it would work here but doesn't
//!   generalize (the next problem might mix minutes and days).
//! - Row 16 (the merchant's jewelry-vs-electronics choice) needs a *comparison* — "pick
//!   whichever profit is bigger" — and [`PlanOp`](cell80::plan::PlanOp) only has
//!   `add`/`sub`/`mul`/`div`. There is no decision primitive in the plan IR at all; a
//!   real M3 either needs one, or needs to route comparison-shaped problems to library
//!   cells (`is_gt`, `max`) directly instead of through a rendered plan.
//!
//! **A third finding, not a gap so much as a convention that needs stating:** GSM8K mixes
//! whole-dollar and decimal-dollar problems freely. Whole-dollar problems (Josh's house,
//! Kylar's glasses) used `unit: "money"` meaning *dollars* here; three problems below
//! (Kyle's book, Marie's pizza, Mishka's clothes) have `$16.50`-style prices and were
//! rescaled to *cents* throughout instead — same unit string, different scale, only
//! internally consistent *within* each plan. That's fine for `render()`'s own dimension
//! checker (it never compares scale across separate plans) but would silently corrupt any
//! cross-plan precipitation comparison that assumed "money" always meant the same scale — a
//! real extraction pipeline needs one firm rule (cents, always), not per-problem judgment.
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
    println!(
        "  {label:<12} {verdict:<28} retrieved={:<5} \"{question}\"",
        o.retrieved
    );
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

    println!("\n=== 17 more (rows 10-27, skipping rows 9 and 16 — see the module doc) ===\n");

    // 10. Eliza's overtime (GSM8K #10). "$10/hr, 1.2x overtime past 40hrs, worked 45." Ans: 460.
    check(
        &mut host,
        "eliza",
        r#"{"quantities":[{"id":"rate","value":10,"unit":"money_per_time"},
                          {"id":"regular_hours","value":40,"unit":"time"},
                          {"id":"worked_hours","value":45,"unit":"time"},
                          {"id":"six","value":6,"unit":"scalar"},
                          {"id":"five","value":5,"unit":"scalar"}],
            "ops":[["sub","worked_hours","regular_hours","overtime_hours"],
                   ["mul","rate","regular_hours","regular_pay"],
                   ["mul","rate","six","scaled"],
                   ["div","scaled","five","overtime_rate"],
                   ["mul","overtime_rate","overtime_hours","overtime_pay"],
                   ["add","regular_pay","overtime_pay","total"]],
            "target":"total"}"#,
        460,
        "$10/hr for 40hrs, 1.2x overtime after that (as 6/5), worked 45hrs. Total pay?",
    );

    // 11. Downloads program (GSM8K #11). "60, then 3x, then -30%." Answer: 366.
    check(
        &mut host,
        "downloads",
        r#"{"quantities":[{"id":"month1","value":60,"unit":"count"},
                          {"id":"three","value":3,"unit":"scalar"},
                          {"id":"pct","value":30,"unit":"scalar"},
                          {"id":"hundred","value":100,"unit":"scalar"}],
            "ops":[["mul","month1","three","month2"],
                   ["mul","month2","pct","scaled"],
                   ["div","scaled","hundred","reduction"],
                   ["sub","month2","reduction","month3"],
                   ["add","month1","month2","partial"],
                   ["add","partial","month3","total"]],
            "target":"total"}"#,
        366,
        "60 downloads month 1, 3x in month 2, -30% in month 3. Total over 3 months?",
    );

    // 12. Toula's bakery (GSM8K #12). Three dozen-priced items. Answer: 694.
    check(
        &mut host,
        "toula",
        r#"{"quantities":[{"id":"donuts_qty","value":3,"unit":"count"},
                          {"id":"donuts_price","value":68,"unit":"money_per_count"},
                          {"id":"cupcakes_qty","value":2,"unit":"count"},
                          {"id":"cupcakes_price","value":80,"unit":"money_per_count"},
                          {"id":"cheesecakes_qty","value":6,"unit":"count"},
                          {"id":"cheesecakes_price","value":55,"unit":"money_per_count"}],
            "ops":[["mul","donuts_qty","donuts_price","donuts_total"],
                   ["mul","cupcakes_qty","cupcakes_price","cupcakes_total"],
                   ["mul","cheesecakes_qty","cheesecakes_price","cheesecakes_total"],
                   ["add","donuts_total","cupcakes_total","partial"],
                   ["add","partial","cheesecakes_total","total"]],
            "target":"total"}"#,
        694,
        "3 dozen donuts @$68/dz, 2 dz cupcakes @$80/dz, 6 dz cheesecakes @$55/dz. Total?",
    );

    // 13. Carlos's lemon tree (GSM8K #13, cents). "$90 tree, 7 lemons/yr @$1.50, -$3/yr upkeep."
    check(
        &mut host,
        "carlos",
        r#"{"quantities":[{"id":"cost","value":9000,"unit":"money"},
                          {"id":"lemons","value":7,"unit":"count"},
                          {"id":"price","value":150,"unit":"money_per_count"},
                          {"id":"water_cost","value":300,"unit":"money"},
                          {"id":"one","value":1,"unit":"scalar"}],
            "ops":[["mul","lemons","price","revenue"],
                   ["sub","revenue","water_cost","net"],
                   ["div","cost","net","years"],
                   ["add","years","one","answer_year"]],
            "target":"answer_year"}"#,
        13,
        "$90 tree, 7 lemons/yr @$1.50 each, -$3/yr upkeep. First year it earns money?",
    );

    // 14. Melanie's vacuums (GSM8K #14) — reverse-chain algebra, not forward-only. Answer: 18.
    check(
        &mut host,
        "melanie",
        r#"{"quantities":[{"id":"remaining","value":5,"unit":"count"},
                          {"id":"two_const","value":2,"unit":"scalar"},
                          {"id":"two_more","value":2,"unit":"count"},
                          {"id":"three","value":3,"unit":"scalar"}],
            "ops":[["mul","remaining","two_const","before_orange"],
                   ["add","before_orange","two_more","before_red"],
                   ["mul","before_red","three","scaled"],
                   ["div","scaled","two_const","started"]],
            "target":"started"}"#,
        18,
        "Sold 1/3 at green, 2 more at red, half of what's left at orange, 5 left. Started with?",
    );

    // 15. Dance class (GSM8K #15) — percent-of-percent-then-reverse-percent. Answer: 60.
    check(
        &mut host,
        "dance",
        r#"{"quantities":[{"id":"total","value":20,"unit":"count"},
                          {"id":"pct1","value":20,"unit":"scalar"},
                          {"id":"pct2","value":25,"unit":"scalar"},
                          {"id":"hundred","value":100,"unit":"scalar"}],
            "ops":[["mul","total","pct1","scaled1"],
                   ["div","scaled1","hundred","contemporary"],
                   ["sub","total","contemporary","remaining1"],
                   ["mul","remaining1","pct2","scaled2"],
                   ["div","scaled2","hundred","jazz"],
                   ["sub","remaining1","jazz","hiphop"],
                   ["mul","hiphop","hundred","scaled3"],
                   ["div","scaled3","total","pct_hiphop"]],
            "target":"pct_hiphop"}"#,
        60,
        "20 students: 20% contemporary, 25% of rest jazz, rest hip-hop. Hip-hop %?",
    );

    // 17. Two trains (GSM8K #17) — replicates the ground truth's own (slightly odd) reasoning.
    check(
        &mut host,
        "trains",
        r#"{"quantities":[{"id":"trains","value":2,"unit":"count"},
                          {"id":"day1_dist","value":80,"unit":"distance"},
                          {"id":"day2_dist","value":150,"unit":"distance"}],
            "ops":[["mul","trains","day1_dist","combined1"],
                   ["mul","trains","day2_dist","combined2"],
                   ["add","combined1","combined2","combined_total"],
                   ["div","combined_total","trains","average"]],
            "target":"average"}"#,
        230,
        "2 trains, 80mi day 1, 150mi day 2 each. Average distance/train over 2 days?",
    );

    // 18. Jill's salary (GSM8K #18). Teach + coach, 50 weeks. Answer: 57500.
    check(
        &mut host,
        "jill",
        r#"{"quantities":[{"id":"teach_rate","value":20,"unit":"money_per_time"},
                          {"id":"teach_hours","value":35,"unit":"time"},
                          {"id":"coach_rate","value":30,"unit":"money_per_time"},
                          {"id":"coach_hours","value":15,"unit":"time"},
                          {"id":"weeks","value":50,"unit":"scalar"}],
            "ops":[["mul","teach_rate","teach_hours","weekly_teach"],
                   ["mul","coach_rate","coach_hours","weekly_coach"],
                   ["add","weekly_teach","weekly_coach","weekly_total"],
                   ["mul","weekly_total","weeks","annual"]],
            "target":"annual"}"#,
        57500,
        "$20/hr x35hrs teaching + $30/hr x15hrs coaching, 50 weeks/year. Annual salary?",
    );

    // 19. Claire's omelets (GSM8K #19). Answer: 7 (dozens).
    check(
        &mut host,
        "claire",
        r#"{"quantities":[{"id":"eggs_per_day","value":3,"unit":"count"},
                          {"id":"days","value":7,"unit":"scalar"},
                          {"id":"weeks","value":4,"unit":"scalar"},
                          {"id":"dozen","value":12,"unit":"scalar"}],
            "ops":[["mul","eggs_per_day","days","eggs_per_week"],
                   ["mul","eggs_per_week","weeks","total_eggs"],
                   ["div","total_eggs","dozen","dozens"]],
            "target":"dozens"}"#,
        7,
        "3-egg omelet daily. Dozens of eggs eaten in 4 weeks?",
    );

    // 20. Marissa's hike (GSM8K #20) — division correctly resolves through named units twice.
    check(
        &mut host,
        "marissa",
        r#"{"quantities":[{"id":"total_dist","value":12,"unit":"distance"},
                          {"id":"target_speed","value":4,"unit":"distance_per_time"},
                          {"id":"dist1","value":4,"unit":"distance"},
                          {"id":"dist2","value":2,"unit":"distance"},
                          {"id":"hour1","value":1,"unit":"time"},
                          {"id":"hour2","value":1,"unit":"time"}],
            "ops":[["div","total_dist","target_speed","total_time"],
                   ["sub","total_time","hour1","remaining_time1"],
                   ["sub","remaining_time1","hour2","remaining_time"],
                   ["sub","total_dist","dist1","remaining_dist1"],
                   ["sub","remaining_dist1","dist2","remaining_dist"],
                   ["div","remaining_dist","remaining_time","needed_speed"]],
            "target":"needed_speed"}"#,
        6,
        "12mi trail @4mph target; already did 1hr@4mi + 1hr@2mi. Needed speed for the rest?",
    );

    // 21. Orange/pineapple drink (GSM8K #21) — two independent fraction-of-quantity chains.
    check(
        &mut host,
        "drink",
        r#"{"quantities":[{"id":"orange","value":10,"unit":"count"},
                          {"id":"pineapple","value":15,"unit":"count"},
                          {"id":"spill","value":1,"unit":"count"},
                          {"id":"three_a","value":3,"unit":"scalar"},
                          {"id":"five","value":5,"unit":"scalar"},
                          {"id":"two_b","value":2,"unit":"scalar"},
                          {"id":"three_b","value":3,"unit":"scalar"}],
            "ops":[["mul","pineapple","three_a","scaled1"],
                   ["div","scaled1","five","pineapple_water"],
                   ["sub","orange","spill","remaining_orange"],
                   ["mul","remaining_orange","two_b","scaled2"],
                   ["div","scaled2","three_b","orange_water"],
                   ["add","pineapple_water","orange_water","total_water"]],
            "target":"total_water"}"#,
        15,
        "10L orange (2/3 water) + 15L pineapple (3/5 water), spill 1L orange. Water in 24L left?",
    );

    // 22. Raymond/Samantha ages (GSM8K #22). Answer: 14.
    check(
        &mut host,
        "raymond",
        r#"{"quantities":[{"id":"gap","value":6,"unit":"time"},
                          {"id":"raymond_age_at_birth","value":23,"unit":"time"},
                          {"id":"samantha_now","value":31,"unit":"time"}],
            "ops":[["sub","raymond_age_at_birth","gap","samantha_then"],
                   ["sub","samantha_now","samantha_then","years_ago"]],
            "target":"years_ago"}"#,
        14,
        "Raymond born 6yrs before Samantha; had a son at 23. Samantha is 31. Son born how long ago?",
    );

    // 23. Billy's DVDs (GSM8K #23). Answer: 7.
    check(
        &mut host,
        "billy",
        r#"{"quantities":[{"id":"c1","value":3,"unit":"count"},
                          {"id":"dvd1","value":1,"unit":"count_per_count"},
                          {"id":"c2","value":2,"unit":"count"},
                          {"id":"dvd2","value":2,"unit":"count_per_count"}],
            "ops":[["mul","c1","dvd1","sold1"],["mul","c2","dvd2","sold2"],
                   ["add","sold1","sold2","total"]],
            "target":"total"}"#,
        7,
        "8 customers: 3 buy 1 DVD each, 2 buy 2 each, 3 buy none. Total DVDs sold?",
    );

    // 24. Candle burn (GSM8K #24). Answer: 8.
    check(
        &mut host,
        "candle",
        r#"{"quantities":[{"id":"start","value":13,"unit":"time"},
                          {"id":"end","value":17,"unit":"time"},
                          {"id":"melt_rate","value":2,"unit":"distance_per_time"}],
            "ops":[["sub","end","start","duration"],
                   ["mul","melt_rate","duration","shorter"]],
            "target":"shorter"}"#,
        8,
        "Candle melts 2cm/hr, burns 1PM to 5PM. How much shorter?",
    );

    // 25. Kyle's book (GSM8K #25, cents) — reverse-percentage. Answer: 2600 (= $26.00).
    check(
        &mut host,
        "kyle",
        r#"{"quantities":[{"id":"discounted","value":1950,"unit":"money"},
                          {"id":"pct_remaining","value":75,"unit":"scalar"},
                          {"id":"hundred","value":100,"unit":"scalar"}],
            "ops":[["mul","discounted","hundred","scaled"],
                   ["div","scaled","pct_remaining","original"]],
            "target":"original"}"#,
        2600,
        "Book bought for $19.50, a 25% discount. Original price (in cents)?",
    );

    // 26. Marie's pizza (GSM8K #26, cents) — solve for an unknown quantity by subtraction.
    check(
        &mut host,
        "marie",
        r#"{"quantities":[{"id":"chicken","value":1200,"unit":"money"},
                          {"id":"milk_qty","value":5,"unit":"count"},
                          {"id":"milk_price","value":300,"unit":"money_per_count"},
                          {"id":"apple_qty","value":4,"unit":"count"},
                          {"id":"apple_price","value":150,"unit":"money_per_count"},
                          {"id":"paid_total","value":5000,"unit":"money"},
                          {"id":"pizza_price","value":850,"unit":"money_per_count"}],
            "ops":[["mul","milk_qty","milk_price","milk_total"],
                   ["mul","apple_qty","apple_price","apple_total"],
                   ["add","chicken","milk_total","partial"],
                   ["add","partial","apple_total","known_total"],
                   ["sub","paid_total","known_total","pizza_total"],
                   ["div","pizza_total","pizza_price","pizza_boxes"]],
            "target":"pizza_boxes"}"#,
        2,
        "$12 chicken, 5x$3 milk, 4x$1.50 apples, paid $50 total. Pizza boxes @$8.50 each?",
    );

    // 27. Mishka's clothes (GSM8K #27, cents). Answer: 24300 (= $243.00).
    check(
        &mut host,
        "mishka",
        r#"{"quantities":[{"id":"shorts","value":1650,"unit":"money"},
                          {"id":"pants","value":2250,"unit":"money"},
                          {"id":"shoes","value":4200,"unit":"money"},
                          {"id":"qty","value":3,"unit":"count"}],
            "ops":[["add","shorts","pants","partial"],
                   ["add","partial","shoes","per_set"],
                   ["mul","per_set","qty","total"]],
            "target":"total"}"#,
        24300,
        "3 pairs each of $16.50 shorts, $22.50 pants, $42 shoes. Total spent (in cents)?",
    );

    println!(
        "\n=== Mini robustness check: same schema, different numbers (GSM-Symbolic-style) ===\n"
    );
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
