//! Composition verbs: `compose` (canonicalize + link + run + agreement gate),
//! `solve` (candidate-plan loop), and `graph` (run a JSON CellGraph).
use super::*;

/// `solve <plans.json> [--cycles N] [--json]` — the minimal `cell_solve` loop (M2,
/// docs/math-campaign-spec.md): each candidate plan renders to canonical dialect
/// Rust, compiles (the plan IS a cell, catalogued by artifact hash), runs with its
/// quantities as state fields, and is killed on any escalate/halt; disagreeing
/// survivors face the counterfactual battery. `plans.json` is one plan object or
/// an array of candidates.
/// `compose <dir> <src.rs> [<src2.rs> ...] [--args a,b,..] [--cycles N] [--facts f] [--json]`
/// — the M2.9 verb: canonicalize each source (Full, wide lane), link unknown calls
/// against the library in `<dir>`, compile, run with `--args`, and gate: one source
/// composes, several sources must agree (unanimous / majority-flagged / escalate).
/// With `--facts`, the accepted runs' fact rows are written out — composed answers
/// become re-verifiable procedural memory.
pub(super) fn cmd_compose(args: &[String]) -> Result<String, String> {
    let dir = args.first().ok_or(USAGE)?;
    let mut sources: Vec<&String> = Vec::new();
    let mut run_args: Vec<u16> = Vec::new();
    let mut cycles = DEFAULT_CYCLES;
    let mut facts_out: Option<&String> = None;
    let mut dump_canon = false;
    let mut json = false;
    let mut it = args[1..].iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--args" => {
                run_args = parse_args(it.next().ok_or("--args needs a,b,..")?)?;
            }
            "--cycles" => {
                cycles = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .ok_or("--cycles needs a number")?
            }
            "--facts" => facts_out = Some(it.next().ok_or("--facts needs a path")?),
            "--dump-canon" => dump_canon = true,
            "--json" => json = true,
            other if other.starts_with("--") => return Err(format!("unknown flag `{other}`")),
            _ => sources.push(a),
        }
    }
    if sources.is_empty() {
        return Err("compose needs at least one source file".into());
    }
    let mut host = host_from_dir(dir)?;
    host.set_cache(true);
    let cells_dir = std::path::Path::new(dir);
    if dump_canon {
        // Debug surface: print each source's canonical linked form and stop —
        // no run, no gate. What the artifact hash actually covers.
        let mut out = String::new();
        for path in &sources {
            let src = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
            let comp = crate::compose::compose(&host, cells_dir, &src)?;
            out.push_str(&comp.source);
            out.push('\n');
        }
        return Ok(out);
    }
    let mut outcomes: Vec<crate::compose::DerivationOutcome> = Vec::new();
    // Original source per derivation — the guard below scans the accepted ones
    // for wide literals (what the canonical/linked form may have folded away).
    let mut sources_text: Vec<String> = Vec::new();
    for path in &sources {
        let src = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        let outcome = match crate::compose::compose(&host, cells_dir, &src) {
            Ok(comp) => crate::compose::run_composed(&mut host, comp, &run_args, cycles)?,
            Err(e) => crate::compose::DerivationOutcome {
                answer: None,
                kill: Some(e),
                artifact: None,
                resolutions: Vec::new(),
                repairs: Vec::new(),
                retrieved: false,
                cycles: 0,
                trapped_ops: 0,
                handle: None,
                base_args: Vec::new(),
                lifted: Vec::new(),
                wide_ret: false,
            },
        };
        outcomes.push(outcome);
        sources_text.push(src);
    }
    let answers: Vec<Option<u64>> = outcomes.iter().map(|o| o.answer).collect();
    let (mut answer, mut agreement, flagged) = crate::compose::agreement(&answers);
    // The counterfactual battery (M2.8): an accepted multi-derivation agreement must
    // survive perturbation of the lifted quantities, or it was coincidental (the
    // `a+b == a*b` at 2,2 class) and escalates instead.
    let mut battery_note: Option<String> = None;
    if let Some(top) = answer {
        let acc_idx: Vec<usize> = (0..outcomes.len())
            .filter(|&i| outcomes[i].answer == Some(top) && outcomes[i].handle.is_some())
            .collect();
        let accepted: Vec<&crate::compose::DerivationOutcome> =
            acc_idx.iter().map(|&i| &outcomes[i]).collect();
        // Perturbations the battery actually ran — its coverage, not just its verdict.
        let mut verified = 0usize;
        if accepted.len() >= 2 && accepted.iter().any(|o| !o.lifted.is_empty()) {
            let rep = crate::compose::battery(&mut host, &accepted, cycles)?;
            match rep.failed_on {
                Some(v) => {
                    answer = None;
                    agreement = "battery_escalate";
                    battery_note = Some(format!(
                        "agreement did not survive perturbing {v} (coincidental)"
                    ));
                }
                None => {
                    verified = rep.perturbed.len();
                    battery_note = Some(format!(
                        "survived {} perturbation(s){}",
                        rep.perturbed.len(),
                        if rep.skipped.is_empty() {
                            String::new()
                        } else {
                            format!(
                                " ({} value(s) not liftable in every derivation, skipped)",
                                rep.skipped.len()
                            )
                        }
                    ));
                }
            }
        }
        // Battery-unverified guard (registered amendment 2026-07-08): a *majority*
        // accept the battery could not verify at all (zero perturbations — wide
        // values are unliftable, so the battery is structurally blind exactly
        // there), with wide values in play, escalates instead of accepting. The
        // flagged band's contract is "accepted agreements survived perturbation";
        // this refuses to pretend an unverifiable one did. Unanimous accepts are
        // exempt. Counterfactually verified over every captured campaign config
        // (8 configs, 160 rows): removes the single accepted-and-wrong (the row89
        // correlated misreading, 79200) at zero yield cost.
        if answer.is_some() && agreement == "majority" && verified == 0 {
            let wide = answer.unwrap() > u16::MAX as u64
                || acc_idx
                    .iter()
                    .any(|&i| crate::compose::has_wide_literal(&sources_text[i]));
            if wide {
                answer = None;
                agreement = "battery_unverified";
                battery_note = Some(
                    "majority with zero battery coverage over wide values — \
                     unverified agreement, escalate"
                        .into(),
                );
            }
        }
    }
    if let (Some(path), Some(_)) = (facts_out, answer) {
        let mut buf = Vec::new();
        host.export_facts(&mut buf, "compose@cell80")
            .map_err(|e| format!("facts export: {e}"))?;
        std::fs::write(path, &buf).map_err(|e| format!("{path}: {e}"))?;
    }
    Ok(if json {
        use serde_json::json;
        json!({
            "answer": answer,
            "agreement": agreement,
            "flagged": flagged,
            "battery": battery_note,
            "derivations": outcomes.iter().zip(&sources).map(|(o, s)| json!({
                "source": s,
                "answer": o.answer,
                "kill": o.kill,
                "artifact": o.artifact,
                "retrieved": o.retrieved,
                "cycles": o.cycles,
                "trapped_ops": o.trapped_ops,
                "resolutions": o.resolutions.iter()
                    .map(|(n, id)| json!({"call": n, "cell": id}))
                    .collect::<Vec<_>>(),
                "repairs": o.repairs,
            })).collect::<Vec<_>>(),
        })
        .to_string()
    } else {
        match answer {
            Some(a) => {
                let mut out = format!(
                    "answer: {a} ({agreement}{})",
                    if flagged { ", flagged for audit" } else { "" }
                );
                if let Some(b) = &battery_note {
                    out += &format!("\n  battery: {b}");
                }
                for (o, s) in outcomes.iter().zip(&sources) {
                    for (n, id) in &o.resolutions {
                        out += &format!("\n  {s}: `{n}` -> {id}");
                    }
                }
                out
            }
            None => {
                let kills: Vec<String> = outcomes
                    .iter()
                    .zip(&sources)
                    .filter_map(|(o, s)| o.kill.as_ref().map(|k| format!("{s}: {k}")))
                    .collect();
                {
                    let mut out = format!("escalate — no agreement\n{}", kills.join("\n"));
                    if let Some(b) = &battery_note {
                        out += &format!("\nbattery: {b}");
                    }
                    out
                }
            }
        }
    })
}

