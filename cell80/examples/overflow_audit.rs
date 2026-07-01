//! **u16 overflow audit** — sizes the `u32`-in-state prize before the ABI/codegen change.
//!
//! For each overflow-prone *value* cell, run it on an **in-domain** input where the true (u32)
//! result — or a necessary intermediate — exceeds `u16`, and compare to the cell's `u16` output.
//! A mismatch is a cell the `u16` ceiling makes silently wrong (`percent`/`ratio`/`scale`: the
//! `part*100` intermediate wraps) or forces to *self-clamp its own domain* (`square` refuses
//! `n > 255`). That is the friction pushing scoring/ratio cells toward "ask the agent to write
//! Python" — exactly what wider state removes. Re-run after `u32`-in-state lands to confirm the
//! fix flips these to `ok`. Deterministic, no model.
//!
//! Run: `cargo run --release --example overflow_audit -p cell80`.

use cell80::{Runner, DEFAULT_CYCLES};
use std::path::PathBuf;

/// Compile `cells/<id>.rs` and run it on `args`, returning the `u16` result.
fn run_cell(id: &str, args: &[u16]) -> u16 {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("cells")
        .join(format!("{id}.rs"));
    let src = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {id}: {e}"));
    let mut r = Runner::compile(&src).unwrap_or_else(|e| panic!("compile {id}: {e}"));
    r.run(None, args, DEFAULT_CYCLES)
        .unwrap_or_else(|e| panic!("run {id}: {e}"))
        .result
}

fn main() {
    // (id, args, true result under u32 math, overflow kind). Inputs are in each cell's stated
    // domain; the wrongness is purely the u16 working width, not misuse.
    let cases: &[(&str, &[u16], u32, &str)] = &[
        ("square", &[300], 90_000, "result"), // self-clamps n>255 → can't represent 300²
        ("weighted_sum", &[30000, 20000, 10000], 100_000, "result"), // a + 2b + 3c
        ("percent", &[700, 1000], 70, "intermediate"), // part*100 = 70000 wraps
        ("permille", &[700, 1000], 700, "intermediate"), // part*1000 wraps hard
        ("ratio_255", &[300, 255], 300, "intermediate"), // part*255 wraps
        ("scale_percent", &[1000, 200], 2000, "intermediate"), // value*pct wraps
        ("within_percent", &[1500, 1000, 100], 1, "predicate-flip"), // target*pct wraps → wrong bool
    ];

    println!("u16 overflow audit — true (u32) vs cell (u16) on in-domain inputs\n");
    println!(
        "  {:<16}{:<24}{:>11}{:>11}   {}",
        "cell", "args", "true(u32)", "cell(u16)", "verdict"
    );
    println!("  {}", "-".repeat(80));
    let mut wrong = 0;
    for (id, args, truth, kind) in cases {
        let got = run_cell(id, args) as u32;
        let ok = got == *truth;
        if !ok {
            wrong += 1;
        }
        println!(
            "  {:<16}{:<24}{:>11}{:>11}   {}",
            id,
            format!("{args:?}"),
            truth,
            got,
            if ok {
                "ok".to_string()
            } else {
                format!("WRONG ({kind} overflow)")
            }
        );
    }
    println!(
        "\n  {}/{} overflow-prone cells are wrong at u16 for in-domain inputs — the percent /\n  \
         ratio / scoring / square families. That is the u32-in-state prize, made concrete.",
        wrong,
        cases.len()
    );
}
