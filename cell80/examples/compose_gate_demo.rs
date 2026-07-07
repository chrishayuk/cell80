//! `cell80 compose` end to end, as a library caller: three derivations of one word
//! problem — inline arithmetic, a library-composed spelling, and a deliberately
//! wrong reading — through canonicalization (slots, defer-division, lifting), the
//! link loop, the registered 2-of-3 agreement gate, and the counterfactual battery.
//!
//!     cargo run -p cell80 --example compose_gate_demo
//!
//! The point to watch: the two correct derivations spell the computation
//! differently (different nouns, different structure), yet the wrong one is
//! outvoted and the accepted answer carries a battery certificate — the agreement
//! survived perturbing every shared quantity, so it isn't a coincidence of these
//! particular numbers.

use cell80::compose::{agreement, battery, compose, run_composed};
use cell80::{Cartridge, CartridgeOpts, CellConfig, CellHost, DEFAULT_CYCLES};
use std::path::Path;

fn main() -> Result<(), String> {
    // "A crate holds 30 pencils. Marcy fills 5 crates, then gives 20 pencils away.
    //  How many pencils does she have?"  (30*5 - 20 = 130)
    let derivations = [
        // Inline arithmetic, the model's own nouns.
        "fn run() -> u16 { let per_crate = 30; let crates = 5; let given = 20; per_crate * crates - given }",
        // Library-composed spelling (different nouns, calls a verified cell).
        "fn run() -> u16 { let pencils = 30; let boxes = 5; let away = 20; abs_diff(pencils * boxes, away) }",
        // A misread: "gives 20 away per crate" — wrong, and outvoted.
        "fn run() -> u16 { let per_crate = 30; let crates = 5; let given = 20; (per_crate - given) * crates }",
    ];

    let cells_dir = format!("{}/cells", env!("CARGO_MANIFEST_DIR"));
    let mut host = CellHost::new();
    for path in walkdir(Path::new(&cells_dir))? {
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let src = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let id = path.file_stem().unwrap().to_string_lossy().to_string();
        let entry = src
            .lines()
            .find_map(|l| l.strip_prefix("//! entry:").map(|e| e.trim().to_string()));
        if let Ok(cart) = Cartridge::compile(
            &src,
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some(id),
                entry,
                ..Default::default()
            },
        ) {
            host.add(cart);
        }
    }

    let mut outcomes = Vec::new();
    for (i, src) in derivations.iter().enumerate() {
        match compose(&host, Path::new(&cells_dir), src) {
            Ok(comp) => {
                println!(
                    "derivation {i}: linked {:?}",
                    comp.resolutions
                        .iter()
                        .map(|r| format!("{} -> {}", r.name, r.cell_id))
                        .collect::<Vec<_>>()
                );
                outcomes.push(run_composed(&mut host, comp, &[], DEFAULT_CYCLES)?);
            }
            Err(e) => {
                println!("derivation {i}: {e}");
            }
        }
    }
    let answers: Vec<Option<u64>> = outcomes.iter().map(|o| o.answer).collect();
    println!("answers: {answers:?}");
    let (answer, gate, flagged) = agreement(&answers);
    println!(
        "gate: {gate}{}",
        if flagged { " (flagged 2-of-3)" } else { "" }
    );

    if let Some(top) = answer {
        let accepted: Vec<_> = outcomes.iter().filter(|o| o.answer == Some(top)).collect();
        let rep = battery(&mut host, &accepted, DEFAULT_CYCLES)?;
        match rep.failed_on {
            None => println!(
                "accepted: {top} — battery survived {} perturbation(s)",
                rep.perturbed.len()
            ),
            Some(v) => println!("battery escalation: agreement broke at {v} — coincidental"),
        }
    } else {
        println!("escalate — no confident answer");
    }
    Ok(())
}

fn walkdir(dir: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).map_err(|e| e.to_string())? {
            let p = e.map_err(|e| e.to_string())?.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push(p);
            }
        }
    }
    out.sort();
    Ok(out)
}