pub(super) fn cmd_solve(args: &[String]) -> Result<String, String> {
    let file = args.first().ok_or(USAGE)?;
    let mut cycles = DEFAULT_CYCLES;
    let mut json = false;
    let mut it = args[1..].iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--cycles" => {
                cycles = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .ok_or("--cycles needs a number")?
            }
            "--json" => json = true,
            other => return Err(format!("unknown flag `{other}`")),
        }
    }
    let text = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
    let plans = crate::plan::plans_from_json(&text)?;
    let mut host = CellHost::new();
    host.set_cache(true);
    let rep = host.solve(&plans, cycles)?;
    Ok(if json {
        rep.to_json()
    } else {
        match rep.answer {
            Some(a) => format!(
                "answer: {a}{}",
                if rep.battery_ran {
                    " (counterfactual battery decided)"
                } else {
                    ""
                }
            ),
            None => {
                let kills: Vec<String> = rep
                    .outcomes
                    .iter()
                    .enumerate()
                    .filter_map(|(i, o)| o.kill.as_ref().map(|k| format!("plan {i}: {k}")))
                    .collect();
                format!(
                    "no surviving consensus — escalate
{}",
                    kills.join(
                        "
"
                    )
                )
            }
        }
    })
}

/// `graph <graph.json> <dir> [--input k=v,...] [--cycles N] [--json]` — load the cell library
/// in `<dir>`, then validate + run the JSON `CellGraph` over it, printing the combined trace.
pub(super) fn cmd_graph(args: &[String]) -> Result<String, String> {
    let mut it = args.iter();
    let graph_file = it.next().ok_or(USAGE)?;
    let dir = it.next().ok_or(USAGE)?;
    let mut inputs = std::collections::HashMap::new();
    let mut cycles = DEFAULT_CYCLES;
    let mut json = false;
    while let Some(a) = it.next() {
        match a.as_str() {
            "--input" => {
                let spec = it.next().ok_or("--input needs `k=v,...`")?;
                for kv in spec.split(',').filter(|s| !s.is_empty()) {
                    let (k, v) = kv
                        .split_once('=')
                        .ok_or_else(|| format!("bad --input `{kv}` (want `k=v`)"))?;
                    let val = v
                        .trim()
                        .parse::<u64>()
                        .map_err(|_| format!("bad --input value `{v}`"))?;
                    inputs.insert(k.trim().to_string(), val);
                }
            }
            "--cycles" => {
                cycles = it
                    .next()
                    .ok_or("--cycles needs N")?
                    .parse()
                    .map_err(|_| "bad --cycles".to_string())?;
            }
            "--json" => json = true,
            other => return Err(format!("unknown option `{other}`\n{USAGE}")),
        }
    }
    let src = std::fs::read_to_string(graph_file).map_err(|e| format!("{graph_file}: {e}"))?;
    let graph = CellGraph::from_json(&src)?;
    let mut host = host_from_dir(dir)?;
    let run = graph.run(&mut host, &inputs, cycles)?;
    Ok(if json { run.to_json() } else { run.to_human() })
}
