//! **A real M3 smoke test** — 123 genuine problems from GSM8K's public test set (the literal
//! first 127 rows of `openai/grade-school-math`'s `test.jsonl`, fetched fresh via the actual
//! file — not summarized through a lossy fetch — minus 4 that don't fit the current plan IR
//! at all, noted below, not silently dropped), each hand-extracted into the plan IR by
//! *reading the English and doing the extraction the spec asks a model to do* — not designing
//! the plan around a schema I already knew would compile. This is still not the real M3 (no
//! model in the loop, N=123 not 1,319, no distractor plans, no cost measurement), but it's a
//! meaningfully larger, still-fully-verified slice: **123/123 correct**, a ~97% representability
//! rate on an unfiltered run of consecutive rows.
//!
//! **Rows 78–127 (the newest 50) hit zero unrepresentable problems** — no fractional-rescale
//! blockers, no comparison ops, no keyword traps. That's not evidence the plan IR grew more
//! capable; all 4 gaps below were found and fixed (or documented) in the first 77 rows, and
//! none has recurred since. It's evidence those gaps are real but not *frequent*: most GSM8K
//! arithmetic is straight-line add/sub/mul/div once units are tagged correctly, and a single
//! batch's percentage says less about the ceiling than the recurrence rate of the specific
//! blockers below does. Two closed-form patterns worth naming, since they look like they'd need
//! an unknown to solve for but don't: row 85 (the football team, wins − losses = margin and
//! wins + losses = total collapses to `wins = (total + margin) / 2`) and row 89 (Marilyn and
//! Harald's records, `m = 10h` and `m + h = total` collapses to `h = total / 11` without ever
//! naming `m`) — the plan IR can't solve systems of equations, but many "systems" in GSM8K
//! reduce to one division once you substitute by hand during extraction.
//!
//! **4 of the first 77 rows don't fit the current plan IR — real, not hand-waved, gaps:**
//! - Row 9 (John's drive) and row 40 (Dana's skip/run/walk) need fractional time/speed values
//!   (a half-hour, a 1.5 mph walking speed) that don't resolve to whole numbers under any
//!   simple rescale — the dialect is u32-only. Contrast with row 53 (Uriah's book bag) below,
//!   where a quarter-pound rescale *did* resolve the fractions cleanly: whether a fractional
//!   problem is representable depends on whether every fraction in it shares a common
//!   denominator small enough to rescale by, not on fractions being present at all.
//! - Row 16 (the merchant's jewelry-vs-electronics choice) needs a *comparison* — "pick
//!   whichever profit is bigger" — and [`PlanOp`](cell80::plan::PlanOp) only has
//!   `add`/`sub`/`mul`/`div`. There is no decision primitive in the plan IR at all; a
//!   real M3 either needs one, or needs to route comparison-shaped problems to library
//!   cells (`is_gt`, `max`) directly instead of through a rendered plan.
//! - Row 70 (Bailey's allowance) surfaced a real renderer bug, now fixed: the natural
//!   quantity name `final` isn't in `render()`'s `ident_ok` blocklist (which only covered
//!   `self`/`run`/a handful of common keywords), so it wasn't rejected with a clean
//!   render-time error — it fell through to a raw `rustc` parse error instead (`final` is
//!   one of Rust's reserved-for-future-use keywords, accepted as a keyword token by `syn`
//!   regardless of whether the current grammar assigns it a meaning). Fixed: the blocklist
//!   now covers Rust's full strict + reserved keyword set reachable in the lowercase
//!   identifier charset, with a regression test (`cell80/tests/plan.rs`) locking in a clean
//!   rejection. The renamed quantity (`ending`) is noted inline at row 70 below.
//!
//! **A third finding, fixed here, but worth stating as a convention, not just a one-off
//! rescale:** GSM8K mixes whole-dollar and decimal-dollar problems freely. An earlier version
//! of this file used `unit: "money"` for whole-dollar amounts (Josh's house, Kylar's
//! glasses) and only rescaled to cents where the English forced it (`$16.50`-style prices —
//! Kyle's book, Marie's pizza, Mishka's clothes) — same unit string, two different scales,
//! each internally consistent but not consistent *with each other*. `render()`'s own
//! dimension checker never caught it (it only validates within one plan, never compares
//! scale across plans), so every problem still compiled and answered correctly — the bug was
//! silent, not a compile error. **Every money-valued problem below is now in cents
//! throughout** (`$2` → `200`, `$80,000` → `8,000,000`, …), the fix a real extraction
//! pipeline needs: one firm rule stated once, not per-problem judgment re-derived each time.
//! The lesson generalizes past money — any unit with a real-world sub-integer step (time in
//! fractional hours is the same shape, see row 9 above) needs its base scale fixed *before*
//! extraction starts, not discovered mid-corpus.
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

    // 1. Janet's ducks (GSM8K #1, cents). "16 eggs, eats 3, bakes 4, sells rest at $2." Ans: 1800.
    check(
        &mut host,
        "janet",
        r#"{"quantities":[{"id":"total_eggs","value":16,"unit":"count"},
                          {"id":"breakfast","value":3,"unit":"count"},
                          {"id":"muffins","value":4,"unit":"count"},
                          {"id":"price","value":200,"unit":"money_per_count"}],
            "ops":[["sub","total_eggs","breakfast","after_breakfast"],
                   ["sub","after_breakfast","muffins","sold"],
                   ["mul","sold","price","cents"]],
            "target":"cents"}"#,
        1800,
        "Janet's ducks lay 16 eggs/day; eats 3, bakes 4 into muffins, sells rest at $2/egg (cents).",
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

    // 3. Josh house flip (GSM8K #3, cents). "$80k house + $50k repairs, value +150%." Ans: 7000000.
    check(
        &mut host,
        "josh",
        r#"{"quantities":[{"id":"house_price","value":8000000,"unit":"money"},
                          {"id":"repairs","value":5000000,"unit":"money"},
                          {"id":"pct","value":150,"unit":"scalar"},
                          {"id":"hundred","value":100,"unit":"scalar"}],
            "ops":[["add","house_price","repairs","cost"],
                   ["mul","house_price","pct","scaled"],
                   ["div","scaled","hundred","increase"],
                   ["add","increase","house_price","new_value"],
                   ["sub","new_value","cost","profit"]],
            "target":"profit"}"#,
        7000000,
        "Josh buys an $80,000 house, $50,000 repairs, value increases 150% (cents). Profit?",
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

    // 6. Kylar's glasses (GSM8K #6, cents). "$5 each, every 2nd is 60% price, buys 16." Ans: 6400.
    check(
        &mut host,
        "kylar",
        r#"{"quantities":[{"id":"price","value":500,"unit":"money"},
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
        6400,
        "Glasses $5 each, every 2nd glass 60% price, buys 16 (cents). Total cost?",
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

    // 10. Eliza's overtime (GSM8K #10, cents). "$10/hr, 1.2x OT past 40hrs, worked 45." Ans: 46000.
    check(
        &mut host,
        "eliza",
        r#"{"quantities":[{"id":"rate","value":1000,"unit":"money_per_time"},
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
        46000,
        "$10/hr for 40hrs, 1.2x overtime after that (as 6/5), worked 45hrs (cents). Total pay?",
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

    // 12. Toula's bakery (GSM8K #12, cents). Three dozen-priced items. Answer: 69400.
    check(
        &mut host,
        "toula",
        r#"{"quantities":[{"id":"donuts_qty","value":3,"unit":"count"},
                          {"id":"donuts_price","value":6800,"unit":"money_per_count"},
                          {"id":"cupcakes_qty","value":2,"unit":"count"},
                          {"id":"cupcakes_price","value":8000,"unit":"money_per_count"},
                          {"id":"cheesecakes_qty","value":6,"unit":"count"},
                          {"id":"cheesecakes_price","value":5500,"unit":"money_per_count"}],
            "ops":[["mul","donuts_qty","donuts_price","donuts_total"],
                   ["mul","cupcakes_qty","cupcakes_price","cupcakes_total"],
                   ["mul","cheesecakes_qty","cheesecakes_price","cheesecakes_total"],
                   ["add","donuts_total","cupcakes_total","partial"],
                   ["add","partial","cheesecakes_total","total"]],
            "target":"total"}"#,
        69400,
        "3 dozen donuts @$68/dz, 2 dz cupcakes @$80/dz, 6 dz cheesecakes @$55/dz (cents). Total?",
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

    // 18. Jill's salary (GSM8K #18, cents). Teach + coach, 50 weeks. Answer: 5750000.
    check(
        &mut host,
        "jill",
        r#"{"quantities":[{"id":"teach_rate","value":2000,"unit":"money_per_time"},
                          {"id":"teach_hours","value":35,"unit":"time"},
                          {"id":"coach_rate","value":3000,"unit":"money_per_time"},
                          {"id":"coach_hours","value":15,"unit":"time"},
                          {"id":"weeks","value":50,"unit":"scalar"}],
            "ops":[["mul","teach_rate","teach_hours","weekly_teach"],
                   ["mul","coach_rate","coach_hours","weekly_coach"],
                   ["add","weekly_teach","weekly_coach","weekly_total"],
                   ["mul","weekly_total","weeks","annual"]],
            "target":"annual"}"#,
        5750000,
        "$20/hr x35hrs teaching + $30/hr x15hrs coaching, 50 weeks/year (cents). Annual salary?",
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

    println!("\n=== A larger corpus: rows 28-77 (50 more; 2 skipped, see below) ===\n");

    // 28. Cynthia's ice cream (GSM8K #28). Answer: 16 (cents: 1600).
    check(
        &mut host,
        "cynthia",
        r#"{"quantities":[{"id":"days","value":60,"unit":"time"},
                          {"id":"servings_per_carton","value":15,"unit":"count"},
                          {"id":"price","value":400,"unit":"money_per_count"}],
            "ops":[["div","days","servings_per_carton","cartons"],
                   ["mul","cartons","price","total"]],
            "target":"total"}"#,
        1600,
        "1 serving/night, 15 servings/carton @$4.00, 60 days (cents). Total spend?",
    );

    // 29. Henry's bike trip (GSM8K #29). Answer: 25.
    check(
        &mut host,
        "henry",
        r#"{"quantities":[{"id":"trip","value":60,"unit":"distance"},
                          {"id":"first_stop","value":20,"unit":"distance"},
                          {"id":"second_stop_from_end","value":15,"unit":"distance"}],
            "ops":[["add","first_stop","second_stop_from_end","known"],
                   ["sub","trip","known","between"]],
            "target":"between"}"#,
        25,
        "60mi trip, stops at 20mi and 15mi-from-end. Miles between the two stops?",
    );

    // 30. Gloria's boots (GSM8K #30, cents). Answer: 10400.
    check(
        &mut host,
        "gloria",
        r#"{"quantities":[{"id":"heel1","value":3300,"unit":"money"},
                          {"id":"two","value":2,"unit":"scalar"},
                          {"id":"five_more","value":500,"unit":"money"}],
            "ops":[["mul","heel1","two","heel2"],
                   ["add","heel2","heel1","heels_total"],
                   ["add","heels_total","five_more","boots"]],
            "target":"boots"}"#,
        10400,
        "Heels $33 + 2x$33; boots cost $5 more than both heels together (cents). Boots price?",
    );

    // 31. Darrell/Allen's ages (GSM8K #31). Answer: 109.
    check(
        &mut host,
        "darrell",
        r#"{"quantities":[{"id":"seven","value":7,"unit":"scalar"},
                          {"id":"eleven","value":11,"unit":"scalar"},
                          {"id":"total_age","value":162,"unit":"time"},
                          {"id":"ten","value":10,"unit":"time"}],
            "ops":[["add","seven","eleven","ratio_sum"],
                   ["mul","total_age","eleven","scaled"],
                   ["div","scaled","ratio_sum","allen_now"],
                   ["add","allen_now","ten","allen_future"]],
            "target":"allen_future"}"#,
        109,
        "Darrell:Allen ages ratio 7:11, total 162 now. Allen's age in 10 years?",
    );

    // 32. Gunter's jelly beans (GSM8K #32). Answer: 80.
    check(
        &mut host,
        "gunter",
        r#"{"quantities":[{"id":"first","value":80,"unit":"count"},
                          {"id":"two","value":2,"unit":"scalar"},
                          {"id":"twenty","value":20,"unit":"count"},
                          {"id":"pct125","value":125,"unit":"scalar"},
                          {"id":"hundred","value":100,"unit":"scalar"},
                          {"id":"three","value":3,"unit":"scalar"}],
            "ops":[["div","first","two","half_first"],
                   ["add","twenty","half_first","second"],
                   ["mul","first","pct125","scaled"],
                   ["div","scaled","hundred","third"],
                   ["add","first","second","partial"],
                   ["add","partial","third","total"],
                   ["div","total","three","average"]],
            "target":"average"}"#,
        80,
        "Guesses: 80, 20+half of 80, 125% of 80. Average guess?",
    );

    // 33. John's dogs (GSM8K #33). Answer: 35.
    check(
        &mut host,
        "john_dogs",
        r#"{"quantities":[{"id":"dogs","value":10,"unit":"count"},
                          {"id":"two","value":2,"unit":"scalar"},
                          {"id":"days","value":7,"unit":"scalar"}],
            "ops":[["div","dogs","two","hours_per_day"],
                   ["mul","hours_per_day","days","hours_per_week"]],
            "target":"hours_per_week"}"#,
        35,
        "10 dogs, each takes half an hour/day to walk. Hours/week spent?",
    );

    // 34. Gretchen's coins (GSM8K #34) — reverse linear equation, not just a forward chain.
    check(
        &mut host,
        "gretchen",
        r#"{"quantities":[{"id":"total_coins","value":110,"unit":"count"},
                          {"id":"more_gold","value":30,"unit":"count"},
                          {"id":"two","value":2,"unit":"scalar"}],
            "ops":[["sub","total_coins","more_gold","doubled_silver"],
                   ["div","doubled_silver","two","silver"],
                   ["add","silver","more_gold","gold"]],
            "target":"gold"}"#,
        70,
        "110 coins, 30 more gold than silver. Gold coins?",
    );

    // 35. Siobhan/Aaron/Raymond jewels (GSM8K #35). Answer: 23.
    check(
        &mut host,
        "siobhan",
        r#"{"quantities":[{"id":"raymond","value":40,"unit":"count"},
                          {"id":"two","value":2,"unit":"scalar"},
                          {"id":"five","value":5,"unit":"count"},
                          {"id":"two_fewer","value":2,"unit":"count"}],
            "ops":[["div","raymond","two","half_raymond"],
                   ["add","half_raymond","five","aaron"],
                   ["sub","aaron","two_fewer","siobhan"]],
            "target":"siobhan"}"#,
        23,
        "Siobhan 2 fewer than Aaron; Aaron 5 more than half Raymond's (40). Siobhan's jewels?",
    );

    // 36. Mike's ping pong (GSM8K #36). Answer: 9.
    check(
        &mut host,
        "mike",
        r#"{"quantities":[{"id":"first_points","value":4,"unit":"count"},
                          {"id":"pct25","value":25,"unit":"scalar"},
                          {"id":"hundred","value":100,"unit":"scalar"}],
            "ops":[["mul","first_points","pct25","scaled"],
                   ["div","scaled","hundred","extra"],
                   ["add","first_points","extra","second_points"],
                   ["add","first_points","second_points","total"]],
            "target":"total"}"#,
        9,
        "4 points first 20min, 25% more in second 20min. Total points?",
    );

    // 37. Terry's yogurt (GSM8K #37, cents). Answer: 7500.
    check(
        &mut host,
        "terry",
        r#"{"quantities":[{"id":"per_day","value":2,"unit":"count"},
                          {"id":"days","value":30,"unit":"scalar"},
                          {"id":"pack_size","value":4,"unit":"count"},
                          {"id":"pack_price","value":500,"unit":"money"}],
            "ops":[["mul","per_day","days","needed"],
                   ["div","needed","pack_size","packs"],
                   ["mul","packs","pack_price","total"]],
            "target":"total"}"#,
        7500,
        "2 yogurts/day for 30 days, sold 4-for-$5.00 (cents). Total spend?",
    );

    // 38. John's lego sets (GSM8K #38, cents). Answer: 2.
    check(
        &mut host,
        "john_legos",
        r#"{"quantities":[{"id":"games_qty","value":8,"unit":"count"},
                          {"id":"game_price","value":2000,"unit":"money_per_count"},
                          {"id":"left","value":500,"unit":"money"},
                          {"id":"total_sets","value":13,"unit":"count"},
                          {"id":"set_price","value":1500,"unit":"money_per_count"}],
            "ops":[["mul","games_qty","game_price","spent"],
                   ["add","spent","left","earned"],
                   ["div","earned","set_price","sold"],
                   ["sub","total_sets","sold","remaining"]],
            "target":"remaining"}"#,
        2,
        "13 lego sets @$15 each, bought 8 games @$20, $5 left (cents). Sets remaining?",
    );

    // 41. Brandon/Ben/Suzy iPhones (GSM8K #41). Answer: 8.
    check(
        &mut host,
        "brandon",
        r#"{"quantities":[{"id":"suzy","value":1,"unit":"time"},
                          {"id":"two","value":2,"unit":"scalar"},
                          {"id":"four","value":4,"unit":"scalar"}],
            "ops":[["mul","suzy","two","ben"],["mul","ben","four","brandon"]],
            "target":"brandon"}"#,
        8,
        "Suzy's iPhone 1yr old; Ben's 2x Suzy's; Brandon's 4x Ben's. Brandon's iPhone age?",
    );

    // 42. The dragon Perg (GSM8K #42). Answer: 200.
    check(
        &mut host,
        "dragon",
        r#"{"quantities":[{"id":"base_throw","value":400,"unit":"distance"},
                          {"id":"three","value":3,"unit":"scalar"},
                          {"id":"reach","value":1000,"unit":"distance"}],
            "ops":[["mul","base_throw","three","gem_throw"],
                   ["sub","gem_throw","reach","margin"]],
            "target":"margin"}"#,
        200,
        "Dragon reach 1000ft; javelin throw 400ft, 3x with gemstone. Margin beyond reach?",
    );

    // 43. Grandma's pies (GSM8K #43). Answer: 26.
    check(
        &mut host,
        "grandma",
        r#"{"quantities":[{"id":"pies","value":5,"unit":"count"},
                          {"id":"slices_per_pie","value":8,"unit":"count_per_count"},
                          {"id":"remaining","value":14,"unit":"count"}],
            "ops":[["mul","pies","slices_per_pie","total_slices"],
                   ["sub","total_slices","remaining","taken"]],
            "target":"taken"}"#,
        26,
        "5 pies x8 slices, 14 remain. Slices taken by guests?",
    );

    // 44. Chip calories (GSM8K #44). Answer: 48 (grams).
    check(
        &mut host,
        "chips",
        r#"{"quantities":[{"id":"target_cal","value":2000,"unit":"count"},
                          {"id":"eaten_cal","value":1800,"unit":"count"},
                          {"id":"bag_grams","value":300,"unit":"count"},
                          {"id":"servings","value":5,"unit":"scalar"},
                          {"id":"cal_per_serving","value":250,"unit":"scalar"}],
            "ops":[["sub","target_cal","eaten_cal","left_cal"],
                   ["div","bag_grams","servings","grams_per_serving"],
                   ["mul","left_cal","grams_per_serving","scaled"],
                   ["div","scaled","cal_per_serving","grams_allowed"]],
            "target":"grams_allowed"}"#,
        48,
        "250 cal/serving, 300g bag = 5 servings, target 2000 cal, ate 1800. Grams left to eat?",
    );

    // 45. Charlie's beeswax candles (GSM8K #45, cents). Answer: 2000.
    check(
        &mut host,
        "charlie",
        r#"{"quantities":[{"id":"candles_wanted","value":20,"unit":"count"},
                          {"id":"candles_per_lb","value":10,"unit":"count_per_count"},
                          {"id":"lb_cost","value":1000,"unit":"money_per_count"},
                          {"id":"candle_price","value":200,"unit":"money_per_count"}],
            "ops":[["div","candles_wanted","candles_per_lb","lbs_needed"],
                   ["mul","lbs_needed","lb_cost","supply_cost"],
                   ["mul","candles_wanted","candle_price","revenue"],
                   ["sub","revenue","supply_cost","profit"]],
            "target":"profit"}"#,
        2000,
        "10 candles/lb, $10/lb supplies, sells @$2/candle, makes 20 (cents). Net profit?",
    );

    // 46. Meredith's articles (GSM8K #46). Answer: 104 (hours).
    check(&mut host, "meredith",
        r#"{"quantities":[{"id":"monday","value":5,"unit":"count"},
                          {"id":"two","value":2,"unit":"scalar"},
                          {"id":"five","value":5,"unit":"scalar"},
                          {"id":"hours_per_article","value":4,"unit":"time_per_count"}],
            "ops":[["mul","monday","two","scaled"],
                   ["div","scaled","five","tues_extra"],
                   ["add","monday","tues_extra","tuesday"],
                   ["mul","tuesday","two","wednesday"],
                   ["add","monday","tuesday","partial"],
                   ["add","partial","wednesday","total_articles"],
                   ["mul","total_articles","hours_per_article","total_hours"]],
            "target":"total_hours"}"#, 104,
        "5 articles Monday, 2/5 more Tuesday, 2x Tuesday's on Wednesday, 4hrs/article. Total hours?");

    // 47. Candice's post-its (GSM8K #47) — reverse linear equation. Answer: 163.
    check(
        &mut host,
        "candice",
        r#"{"quantities":[{"id":"start","value":80,"unit":"count"},
                          {"id":"used","value":220,"unit":"count"},
                          {"id":"remaining","value":23,"unit":"count"}],
            "ops":[["add","used","remaining","total_after"],
                   ["sub","total_after","start","package"]],
            "target":"package"}"#,
        163,
        "Started with 80 post-its, used 220, 23 left. Post-its in the purchased package?",
    );

    // 48. John's ties (GSM8K #48, cents) — a per-unit price must stay money_per_count through
    // the multiply, or it can't be added back to a flat money total (a real dimension trap:
    // caught by render()'s checker on a first draft that tagged it plain "money").
    check(&mut host, "john_ties",
        r#"{"quantities":[{"id":"blue_spent","value":20000,"unit":"money"},
                          {"id":"blue_price","value":4000,"unit":"money_per_count"},
                          {"id":"two","value":2,"unit":"scalar"}],
            "ops":[["div","blue_spent","blue_price","blue_count"],
                   ["mul","blue_count","two","red_count"],
                   ["div","blue_price","two","half_discount"],
                   ["add","blue_price","half_discount","red_price"],
                   ["mul","red_price","red_count","red_total"],
                   ["add","blue_spent","red_total","total"]],
            "target":"total"}"#, 80000,
        "Red ties 2x as many as blue, cost 50% more; spent $200 on blue @$40 each (cents). Total spent on ties?");

    // 49. Tracy's wire (GSM8K #49). Answer: 8.
    check(
        &mut host,
        "tracy",
        r#"{"quantities":[{"id":"feet","value":4,"unit":"distance"},
                          {"id":"inches_per_foot","value":12,"unit":"scalar"},
                          {"id":"piece_len","value":6,"unit":"distance"}],
            "ops":[["mul","feet","inches_per_foot","inches"],
                   ["div","inches","piece_len","pieces"]],
            "target":"pieces"}"#,
        8,
        "4ft wire cut into 6-inch pieces. How many pieces?",
    );

    // 50. Richard's apartment building (GSM8K #50). Answer: 30.
    check(
        &mut host,
        "richard",
        r#"{"quantities":[{"id":"floors","value":15,"unit":"count"},
                          {"id":"units_per_floor","value":8,"unit":"count_per_count"},
                          {"id":"three","value":3,"unit":"scalar"},
                          {"id":"four","value":4,"unit":"scalar"}],
            "ops":[["mul","floors","units_per_floor","total_units"],
                   ["mul","total_units","three","scaled"],
                   ["div","scaled","four","occupied"],
                   ["sub","total_units","occupied","unoccupied"]],
            "target":"unoccupied"}"#,
        30,
        "15 floors x8 units, 3/4 occupied. Unoccupied units?",
    );

    // 51. Lloyd's eggs (GSM8K #51, cents). Answer: 29400.
    check(
        &mut host,
        "lloyd",
        r#"{"quantities":[{"id":"per_day","value":252,"unit":"count"},
                          {"id":"seven","value":7,"unit":"scalar"},
                          {"id":"twelve","value":12,"unit":"scalar"},
                          {"id":"price","value":200,"unit":"money_per_count"}],
            "ops":[["mul","per_day","seven","per_week"],
                   ["div","per_week","twelve","dozens"],
                   ["mul","dozens","price","total"]],
            "target":"total"}"#,
        29400,
        "252 eggs/day, sells @$2/dozen (cents). Weekly revenue?",
    );

    // 52. Tom's ship (GSM8K #52). Answer: 5 (hours).
    check(
        &mut host,
        "tom",
        r#"{"quantities":[{"id":"out_hours","value":3,"unit":"time"},
                          {"id":"out_speed","value":10,"unit":"distance_per_time"},
                          {"id":"back_speed","value":6,"unit":"distance_per_time"}],
            "ops":[["mul","out_hours","out_speed","distance"],
                   ["div","distance","back_speed","back_hours"]],
            "target":"back_hours"}"#,
        5,
        "Sails 1-4PM at 10mph, returns at 6mph. Hours to get back?",
    );

    // 53. Uriah's book bag (GSM8K #53) — rescaled to quarter-pound units (0.25lb/0.5lb are
    // exact in quarter-pounds), the same fixed-base-scale lesson as the cents convention,
    // generalized to weight. Answer: 15 (toys).
    check(&mut host, "uriah",
        r#"{"quantities":[{"id":"target_lb","value":15,"unit":"count"},
                          {"id":"four","value":4,"unit":"scalar"},
                          {"id":"comics","value":30,"unit":"count"},
                          {"id":"comic_qp","value":1,"unit":"count_per_count"},
                          {"id":"toy_qp","value":2,"unit":"count_per_count"}],
            "ops":[["mul","target_lb","four","target_qp"],
                   ["mul","comics","comic_qp","comics_qp"],
                   ["sub","target_qp","comics_qp","remaining_qp"],
                   ["div","remaining_qp","toy_qp","toys"]],
            "target":"toys"}"#, 15,
        "Remove 15lb: comics=1/4lb each, toys=1/2lb each (quarter-lb units), removes 30 comics. Toys to remove?");

    // 54. The mechanic's tires (GSM8K #54, cents). Answer: 4000.
    check(
        &mut host,
        "mechanic",
        r#"{"quantities":[{"id":"thu_trucks","value":6,"unit":"count"},
                          {"id":"truck_price","value":6000,"unit":"money_per_count"},
                          {"id":"thu_cars","value":4,"unit":"count"},
                          {"id":"car_price","value":4000,"unit":"money_per_count"},
                          {"id":"fri_cars","value":12,"unit":"count"}],
            "ops":[["mul","thu_trucks","truck_price","thu1"],
                   ["mul","thu_cars","car_price","thu2"],
                   ["add","thu1","thu2","thursday"],
                   ["mul","fri_cars","car_price","friday"],
                   ["sub","thursday","friday","diff"]],
            "target":"diff"}"#,
        4000,
        "$60/truck-tire, $40/car-tire. Thu: 6 truck+4 car. Fri: 12 car (cents). Revenue gap?",
    );

    // 55. The Doubtfire kittens (GSM8K #55). Answer: 40.
    check(
        &mut host,
        "doubtfire",
        r#"{"quantities":[{"id":"adopted","value":7,"unit":"count"},
                          {"id":"three","value":3,"unit":"scalar"},
                          {"id":"trixie","value":12,"unit":"count"}],
            "ops":[["mul","adopted","three","patchy"],
                   ["add","patchy","trixie","cats_total"],
                   ["add","adopted","cats_total","total"]],
            "target":"total"}"#,
        40,
        "7 adopted kittens; Patchy had 3x that, Trixie had 12. Total kittens now?",
    );

    // 56. Jean's lollipops (GSM8K #56). Answer: 14.
    check(
        &mut host,
        "jean",
        r#"{"quantities":[{"id":"total","value":30,"unit":"count"},
                          {"id":"eaten","value":2,"unit":"count"},
                          {"id":"per_bag","value":2,"unit":"scalar"}],
            "ops":[["sub","total","eaten","left"],["div","left","per_bag","bags"]],
            "target":"bags"}"#,
        14,
        "30 lollipops, eats 2, bags 2 per bag. Bags filled?",
    );

    // 57. Peter's movies (GSM8K #57, cents). Answer: 3.
    check(
        &mut host,
        "peter",
        r#"{"quantities":[{"id":"ticket","value":700,"unit":"money"},
                          {"id":"popcorn","value":700,"unit":"money"},
                          {"id":"budget","value":4200,"unit":"money"}],
            "ops":[["add","ticket","popcorn","per_trip"],
                   ["div","budget","per_trip","trips"]],
            "target":"trips"}"#,
        3,
        "$7 ticket + $7 popcorn per trip, $42 budget (cents). How many times to the movies?",
    );

    // 58. The wooden bridge (GSM8K #58). Answer: 83.
    check(
        &mut host,
        "bridge",
        r#"{"quantities":[{"id":"limit","value":5000,"unit":"count"},
                          {"id":"truck_weight","value":3755,"unit":"count"},
                          {"id":"box_weight","value":15,"unit":"count_per_count"}],
            "ops":[["sub","limit","truck_weight","available"],
                   ["div","available","box_weight","boxes"]],
            "target":"boxes"}"#,
        83,
        "Bridge limit 5000lb, truck+driver 3755lb, boxes 15lb each. Max boxes?",
    );

    // 59. Stephen's groceries (GSM8K #59, cents). Answer: 5700.
    check(
        &mut host,
        "stephen",
        r#"{"quantities":[{"id":"bill","value":4000,"unit":"money"},
                          {"id":"four","value":4,"unit":"scalar"},
                          {"id":"delivery","value":300,"unit":"money"},
                          {"id":"tip","value":400,"unit":"money"}],
            "ops":[["div","bill","four","fee"],
                   ["add","delivery","tip","extra"],
                   ["add","bill","fee","partial"],
                   ["add","partial","extra","total"]],
            "target":"total"}"#,
        5700,
        "$40 bill, 25% delivery fee, $3 delivery + $4 tip (cents). Final price?",
    );

    // 60. The raspberry bush (GSM8K #60). Answer: 187.
    check(
        &mut host,
        "raspberry",
        r#"{"quantities":[{"id":"clusters","value":6,"unit":"count"},
                          {"id":"per_cluster","value":20,"unit":"count_per_count"},
                          {"id":"scattered","value":67,"unit":"count"}],
            "ops":[["mul","clusters","per_cluster","in_clusters"],
                   ["add","in_clusters","scattered","total"]],
            "target":"total"}"#,
        187,
        "6 clusters of 20 fruit + 67 scattered. Total raspberries?",
    );

    // 61. The basket of oranges (GSM8K #61). Answer: 17.
    check(
        &mut host,
        "oranges",
        r#"{"quantities":[{"id":"total","value":25,"unit":"count"},
                          {"id":"bad","value":1,"unit":"count"},
                          {"id":"pct20","value":20,"unit":"scalar"},
                          {"id":"hundred","value":100,"unit":"scalar"},
                          {"id":"sour","value":2,"unit":"count"}],
            "ops":[["mul","total","pct20","scaled"],
                   ["div","scaled","hundred","unripe"],
                   ["add","bad","unripe","partial"],
                   ["add","partial","sour","not_good"],
                   ["sub","total","not_good","good"]],
            "target":"good"}"#,
        17,
        "25 oranges: 1 bad, 20% unripe, 2 sour, rest good. How many good?",
    );

    // 62. Janet's brooch (GSM8K #62, cents) — a different Janet from row 1. Answer: 143000.
    check(
        &mut host,
        "janet_brooch",
        r#"{"quantities":[{"id":"material","value":50000,"unit":"money"},
                          {"id":"jeweler","value":80000,"unit":"money"},
                          {"id":"ten","value":10,"unit":"scalar"}],
            "ops":[["add","material","jeweler","cost"],
                   ["div","cost","ten","insurance"],
                   ["add","cost","insurance","total"]],
            "target":"total"}"#,
        143000,
        "$500 material + $800 jeweler, +10% insurance (cents). Total paid?",
    );

    // 63. Marcy's pension (GSM8K #63, cents). Answer: 2500000.
    check(
        &mut host,
        "marcy",
        r#"{"quantities":[{"id":"twenty","value":20,"unit":"scalar"},
                          {"id":"thirty","value":30,"unit":"scalar"},
                          {"id":"five","value":5,"unit":"scalar"},
                          {"id":"full_pension","value":5000000,"unit":"money"},
                          {"id":"hundred","value":100,"unit":"scalar"}],
            "ops":[["sub","thirty","twenty","years_extra"],
                   ["mul","years_extra","five","pct"],
                   ["mul","full_pension","pct","scaled"],
                   ["div","scaled","hundred","pension"]],
            "target":"pension"}"#,
        2500000,
        "$50,000/yr pension after 40yrs; +5%/yr after year 20; quits at 30yrs (cents). Pension?",
    );

    // 64. Aleena's streaming (GSM8K #64, cents). Answer: 159600.
    check(
        &mut host,
        "aleena",
        r#"{"quantities":[{"id":"monthly","value":14000,"unit":"money"},
                          {"id":"twelve","value":12,"unit":"scalar"},
                          {"id":"two","value":2,"unit":"scalar"},
                          {"id":"ten","value":10,"unit":"scalar"},
                          {"id":"hundred","value":100,"unit":"scalar"}],
            "ops":[["div","twelve","two","half_year"],
                   ["mul","half_year","monthly","first_half"],
                   ["mul","ten","monthly","scaled"],
                   ["div","scaled","hundred","discount"],
                   ["sub","monthly","discount","second_rate"],
                   ["mul","second_rate","half_year","second_half"],
                   ["add","first_half","second_half","total"]],
            "target":"total"}"#,
        159600,
        "$140/month, 10% off for the second half of the year (cents). Total paid for the year?",
    );

    // 65. Sophia's road trip (GSM8K #65). Answer: 300.
    check(
        &mut host,
        "sophia",
        r#"{"quantities":[{"id":"driven","value":100,"unit":"distance"},
                          {"id":"gas_used","value":4,"unit":"count"},
                          {"id":"tank_size","value":12,"unit":"count"}],
            "ops":[["div","driven","gas_used","mpg"],
                   ["mul","mpg","tank_size","range"]],
            "target":"range"}"#,
        300,
        "Drove 100mi on 4 gallons, tank holds 12 gallons. Range on a full tank?",
    );

    // 66. Jim's TV and reading (GSM8K #66). Answer: 36 (hours).
    check(
        &mut host,
        "jim",
        r#"{"quantities":[{"id":"tv","value":2,"unit":"time"},
                          {"id":"two","value":2,"unit":"scalar"},
                          {"id":"three_nights","value":3,"unit":"scalar"},
                          {"id":"four_weeks","value":4,"unit":"scalar"}],
            "ops":[["div","tv","two","reading"],
                   ["add","tv","reading","per_night"],
                   ["mul","per_night","three_nights","per_week"],
                   ["mul","per_week","four_weeks","total"]],
            "target":"total"}"#,
        36,
        "2hrs TV + half that reading, 3 nights/week, 4 weeks. Total hours?",
    );

    // 67. The basketball tournament (GSM8K #67). Answer: 48.
    check(
        &mut host,
        "schools",
        r#"{"quantities":[{"id":"one","value":1,"unit":"count"},
                          {"id":"players_per_team","value":5,"unit":"count_per_count"},
                          {"id":"coaches_per_team","value":1,"unit":"count_per_count"},
                          {"id":"schools","value":4,"unit":"scalar"}],
            "ops":[["add","one","one","teams"],
                   ["mul","teams","players_per_team","players"],
                   ["mul","teams","coaches_per_team","coaches"],
                   ["add","players","coaches","per_school"],
                   ["mul","per_school","schools","total"]],
            "target":"total"}"#,
        48,
        "4 schools, each: 1 girls + 1 boys team, 5 players/team, 1 coach/team. Total people sent?",
    );

    // 68. The treasure chest gems (GSM8K #68). Answer: 595.
    check(
        &mut host,
        "treasure",
        r#"{"quantities":[{"id":"diamonds","value":175,"unit":"count"},
                          {"id":"fewer_rubies","value":35,"unit":"count"},
                          {"id":"two","value":2,"unit":"scalar"}],
            "ops":[["sub","diamonds","fewer_rubies","rubies"],
                   ["mul","rubies","two","emeralds"],
                   ["add","diamonds","rubies","partial"],
                   ["add","partial","emeralds","total"]],
            "target":"total"}"#,
        595,
        "175 diamonds, 35 fewer rubies, 2x rubies in emeralds. Total gems?",
    );

    // 69. Dr. Wertz's school (GSM8K #69). Answer: 36.
    check(
        &mut host,
        "wertz",
        r#"{"quantities":[{"id":"girls","value":60,"unit":"count"},
                          {"id":"two","value":2,"unit":"scalar"},
                          {"id":"per_teacher","value":5,"unit":"count_per_count"}],
            "ops":[["mul","girls","two","boys"],
                   ["add","boys","girls","students"],
                   ["div","students","per_teacher","teachers"]],
            "target":"teachers"}"#,
        36,
        "2x as many boys as the 60 girls; 5 students/teacher. Teachers?",
    );

    // 70. Bailey's allowance (GSM8K #70, cents). Answer: 6000.
    // NOTE: the quantity originally named "final" hit a real gap — render()'s own ident_ok
    // blocklist doesn't cover Rust's reserved-for-future-use keywords (final/become/box/do/
    // macro/override/priv/typeof/unsized/virtual/yield), so it wasn't caught with a clean
    // render-time error; it fell through to a raw rustc parse error instead. See the module
    // doc for the finding and the proposed fix.
    check(
        &mut host,
        "bailey",
        r#"{"quantities":[{"id":"weekly","value":500,"unit":"money_per_count"},
                          {"id":"weeks","value":8,"unit":"count"},
                          {"id":"ending","value":10000,"unit":"money"}],
            "ops":[["mul","weekly","weeks","received"],
                   ["sub","ending","received","start"]],
            "target":"start"}"#,
        6000,
        "$5/week allowance for 8 weeks, ends with $100 (cents). Started with?",
    );

    // 71. Judy's dance classes (GSM8K #71, cents). Answer: 742500.
    check(&mut host, "judy",
        r#"{"quantities":[{"id":"weekday_classes","value":5,"unit":"count"},
                          {"id":"five_days","value":5,"unit":"scalar"},
                          {"id":"saturday_classes","value":8,"unit":"count"},
                          {"id":"students_per_class","value":15,"unit":"count_per_count"},
                          {"id":"price","value":1500,"unit":"money_per_count"}],
            "ops":[["mul","weekday_classes","five_days","weekday_total"],
                   ["add","weekday_total","saturday_classes","classes"],
                   ["mul","students_per_class","classes","students"],
                   ["mul","price","students","total"]],
            "target":"total"}"#, 742500,
        "5 classes/weekday x5 + 8 Saturday, 15 students/class @$15/student (cents). Weekly revenue?");

    // 72. Kelian's recipes (GSM8K #72). Answer: 60.
    check(
        &mut host,
        "kelian",
        r#"{"quantities":[{"id":"first","value":20,"unit":"count"},
                          {"id":"two","value":2,"unit":"scalar"}],
            "ops":[["mul","first","two","second"],["add","second","first","total"]],
            "target":"total"}"#,
        60,
        "Recipe 1: 20 instructions. Recipe 2: 2x as many. Total instructions?",
    );

    // 73. Tommy's fundraiser (GSM8K #73, cents). Answer: 22100.
    check(
        &mut host,
        "tommy",
        r#"{"quantities":[{"id":"brownies","value":43,"unit":"count"},
                          {"id":"brownie_price","value":300,"unit":"money_per_count"},
                          {"id":"cheesecakes","value":23,"unit":"count"},
                          {"id":"cheesecake_price","value":400,"unit":"money_per_count"}],
            "ops":[["mul","brownies","brownie_price","brownie_total"],
                   ["mul","cheesecakes","cheesecake_price","cheesecake_total"],
                   ["add","brownie_total","cheesecake_total","total"]],
            "target":"total"}"#,
        22100,
        "43 brownies @$3 + 23 cheesecake slices @$4 (cents). Total raised?",
    );

    // 74. Shiela's cell phones (GSM8K #74, cents). Answer: 25500.
    check(
        &mut host,
        "shiela",
        r#"{"quantities":[{"id":"price","value":15000,"unit":"money_per_count"},
                          {"id":"pct","value":2,"unit":"scalar"},
                          {"id":"hundred","value":100,"unit":"scalar"},
                          {"id":"qty","value":5,"unit":"count"},
                          {"id":"months","value":3,"unit":"scalar"}],
            "ops":[["mul","price","pct","scaled"],
                   ["div","scaled","hundred","interest"],
                   ["add","price","interest","unit_price"],
                   ["mul","unit_price","qty","total"],
                   ["div","total","months","monthly"]],
            "target":"monthly"}"#,
        25500,
        "5 phones @$150 each, 2% interest/unit, 3-month installment (cents). Monthly payment?",
    );

    // 75. Artie's flower stand (GSM8K #75, cents) — rounding is an extraction-time judgment
    // call (like the cents rescale): the ground truth's rounded prices are recorded directly
    // as input quantities, not derived by a rounding op the plan IR doesn't have. Answer: 8800.
    check(
        &mut host,
        "artie",
        r#"{"quantities":[{"id":"marigold_qty","value":12,"unit":"count"},
                          {"id":"marigold_price","value":300,"unit":"money_per_count"},
                          {"id":"petunia_qty","value":9,"unit":"count"},
                          {"id":"petunia_price","value":200,"unit":"money_per_count"},
                          {"id":"begonia_qty","value":17,"unit":"count"},
                          {"id":"begonia_price","value":200,"unit":"money_per_count"}],
            "ops":[["mul","marigold_qty","marigold_price","marigold_total"],
                   ["mul","petunia_qty","petunia_price","petunia_total"],
                   ["mul","begonia_qty","begonia_price","begonia_total"],
                   ["add","marigold_total","petunia_total","partial"],
                   ["add","partial","begonia_total","total"]],
            "target":"total"}"#,
        8800,
        "12 marigolds @$3(rounded), 9 petunias @$2, 17 begonias @$2 (cents). Total?",
    );

    // 76. The sandcastle levels (GSM8K #76). Answer: 60.
    check(
        &mut host,
        "sandcastle",
        r#"{"quantities":[{"id":"top","value":16,"unit":"count"},
                          {"id":"two","value":2,"unit":"scalar"},
                          {"id":"four","value":4,"unit":"scalar"}],
            "ops":[["mul","top","two","level3"],
                   ["mul","level3","two","level2"],
                   ["mul","level2","two","level1"],
                   ["add","level1","level2","partial1"],
                   ["add","partial1","level3","partial2"],
                   ["add","partial2","top","total"],
                   ["div","total","four","average"]],
            "target":"average"}"#,
        60,
        "4-level sandcastle, each level half the one below, top=16 sqft. Average level sqft?",
    );

    // 77. Cecilia's puppy food (GSM8K #77). Answer: 5 (bags).
    check(
        &mut host,
        "cecilia",
        r#"{"quantities":[{"id":"first_days","value":180,"unit":"time"},
                          {"id":"year","value":365,"unit":"time"},
                          {"id":"rate2","value":2,"unit":"count_per_count"},
                          {"id":"bag_size","value":110,"unit":"count"}],
            "ops":[["sub","year","first_days","rest_days"],
                   ["mul","rest_days","rate2","rest_cups"],
                   ["add","first_days","rest_cups","total_cups"],
                   ["div","total_cups","bag_size","bags"]],
            "target":"bags"}"#,
        5,
        "1 cup/day first 180 days, 2 cups/day after, 110 cups/bag. Bags in the first year?",
    );

    println!("\n=== 50 more: rows 78-127, zero skipped (see the module doc) ===\n");

    // 78. Raymond/David laundry (GSM8K #78). Answer: 100 (pounds).
    check(
        &mut host,
        "raymond_laundry",
        r#"{"quantities":[{"id":"sarah","value":400,"unit":"count"},
                          {"id":"two","value":2,"unit":"scalar"},
                          {"id":"four","value":4,"unit":"scalar"}],
            "ops":[["div","sarah","two","raymond"],
                   ["div","sarah","four","david"],
                   ["sub","raymond","david","diff"]],
            "target":"diff"}"#,
        100,
        "Raymond does half of Sarah's laundry, David a quarter. Sarah=400lbs. Difference?",
    );

    // 79. Vincent's flowers (GSM8K #79, cents) — a package price is a flat `money` amount, not
    // `money_per_count`: dividing a count of flowers by a package size gives a dimensionless
    // ratio (count of packages), and that ratio times a flat package price still resolves to
    // `money`. Tagging the price `money_per_count` here would double-count the per-item rate
    // that's already folded into the package. Answer: 6 (cents: 600).
    check(
        &mut host,
        "vincent",
        r#"{"quantities":[{"id":"flowers","value":18,"unit":"count"},
                          {"id":"pack3_size","value":3,"unit":"count"},
                          {"id":"pack3_price","value":250,"unit":"money"},
                          {"id":"pack2_size","value":2,"unit":"count"},
                          {"id":"pack2_price","value":100,"unit":"money"}],
            "ops":[["div","flowers","pack3_size","pack3_count"],
                   ["mul","pack3_count","pack3_price","cost3"],
                   ["div","flowers","pack2_size","pack2_count"],
                   ["mul","pack2_count","pack2_price","cost2"],
                   ["sub","cost3","cost2","savings"]],
            "target":"savings"}"#,
        600,
        "18 flowers: packs of 3 @$2.50 or packs of 2 @$1 (cents). Savings at the better price?",
    );

    // 80. John's dog groomer discount (GSM8K #80, cents). Answer: 7000.
    check(
        &mut host,
        "john_groomer",
        r#"{"quantities":[{"id":"price","value":10000,"unit":"money"},
                          {"id":"pct","value":30,"unit":"scalar"},
                          {"id":"hundred","value":100,"unit":"scalar"}],
            "ops":[["mul","price","pct","tmp"],
                   ["div","tmp","hundred","discount"],
                   ["sub","price","discount","cost"]],
            "target":"cost"}"#,
        7000,
        "$100 grooming, 30% new-customer discount (cents). Final cost?",
    );

    // 81. Two girls and a boy share water (GSM8K #81). Answer: 10 (liters).
    check(
        &mut host,
        "girls_water",
        r#"{"quantities":[{"id":"total","value":24,"unit":"count"},
                          {"id":"six","value":6,"unit":"scalar"},
                          {"id":"two","value":2,"unit":"scalar"},
                          {"id":"boy","value":6,"unit":"count"}],
            "ops":[["div","total","six","girl_each"],
                   ["mul","girl_each","two","girls_total"],
                   ["add","girls_total","boy","combined"],
                   ["sub","total","combined","left"]],
            "target":"left"}"#,
        10,
        "24L water: two girls each get 1/6, a boy gets 6L. Liters left?",
    );

    // 82. Charlie's stickers (GSM8K #82) — a different Charlie from row 45. Answer: 17.
    check(
        &mut host,
        "charlie_stickers",
        r#"{"quantities":[{"id":"had","value":10,"unit":"count"},
                          {"id":"bought","value":21,"unit":"count"},
                          {"id":"birthday","value":23,"unit":"count"},
                          {"id":"gave","value":9,"unit":"count"},
                          {"id":"used","value":28,"unit":"count"}],
            "ops":[["add","had","bought","tmp1"],
                   ["add","tmp1","birthday","total"],
                   ["add","gave","used","given"],
                   ["sub","total","given","left"]],
            "target":"left"}"#,
        17,
        "10 stickers, +21 bought, +23 birthday, -9 given away, -28 used. Left?",
    );

    // 83. Grace and Alex's combined weight (GSM8K #83). Answer: 623 (pounds).
    check(
        &mut host,
        "grace_alex",
        r#"{"quantities":[{"id":"grace","value":125,"unit":"count"},
                          {"id":"four","value":4,"unit":"scalar"},
                          {"id":"two_lbs","value":2,"unit":"count"}],
            "ops":[["mul","grace","four","tmp"],
                   ["sub","tmp","two_lbs","alex"],
                   ["add","grace","alex","combined"]],
            "target":"combined"}"#,
        623,
        "Grace=125lbs. Alex=4x Grace minus 2lbs. Combined weight?",
    );

    // 84. Dan's rose bush thorns (GSM8K #84). Answer: 600.
    check(
        &mut host,
        "dan_roses",
        r#"{"quantities":[{"id":"bushes","value":3,"unit":"count"},
                          {"id":"roses_per_bush","value":25,"unit":"count_per_count"},
                          {"id":"thorns_per_rose","value":8,"unit":"count_per_count"}],
            "ops":[["mul","bushes","roses_per_bush","roses"],
                   ["mul","roses","thorns_per_rose","thorns"]],
            "target":"thorns"}"#,
        600,
        "3 rose bushes, 25 roses/bush, 8 thorns/rose. Total thorns?",
    );

    // 85. The football team's wins (GSM8K #85) — the ground truth solves via a variable (L for
    // losses); the plan IR has no unknowns to solve for, so it goes straight to the closed form:
    // wins = (games + win_margin) / 2. Answer: 15.
    check(
        &mut host,
        "football_team",
        r#"{"quantities":[{"id":"games","value":22,"unit":"count"},
                          {"id":"eight","value":8,"unit":"count"},
                          {"id":"two","value":2,"unit":"scalar"}],
            "ops":[["add","games","eight","sum"],
                   ["div","sum","two","wins"]],
            "target":"wins"}"#,
        15,
        "22 games played, won 8 more than lost. Games won?",
    );

    // 86. Gene's quilt blocks (GSM8K #86). Answer: 44.
    check(
        &mut host,
        "gene_quilt",
        r#"{"quantities":[{"id":"now","value":34,"unit":"count"},
                          {"id":"started","value":23,"unit":"count"},
                          {"id":"four","value":4,"unit":"count_per_count"}],
            "ops":[["sub","now","started","years"],
                   ["mul","years","four","blocks"]],
            "target":"blocks"}"#,
        44,
        "Vacationing since 23, now 34, 4 vacations/year, 1 shirt/vacation. Quilt blocks?",
    );

    // 87. Greg's alarm rings (GSM8K #87). Answer: 22.
    check(
        &mut host,
        "greg_alarm",
        r#"{"quantities":[{"id":"first","value":4,"unit":"count"},
                          {"id":"three","value":3,"unit":"scalar"},
                          {"id":"two","value":2,"unit":"scalar"}],
            "ops":[["mul","first","three","second"],
                   ["div","second","two","third"],
                   ["add","first","second","tmp"],
                   ["add","tmp","third","total"]],
            "target":"total"}"#,
        22,
        "Alarm rings 4x, then 3x as long, then half that long. Total rings?",
    );

    // 88. Sylvie's salary raise (GSM8K #88, cents). Answer: 936000.
    check(
        &mut host,
        "sylvie_salary",
        r#"{"quantities":[{"id":"monthly","value":60000,"unit":"money_per_count"},
                          {"id":"months","value":12,"unit":"count"},
                          {"id":"ten","value":10,"unit":"scalar"},
                          {"id":"hundred","value":100,"unit":"scalar"},
                          {"id":"three","value":3,"unit":"scalar"}],
            "ops":[["mul","monthly","months","annual"],
                   ["mul","annual","ten","tmp"],
                   ["div","tmp","hundred","increase"],
                   ["mul","increase","three","three_year"],
                   ["add","annual","three_year","final_salary"]],
            "target":"final_salary"}"#,
        936000,
        "$600/month, +10%/year after 5yrs, 3 more years served (cents). Annual salary?",
    );

    // 89. Marilyn and Harald's records (GSM8K #89) — same closed-form-over-variable pattern as
    // row 85: m=10h and m+h=total collapses to h=total/11 without ever naming m. Answer: 8000.
    check(
        &mut host,
        "marilyn_harald",
        r#"{"quantities":[{"id":"total","value":88000,"unit":"count"},
                          {"id":"eleven","value":11,"unit":"scalar"}],
            "ops":[["div","total","eleven","harald"]],
            "target":"harald"}"#,
        8000,
        "Marilyn sold 10x Harald's copies, 88,000 combined. Harald's copies?",
    );

    // 90. Christina's gift bags (GSM8K #90, cents). Answer: 2400.
    check(
        &mut host,
        "christina_giftbags",
        r#"{"quantities":[{"id":"guests","value":16,"unit":"count"},
                          {"id":"three","value":3,"unit":"scalar"},
                          {"id":"four","value":4,"unit":"scalar"},
                          {"id":"price","value":200,"unit":"money_per_count"}],
            "ops":[["mul","guests","three","tmp"],
                   ["div","tmp","four","bags"],
                   ["mul","bags","price","total"]],
            "target":"total"}"#,
        2400,
        "16 guests, 0.75 gift bags/guest @$2 each (cents). Total spend?",
    );

    // 91. Ted's potato salad (GSM8K #91). Answer: 225 (pounds).
    check(
        &mut host,
        "ted_dinosaur",
        r#"{"quantities":[{"id":"adults","value":20,"unit":"count"},
                          {"id":"adult_rate","value":10,"unit":"count_per_count"},
                          {"id":"children","value":5,"unit":"count"},
                          {"id":"child_rate","value":5,"unit":"count_per_count"}],
            "ops":[["mul","adults","adult_rate","adult_total"],
                   ["mul","children","child_rate","child_total"],
                   ["add","adult_total","child_total","total"]],
            "target":"total"}"#,
        225,
        "20 adults @10lbs, 5 children @5lbs potato salad. Total pounds?",
    );

    // 92. Jan, Marcia, and Cindy's pets (GSM8K #92). Answer: 28.
    check(
        &mut host,
        "jan_marcia_cindy",
        r#"{"quantities":[{"id":"cindy","value":4,"unit":"count"},
                          {"id":"two","value":2,"unit":"count"},
                          {"id":"three","value":3,"unit":"scalar"}],
            "ops":[["add","cindy","two","marcia"],
                   ["mul","marcia","three","jan"],
                   ["add","cindy","marcia","tmp"],
                   ["add","tmp","jan","total"]],
            "target":"total"}"#,
        28,
        "Cindy=4 pets, Marcia=Cindy+2, Jan=3x Marcia. Total pets?",
    );

    // 93. Emily's kids' ages (GSM8K #93). Answer: 4 (Jackson's age).
    check(
        &mut host,
        "emily_kids",
        r#"{"quantities":[{"id":"james","value":10,"unit":"count"},
                          {"id":"one","value":1,"unit":"count"},
                          {"id":"two","value":2,"unit":"count"},
                          {"id":"five","value":5,"unit":"count"}],
            "ops":[["add","james","one","corey"],
                   ["sub","corey","two","amy"],
                   ["sub","amy","five","jackson"]],
            "target":"jackson"}"#,
        4,
        "James=10, 1yr younger than Corey. Amy=Corey-2=Jackson+5. Jackson's age?",
    );

    // 94. Lee and Gerald's hurdles time (GSM8K #94). Answer: 36 (seconds).
    check(
        &mut host,
        "lee_gerald",
        r#"{"quantities":[{"id":"lee","value":38,"unit":"time"},
                          {"id":"two","value":2,"unit":"time"},
                          {"id":"ten","value":10,"unit":"scalar"},
                          {"id":"hundred","value":100,"unit":"scalar"}],
            "ops":[["add","lee","two","gerald_initial"],
                   ["mul","gerald_initial","ten","tmp"],
                   ["div","tmp","hundred","reduction"],
                   ["sub","gerald_initial","reduction","gerald_final"]],
            "target":"gerald_final"}"#,
        36,
        "Lee runs 38s, 2s faster than Gerald used to. Gerald's diet cuts his time 10%. New time?",
    );

    // 95. Rabbits, dogs, and cats (GSM8K #95). Answer: 348.
    check(
        &mut host,
        "rabbits_dogs_cats",
        r#"{"quantities":[{"id":"dogs","value":60,"unit":"count"},
                          {"id":"two","value":2,"unit":"scalar"},
                          {"id":"twelve","value":12,"unit":"count"}],
            "ops":[["mul","dogs","two","cats"],
                   ["add","dogs","cats","combined"],
                   ["sub","combined","twelve","rabbits"],
                   ["add","rabbits","combined","total"]],
            "target":"total"}"#,
        348,
        "60 dogs, 2 cats/dog, rabbits = combined dogs+cats minus 12. Total pets?",
    );

    // 96. Grade 5 girls not in girl scouts (GSM8K #96). Answer: 40.
    check(
        &mut host,
        "grade5_girls",
        r#"{"quantities":[{"id":"students","value":200,"unit":"count"},
                          {"id":"two","value":2,"unit":"scalar"},
                          {"id":"five","value":5,"unit":"scalar"},
                          {"id":"three","value":3,"unit":"scalar"}],
            "ops":[["mul","students","two","tmp1"],
                   ["div","tmp1","five","boys"],
                   ["sub","students","boys","girls"],
                   ["mul","girls","two","tmp2"],
                   ["div","tmp2","three","scouts"],
                   ["sub","girls","scouts","not_scouts"]],
            "target":"not_scouts"}"#,
        40,
        "200 students, 2/5 boys, 2/3 of girls in girl scouts. Girls not in scouts?",
    );

    // 97. Harry and James's sleep (GSM8K #97). Answer: 3 (hours).
    check(
        &mut host,
        "harry_james",
        r#"{"quantities":[{"id":"harry","value":9,"unit":"time"},
                          {"id":"two","value":2,"unit":"scalar"},
                          {"id":"three","value":3,"unit":"scalar"}],
            "ops":[["mul","harry","two","tmp"],
                   ["div","tmp","three","james"],
                   ["sub","harry","james","diff"]],
            "target":"diff"}"#,
        3,
        "Harry slept 9hrs, James slept 2/3 of that. How much more did Harry sleep?",
    );

    // 98. Freda's tomato sauce (GSM8K #98). Answer: 12 (tomatoes).
    check(
        &mut host,
        "freda_tomatoes",
        r#"{"quantities":[{"id":"sauce","value":32,"unit":"count"},
                          {"id":"two","value":2,"unit":"scalar"},
                          {"id":"can_size","value":16,"unit":"count_per_count"},
                          {"id":"tomatoes_per_can","value":3,"unit":"count_per_count"}],
            "ops":[["mul","sauce","two","tomatoes_vol"],
                   ["div","tomatoes_vol","can_size","cans"],
                   ["mul","cans","tomatoes_per_can","tomatoes"]],
            "target":"tomatoes"}"#,
        12,
        "32oz sauce (half its tomato volume), 16oz cans of 3 tomatoes each. Tomatoes used?",
    );

    // 99. Cars through the motorway jam (GSM8K #99). Answer: 5.
    check(
        &mut host,
        "cars_motorway",
        r#"{"quantities":[{"id":"orig","value":30,"unit":"count"},
                          {"id":"exited","value":5,"unit":"count"},
                          {"id":"last15","value":20,"unit":"count"}],
            "ops":[["sub","orig","exited","remaining"],
                   ["sub","remaining","last15","first15"]],
            "target":"first15"}"#,
        5,
        "30 cars, 5 exited, 20 drove through in the last 15min. Cars in the first 15min?",
    );

    // 100. Mary's potted plants (GSM8K #100). Answer: 58.
    check(
        &mut host,
        "mary_plants",
        r#"{"quantities":[{"id":"ledges","value":40,"unit":"count"},
                          {"id":"two","value":2,"unit":"scalar"},
                          {"id":"eighteen","value":18,"unit":"count"}],
            "ops":[["mul","ledges","two","before"],
                   ["add","before","eighteen","after"],
                   ["sub","after","ledges","remain"]],
            "target":"remain"}"#,
        58,
        "40 ledges x2 plants +18 new, gives away 1/ledge. Plants remaining?",
    );

    // 101. Jerome's doorbell rings (GSM8K #101). Answer: 175.
    check(
        &mut host,
        "jerome_doorbell",
        r#"{"quantities":[{"id":"first","value":20,"unit":"count"},
                          {"id":"four","value":4,"unit":"scalar"},
                          {"id":"fourth","value":60,"unit":"count"},
                          {"id":"ten","value":10,"unit":"count"}],
            "ops":[["div","first","four","quarter"],
                   ["add","first","quarter","second"],
                   ["add","first","second","firsttwo"],
                   ["add","fourth","ten","third"],
                   ["add","third","fourth","lasttwo"],
                   ["add","firsttwo","lasttwo","total"]],
            "target":"total"}"#,
        175,
        "4 friends ring doorbell: 20, +1/4 more, third=fourth+10, fourth=60. Total rings?",
    );

    // 102. Solo's reading pages (GSM8K #102). Answer: 6 (pages/day).
    check(
        &mut host,
        "solo_pages",
        r#"{"quantities":[{"id":"science","value":4,"unit":"count"},
                          {"id":"social","value":20,"unit":"count"},
                          {"id":"history","value":7,"unit":"count"},
                          {"id":"geo","value":8,"unit":"count"},
                          {"id":"monday","value":15,"unit":"count"},
                          {"id":"four_days","value":4,"unit":"scalar"}],
            "ops":[["add","science","social","t1"],
                   ["add","t1","history","t2"],
                   ["add","t2","geo","totalpages"],
                   ["sub","totalpages","monday","remainder"],
                   ["div","remainder","four_days","perday"]],
            "target":"perday"}"#,
        6,
        "4+20+7+8 pages across subjects, read 15 Monday, 4 days left. Pages/day needed?",
    );

    // 103. John's glasses of water (GSM8K #103). Answer: 26.
    check(
        &mut host,
        "john_water",
        r#"{"quantities":[{"id":"weekday_glasses","value":4,"unit":"count"},
                          {"id":"weekdays","value":5,"unit":"scalar"},
                          {"id":"weekend_glasses","value":3,"unit":"count"},
                          {"id":"weekend_days","value":2,"unit":"scalar"}],
            "ops":[["mul","weekday_glasses","weekdays","wd"],
                   ["mul","weekend_glasses","weekend_days","we"],
                   ["add","wd","we","total"]],
            "target":"total"}"#,
        26,
        "4 glasses/weekday x5, 3 glasses/weekend-day x2. Glasses/week?",
    );

    // 104. The fog bank (GSM8K #104). Answer: 140 (minutes).
    check(
        &mut host,
        "fog_bank",
        r#"{"quantities":[{"id":"city","value":42,"unit":"distance"},
                          {"id":"per","value":3,"unit":"distance"},
                          {"id":"interval_time","value":10,"unit":"time"}],
            "ops":[["div","city","per","intervals"],
                   ["mul","intervals","interval_time","total_time"]],
            "target":"total_time"}"#,
        140,
        "Fog covers 3mi every 10min, city is 42mi across. Minutes to cover the city?",
    );

    // 105. Poppy's jigsaw puzzle (GSM8K #105). Answer: 500 (pieces).
    check(
        &mut host,
        "poppy_puzzle",
        r#"{"quantities":[{"id":"pieces","value":1000,"unit":"count"},
                          {"id":"four","value":4,"unit":"scalar"},
                          {"id":"three","value":3,"unit":"scalar"}],
            "ops":[["div","pieces","four","placed1"],
                   ["sub","pieces","placed1","remaining1"],
                   ["div","remaining1","three","placed2"],
                   ["sub","remaining1","placed2","remaining2"]],
            "target":"remaining2"}"#,
        500,
        "1000-piece puzzle: Poppy places 1/4, mom places 1/3 of the rest. Pieces left?",
    );

    // 106. Cody and Amir's cookies (GSM8K #106). Answer: 20.
    check(
        &mut host,
        "cody_amir",
        r#"{"quantities":[{"id":"amir","value":5,"unit":"count"},
                          {"id":"three","value":3,"unit":"scalar"}],
            "ops":[["mul","amir","three","cody"],
                   ["add","cody","amir","total"]],
            "target":"total"}"#,
        20,
        "Cody eats 3x as many cookies as Amir. Amir eats 5. Total eaten together?",
    );

    // 107. John's boxes' inner volume (GSM8K #107). Answer: 72 (cubic inches).
    check(
        &mut host,
        "john_boxes",
        r#"{"quantities":[{"id":"w","value":5,"unit":"distance"},
                          {"id":"h","value":6,"unit":"distance"},
                          {"id":"d","value":4,"unit":"distance"},
                          {"id":"wall","value":2,"unit":"distance"},
                          {"id":"three_boxes","value":3,"unit":"scalar"}],
            "ops":[["sub","w","wall","w2"],
                   ["sub","h","wall","h2"],
                   ["sub","d","wall","d2"],
                   ["mul","w2","h2","tmp"],
                   ["mul","tmp","d2","vol"],
                   ["mul","vol","three_boxes","total"]],
            "target":"total"}"#,
        72,
        "3 boxes, 5x6x4in, 1in-thick walls. Total inner volume?",
    );

    // 108. Frankie's TV watching (GSM8K #108). Answer: 3 (30-min episodes).
    check(
        &mut host,
        "frankie_tv",
        r#"{"quantities":[{"id":"total_min","value":420,"unit":"time"},
                          {"id":"mon","value":60,"unit":"time"},
                          {"id":"tue","value":60,"unit":"time"},
                          {"id":"thu_ep","value":60,"unit":"time"},
                          {"id":"thu_show","value":30,"unit":"time"},
                          {"id":"fri","value":120,"unit":"time"},
                          {"id":"thirty","value":30,"unit":"time"}],
            "ops":[["add","thu_ep","thu_show","thu"],
                   ["add","mon","tue","t1"],
                   ["add","t1","thu","t2"],
                   ["add","t2","fri","used"],
                   ["sub","total_min","used","remaining"],
                   ["div","remaining","thirty","episodes"]],
            "target":"episodes"}"#,
        3,
        "7hrs TV: Mon/Tue 1hr, Thu 1.5hr, Fri 2hr, rest in 30-min shows Wed. How many Wed shows?",
    );

    // 109. Henry's cookie baking (GSM8K #109) — a different Henry from row 29. Answer: 50.
    check(
        &mut host,
        "henry_cookies",
        r#"{"quantities":[{"id":"total","value":110,"unit":"count"},
                          {"id":"dropped","value":5,"unit":"count"},
                          {"id":"fifteen","value":15,"unit":"count"},
                          {"id":"two","value":2,"unit":"scalar"}],
            "ops":[["add","total","dropped","before_drop"],
                   ["sub","before_drop","fifteen","before_extra"],
                   ["div","before_extra","two","last_year"]],
            "target":"last_year"}"#,
        50,
        "Baked 2x last year +15 extra, dropped 5, has 110 now. Cookies last year?",
    );

    // 110. Gas station cashback (GSM8K #110, cents). Answer: 2800.
    check(
        &mut host,
        "gas_station",
        r#"{"quantities":[{"id":"price","value":300,"unit":"money_per_count"},
                          {"id":"gallons","value":10,"unit":"count"},
                          {"id":"cashback_rate","value":20,"unit":"money_per_count"}],
            "ops":[["mul","price","gallons","spent"],
                   ["mul","cashback_rate","gallons","cashback"],
                   ["sub","spent","cashback","net"]],
            "target":"net"}"#,
        2800,
        "$3.00/gallon, 10 gallons, $0.20/gallon cashback (cents). Net cost?",
    );

    // 111. Marcell and Beatrice's fruit roll-ups (GSM8K #111). Answer: 45 (average).
    check(
        &mut host,
        "marcell_beatrice",
        r#"{"quantities":[{"id":"beatrice_wide","value":2,"unit":"count"},
                          {"id":"beatrice_long","value":24,"unit":"count"},
                          {"id":"marcell_wide","value":3,"unit":"count"},
                          {"id":"marcell_long","value":14,"unit":"count"},
                          {"id":"two","value":2,"unit":"scalar"}],
            "ops":[["mul","beatrice_long","beatrice_wide","beatrice_total"],
                   ["mul","marcell_long","marcell_wide","marcell_total"],
                   ["add","beatrice_total","marcell_total","total"],
                   ["div","total","two","average"]],
            "target":"average"}"#,
        45,
        "Beatrice's roll-up 2x24, Marcell's 3x14. Average eaten?",
    );

    // 112. Julia's leaking boat (GSM8K #112). Answer: 16 (liters).
    check(
        &mut host,
        "julia_boat",
        r#"{"quantities":[{"id":"shore_time","value":64,"unit":"time"},
                          {"id":"row_time","value":16,"unit":"time"},
                          {"id":"row_dist","value":20,"unit":"distance"},
                          {"id":"ten","value":10,"unit":"distance"},
                          {"id":"two","value":2,"unit":"count"}],
            "ops":[["div","shore_time","row_time","multiples"],
                   ["mul","row_dist","multiples","dist"],
                   ["div","dist","ten","water_units"],
                   ["mul","water_units","two","water_total"]],
            "target":"water_total"}"#,
        16,
        "2L taken on per 10ft rowed, 16s per 20ft, shore 64s away. Total water taken on?",
    );

    // 113. The classroom whiteboard (GSM8K #113). Answer: 24 (cleanings/day).
    check(
        &mut host,
        "whiteboard",
        r#"{"quantities":[{"id":"teachers","value":4,"unit":"count"},
                          {"id":"lessons","value":2,"unit":"scalar"},
                          {"id":"cleans","value":3,"unit":"scalar"}],
            "ops":[["mul","teachers","lessons","total_lessons"],
                   ["mul","total_lessons","cleans","total"]],
            "target":"total"}"#,
        24,
        "4 teachers x2 lessons/day, whiteboard cleaned 3x/lesson. Cleanings/day?",
    );

    // 114. Ryan's flowers (GSM8K #114). Answer: 25.
    check(
        &mut host,
        "ryan_flowers",
        r#"{"quantities":[{"id":"rate","value":2,"unit":"count_per_count"},
                          {"id":"days","value":15,"unit":"count"},
                          {"id":"five","value":5,"unit":"count"}],
            "ops":[["mul","rate","days","planted"],
                   ["sub","planted","five","total"]],
            "target":"total"}"#,
        25,
        "Ryan plants 2 flowers/day for 15 days, 5 didn't grow. Flowers he has now?",
    );

    // 115. Jamal's phone photos (GSM8K #115). Answer: 6 (ducks).
    check(
        &mut host,
        "jamal_phone",
        r#"{"quantities":[{"id":"jamal","value":1800,"unit":"count"},
                          {"id":"six","value":6,"unit":"scalar"},
                          {"id":"fifty","value":50,"unit":"scalar"}],
            "ops":[["div","jamal","six","brittany"],
                   ["div","brittany","fifty","ducks"]],
            "target":"ducks"}"#,
        6,
        "Jamal's phone (1800 photos) holds 6x Brittany's, which holds 50x the duck count. Ducks?",
    );

    // 116. Sasha's lumber profit (GSM8K #116, cents). Answer: 9000.
    check(
        &mut host,
        "sasha_lumber",
        r#"{"quantities":[{"id":"n2x4","value":10,"unit":"count"},
                          {"id":"price2x4","value":1000,"unit":"money_per_count"},
                          {"id":"n4x4","value":5,"unit":"count"},
                          {"id":"price4x4","value":1600,"unit":"money_per_count"},
                          {"id":"three","value":3,"unit":"scalar"},
                          {"id":"two","value":2,"unit":"scalar"}],
            "ops":[["mul","n2x4","price2x4","cost1"],
                   ["mul","n4x4","price4x4","cost2"],
                   ["add","cost1","cost2","totalcost"],
                   ["mul","totalcost","three","tmp"],
                   ["div","tmp","two","newprice"],
                   ["sub","newprice","totalcost","profit"]],
            "target":"profit"}"#,
        9000,
        "10 boards@$10 + 5 boards@$16, prices +50% since (cents). Profit selling all?",
    );

    // 117. Katy's sugar-to-water ratio (GSM8K #117). Answer: 42 (teaspoons).
    check(
        &mut host,
        "katy_sugar",
        r#"{"quantities":[{"id":"total","value":120,"unit":"count"},
                          {"id":"seven","value":7,"unit":"scalar"},
                          {"id":"twenty","value":20,"unit":"scalar"}],
            "ops":[["mul","total","seven","tmp"],
                   ["div","tmp","twenty","sugar"]],
            "target":"sugar"}"#,
        42,
        "Sugar:water ratio 7:13, 120 total. Teaspoons of sugar?",
    );

    // 118. John's shoes for his kids (GSM8K #118, cents). Answer: 36000.
    check(
        &mut host,
        "john_shoes",
        r#"{"quantities":[{"id":"children","value":3,"unit":"count"},
                          {"id":"two","value":2,"unit":"scalar"},
                          {"id":"price","value":6000,"unit":"money_per_count"}],
            "ops":[["mul","children","two","pairs"],
                   ["mul","pairs","price","total"]],
            "target":"total"}"#,
        36000,
        "2 pairs of shoes for each of 3 kids, $60/pair (cents). Total paid?",
    );

    // 119. Customs containers (GSM8K #119). Answer: 4 (containers).
    check(
        &mut host,
        "customs_containers",
        r#"{"quantities":[{"id":"containers1","value":2,"unit":"count"},
                          {"id":"vehicles_per","value":5,"unit":"count_per_count"},
                          {"id":"totalv","value":30,"unit":"count"}],
            "ops":[["mul","containers1","vehicles_per","day1v"],
                   ["sub","totalv","day1v","day2v"],
                   ["div","day2v","vehicles_per","containers2"]],
            "target":"containers2"}"#,
        4,
        "2 containers of 5 vehicles day 1, 30 vehicles total, 5/container day 2. Day-2 containers?",
    );

    // 120. Adrien and Lylah's combined salary (GSM8K #120, cents) — the largest plan in this
    // batch (10 ops) but every division lands exact, so no rescale judgment was needed.
    // Answer: 9520000.
    check(
        &mut host,
        "adrien_lylah",
        r#"{"quantities":[{"id":"adrien4","value":4000000,"unit":"money"},
                          {"id":"forty","value":40,"unit":"scalar"},
                          {"id":"hundred","value":100,"unit":"scalar"},
                          {"id":"thirty","value":30,"unit":"scalar"}],
            "ops":[["mul","adrien4","forty","r1"],
                   ["div","r1","hundred","raise_a"],
                   ["add","adrien4","raise_a","adrien_later"],
                   ["mul","adrien4","thirty","r2"],
                   ["div","r2","hundred","discount_l"],
                   ["sub","adrien4","discount_l","lylah4"],
                   ["mul","lylah4","forty","r3"],
                   ["div","r3","hundred","raise_l"],
                   ["add","lylah4","raise_l","lylah_later"],
                   ["add","adrien_later","lylah_later","total"]],
            "target":"total"}"#,
        9520000,
        "Adrien $40k(4yr ago)=Lylah+30%, both +40% since (cents). Combined salary now?",
    );

    // 121. Miguel's paper usage (GSM8K #121). Answer: 240 (sheets/month).
    check(
        &mut host,
        "miguel_paper",
        r#"{"quantities":[{"id":"sheets_per_pad","value":30,"unit":"count"},
                          {"id":"pads","value":2,"unit":"scalar"},
                          {"id":"four","value":4,"unit":"scalar"}],
            "ops":[["mul","sheets_per_pad","pads","weekly"],
                   ["mul","weekly","four","monthly"]],
            "target":"monthly"}"#,
        240,
        "2 pads/week, 30 sheets/pad, 4 weeks/month. Sheets/month?",
    );

    // 122. Morisette and Kael's fruits (GSM8K #122). Answer: 27.
    check(
        &mut host,
        "morisette_kael",
        r#"{"quantities":[{"id":"m_apples","value":5,"unit":"count"},
                          {"id":"m_oranges","value":8,"unit":"count"},
                          {"id":"two","value":2,"unit":"scalar"}],
            "ops":[["mul","m_apples","two","k_apples"],
                   ["div","m_oranges","two","k_oranges"],
                   ["add","m_apples","k_apples","tmp1"],
                   ["add","m_oranges","k_oranges","tmp2"],
                   ["add","tmp1","tmp2","total"]],
            "target":"total"}"#,
        27,
        "Morisette: 5 apples, 8 oranges. Kael: 2x apples, half oranges. Total fruits?",
    );

    // 123. Sadie's sleep for the week (GSM8K #123). Answer: 48 (hours).
    check(
        &mut host,
        "sadie_sleep",
        r#"{"quantities":[{"id":"monday","value":8,"unit":"time"},
                          {"id":"two","value":2,"unit":"time"},
                          {"id":"one","value":1,"unit":"time"},
                          {"id":"two_days","value":2,"unit":"scalar"},
                          {"id":"four_days","value":4,"unit":"scalar"}],
            "ops":[["sub","monday","two","next2each"],
                   ["mul","next2each","two_days","next2total"],
                   ["add","next2each","one","resteach"],
                   ["mul","resteach","four_days","resttotal"],
                   ["add","monday","next2total","t1"],
                   ["add","t1","resttotal","total"]],
            "target":"total"}"#,
        48,
        "Mon 8hrs, next 2 days -2hrs each, rest of week +1hr vs those. Total for the week?",
    );

    // 124. Rosie's run (GSM8K #124). Answer: 50 (miles).
    check(
        &mut host,
        "rosie_running",
        r#"{"quantities":[{"id":"speed1","value":10,"unit":"distance_per_time"},
                          {"id":"hours1","value":3,"unit":"time"},
                          {"id":"speed2","value":5,"unit":"distance_per_time"},
                          {"id":"total_hours","value":7,"unit":"time"}],
            "ops":[["mul","speed1","hours1","dist1"],
                   ["sub","total_hours","hours1","remaining_h"],
                   ["mul","speed2","remaining_h","dist2"],
                   ["add","dist1","dist2","total"]],
            "target":"total"}"#,
        50,
        "10mph for 3hrs, then 5mph. Miles run in 7hrs total?",
    );

    // 125. Jennie's stamped letters (GSM8K #125). Answer: 10.
    check(
        &mut host,
        "jennie_stamps",
        r#"{"quantities":[{"id":"letters","value":60,"unit":"count"},
                          {"id":"three","value":3,"unit":"scalar"},
                          {"id":"thirty","value":30,"unit":"count"}],
            "ops":[["div","letters","three","stamped"],
                   ["sub","thirty","stamped","before"]],
            "target":"before"}"#,
        10,
        "60 letters need stamps, 1/3 stamped, 30 now in the stamped pile. Pile size before?",
    );

    // 126. Julia's spoons (GSM8K #126) — a different Julia from row 112. Answer: 10.
    check(&mut host, "julia_spoons",
        r#"{"quantities":[{"id":"total","value":12,"unit":"count"},
                          {"id":"used","value":3,"unit":"count"},
                          {"id":"husband","value":5,"unit":"count"}],
            "ops":[["add","total","used","combined"],
                   ["sub","combined","husband","juliapack"]],
            "target":"juliapack"}"#, 10,
        "Husband gave a 5-spoon pack, 3 used sampling stew, 12 total at the table. Julia's pack size?");

    // 127. Dylan's sausages (GSM8K #127). Answer: 82.
    check(
        &mut host,
        "dylan_sausages",
        r#"{"quantities":[{"id":"chicken","value":38,"unit":"count"},
                          {"id":"six","value":6,"unit":"count"}],
            "ops":[["add","chicken","six","fish"],
                   ["add","chicken","fish","total"]],
            "target":"total"}"#,
        82,
        "38 chicken sausages, 6 more fish sausages than chicken. Total bought?",
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
