//! **u16 overflow audit** — sizes the `u32`-in-state prize before the ABI/codegen change.
//!
//! For each overflow-prone *value* cell, run it on an **in-domain** input where the true (u32)
//! result — or a necessary intermediate — exceeds `u16`, and compare to the cell's `u16` output.
//! A mismatch is a cell the `u16` ceiling makes silently wrong. Two overflow kinds, fixed by two
//! different stages: **intermediate** overflow (`percent`/`ratio`/`scale`: `part*100` wraps) is
//! fixed by `u32` *arithmetic* (widen → wide mul/div → narrow — landed; those rows read `ok`),
//! while **result** overflow (`square(300)` = 90000, `weighted_sum` ≥ 65536) can't fit the `u16`
//! return at all — that is the `u32`-in-state prize still open. Deterministic, no model.
//!
//! Run: `cargo run --release --example overflow_audit -p cell80`.

use cell80::{Runner, StateCell, DEFAULT_CYCLES};
use std::path::PathBuf;

fn cell_src(id: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("cells")
        .join(format!("{id}.rs"));
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {id}: {e}"))
}

/// Compile `cells/<id>.rs` and run it on `args`, returning the `u16` result.
fn run_cell(id: &str, args: &[u16]) -> u16 {
    let src = cell_src(id);
    let mut r = Runner::compile(&src).unwrap_or_else(|e| panic!("compile {id}: {e}"));
    r.run(None, args, DEFAULT_CYCLES)
        .unwrap_or_else(|e| panic!("run {id}: {e}"))
        .result
}

/// Run a wide state cell by named fields and read a `u32` output field back exactly.
fn run_wide(id: &str, state: &str, fields: &[(&str, u64)], out: &str) -> u64 {
    let mut cell =
        StateCell::bind(&cell_src(id), state, None).unwrap_or_else(|e| panic!("bind {id}: {e}"));
    for (f, v) in fields {
        cell.set(f, *v)
            .unwrap_or_else(|e| panic!("set {id}.{f}: {e}"));
    }
    cell.run(DEFAULT_CYCLES)
        .unwrap_or_else(|e| panic!("run {id}: {e}"));
    cell.get(out)
        .unwrap_or_else(|| panic!("{id} has no field `{out}`"))
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
        "  {:<16}{:<24}{:>11}{:>11}   verdict",
        "cell", "args", "true(u32)", "cell(u16)"
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
        "\n  {}/{} overflow-prone cells are wrong at u16 for in-domain inputs. u32 *arithmetic*\n  \
         (widen → wide mul/div → narrow) fixed the intermediate-overflow families; a value\n  \
         cell's u16 *return* stays capped by the register convention.",
        wrong,
        cases.len()
    );

    // The result-overflow fix: a wide **u32 state field** carries the exact value the
    // u16 return can't — driven and read by name over the typed-state ABI (v4 manifests
    // carry a width per field).
    type WideCase<'a> = (&'a str, &'a str, &'a [(&'a str, u64)], &'a str, u64);
    let wide: &[WideCase] = &[
        ("square_wide", "Sq", &[("n", 300)], "sq", 90_000),
        (
            "weighted_sum_wide",
            "Ws",
            &[("a", 30_000), ("b", 20_000), ("c", 10_000)],
            "sum",
            100_000,
        ),
        (
            "euclid_sq",
            "Pts",
            &[("x1", 0), ("y1", 0), ("x2", 300), ("y2", 400)],
            "dist",
            250_000,
        ),
    ];
    println!("\n  wide (u32-in-state) siblings — the exact result, read from the named field\n");
    println!(
        "  {:<18}{:<34}{:>11}{:>11}   verdict",
        "cell", "fields", "true(u32)", "field(u32)"
    );
    println!("  {}", "-".repeat(84));
    let mut wide_wrong = 0;
    for (id, state, fields, out, truth) in wide {
        let got = run_wide(id, state, fields, out);
        if got != *truth {
            wide_wrong += 1;
        }
        println!(
            "  {:<18}{:<34}{:>11}{:>11}   {}",
            format!("{id}.{out}"),
            format!("{fields:?}"),
            truth,
            got,
            if got == *truth { "ok" } else { "WRONG" }
        );
    }
    println!(
        "\n  {}/{} wide fields exact — the u16 ceiling is gone end-to-end: compute wide\n  \
         (u32 arithmetic), persist wide (u32 state), read wide (typed-state ABI by name).",
        wide.len() - wide_wrong,
        wide.len()
    );
}
