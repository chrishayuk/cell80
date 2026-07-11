//! Library-level discovery verbs — `index` (list / admission gate), `search`
//! (text + fused examples), `route` (pure behaviour) — and the warm-host builder
//! they all share.
use super::*;

/// `index <dir> [--gate <retrieval.jsonl>] [--json]` — list the cell library (in id order),
/// or, with `--gate`, run the admission gate (roadmap 2.2): admit each cell only if it's
/// behaviourally distinct from every already-admitted cell and carries retrieval-dataset
/// rows, refusing (with a report) the ones that don't.
pub(super) fn cmd_index(args: &[String]) -> Result<String, String> {
    let dir = args.first().ok_or(USAGE)?;
    let mut gate: Option<&str> = None;
    let mut json = false;
    let mut it = args[1..].iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--gate" => gate = Some(it.next().ok_or("--gate needs a retrieval.jsonl path")?),
            "--json" => json = true,
            other => return Err(format!("unknown option `{other}`\n{USAGE}")),
        }
    }
    if let Some(path) = gate {
        let report = crate::admission::admit(dir, std::path::Path::new(path))?;
        return Ok(if json {
            report.to_json()
        } else {
            render_admission(dir, &report)
        });
    }

    let paths = crate::discover::discover_cell_files(dir)?;
    let mut manifests = Vec::new();
    for path in paths {
        if let Some(c) = library_cartridge(&path) {
            let manifest = c?.manifest;
            // The pack a cell belongs to is its immediate parent directory name
            // (`cell80/cells/<pack>/<id>.rs`) — the directory *is* the pack now, so
            // nothing downstream needs a separately hand-maintained pack list.
            let pack = path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            manifests.push((pack, manifest));
        }
    }
    if json {
        use serde_json::json;
        let cells: Vec<_> = manifests
            .iter()
            .map(|(pack, m)| {
                json!({
                    "id": m.id, "summary": m.summary, "tags": m.tags,
                    "signature": m.signature.to_decl(&m.entry), "pack": pack,
                })
            })
            .collect();
        return Ok(json!({ "dir": dir, "cells": cells }).to_string());
    }
    let rows: Vec<String> = manifests.iter().map(|(_, m)| render(m)).collect();
    Ok(format!(
        "cell library `{dir}` ({} cells):\n{}",
        rows.len(),
        rows.join("\n")
    ))
}

/// Render an [`AdmissionReport`](crate::AdmissionReport): admitted cells exactly like plain
/// `index`, with a `REFUSED:` section appended only when non-empty.
fn render_admission(dir: &str, report: &crate::AdmissionReport) -> String {
    let rows: Vec<String> = report.admitted.iter().map(render).collect();
    let mut out = format!(
        "cell library `{dir}` admission ({} admitted, {} refused):\n{}",
        report.admitted.len(),
        report.refused.len(),
        rows.join("\n")
    );
    if !report.refused.is_empty() {
        out.push_str("\nREFUSED:\n");
        for (m, reasons) in &report.refused {
            for r in reasons {
                out.push_str(&format!("  {} — {}\n", m.id, render_reason(r)));
            }
        }
    }
    out
}

/// Human-readable rendering of one [`RefusalReason`](crate::RefusalReason).
fn render_reason(r: &crate::RefusalReason) -> String {
    use crate::RefusalReason::*;
    match r {
        BehaviouralDuplicate {
            of,
            agreement,
            colliding_queries,
        } => {
            let mut s = format!(
                "behavioural duplicate of `{of}` (agreement {agreement:.2}) — \
                 alias it on `{of}` in metadata instead of shipping new code \
                 (docs/library-growth.md: no behavioural duplicates)"
            );
            for (case_id, query, category) in colliding_queries {
                s += &format!(
                    "\n    query collision: {category} row `{case_id}` (\"{query}\") ranks `{of}` first"
                );
            }
            s
        }
        NoRetrievalRows => "no retrieval.jsonl rows — ships without a paraphrase/adversarial \
             query set (docs/library-growth.md: pay the eval tax per cell)"
            .to_string(),
    }
}

/// `search <query> <dir> [examples…]` — rank the library by relevance to `query`.
/// Trailing example tokens fuse **behaviour** into the ranking
/// ([`CellHost::search_with_examples`]): behaviour first, text order breaking ties —
/// the same-shape-sibling separator. Positional (`3,7=3`) and field
/// (`a:9,b:3=1,out:12`) forms, not mixed.
pub(super) fn cmd_search(args: &[String]) -> Result<String, String> {
    let query = args.first().ok_or(USAGE)?;
    let dir = args.get(1).ok_or("search needs a directory")?;
    let example_toks: Vec<&str> = args[2..].iter().map(String::as_str).collect();
    // Build a warm host so `search` uses the *same* TF-IDF index path as `serve`/MCP.
    let mut host = host_from_dir(dir)?;
    let n_examples = example_toks.len();
    // Captured before the fused call: its `&mut` borrow lives as long as `hits`.
    let n_cells = host.len();
    let hits = if example_toks.is_empty() {
        host.search(query, 10)
    } else {
        host.set_cache(true); // repeated probe runs memoize
        let field_form = example_toks.iter().any(|t| t.contains(':'));
        if field_form {
            let examples = parse_field_examples(&example_toks)?;
            host.search_with_field_examples(query, &examples, 10)?
        } else {
            let examples = parse_examples(&example_toks)?;
            host.search_with_examples(query, &examples, 10)?
        }
    };
    let examples_note = if n_examples > 0 {
        format!(" + {n_examples} example(s)")
    } else {
        String::new()
    };
    let mut out = format!(
        "indexed {n_cells} cells; query `{query}`{examples_note} → {} match(es):\n",
        hits.len()
    );
    for m in hits {
        out += &render(m);
        out.push('\n');
    }
    Ok(out)
}

/// `route <dir> <in,..>=<out> ... [--facts <file.facts>] [--json]` — rank the library by
/// **behaviour**: which cells reproduce the given input→output examples. The
/// phrasing-independent lookup that tells `min` from `max` where their manifests are
/// identical. With `--facts`, the file is imported first (spot-checked, exactly like
/// `facts import`) and matching claims answer probe runs without executing — the
/// provenance split is reported either way.
pub(super) fn cmd_route(args: &[String]) -> Result<String, String> {
    let dir = args.first().ok_or(USAGE)?;
    let mut facts_file: Option<&str> = None;
    let mut json = false;
    let mut example_toks: Vec<&str> = Vec::new();
    let mut it = args[1..].iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--facts" => facts_file = Some(it.next().ok_or("--facts needs a file")?),
            "--json" => json = true,
            tok if tok.starts_with("--") => return Err(format!("unknown option `{tok}`\n{USAGE}")),
            tok => example_toks.push(tok),
        }
    }
    // Named-field form (`x1:3,y1:4=11`) routes state cells — register probes
    // can't drive named state. All-or-nothing: mixing forms is an error.
    let field_form = example_toks.iter().any(|t| t.contains(':'));
    if field_form {
        if !example_toks.iter().all(|t| t.contains(':')) {
            return Err(
                "mixing positional (`3,7=3`) and field (`x:3=6`) examples — pick one form".into(),
            );
        }
        let examples: Vec<(Vec<(String, u64)>, u16)> = parse_field_examples(&example_toks)?
            .into_iter()
            .map(|ex| match (ex.want_result, ex.want_fields.is_empty()) {
                (Some(out), true) => Ok((ex.fields, out)),
                // `route_by_field_examples` matches on the return only; expected
                // post-run fields (`=out:12,..`) are a `search` example form.
                _ => Err(
                    "route matches the return only (`=out`); expected post-run fields \
                     (`=out:12,..`) are a `search` example form"
                        .to_string(),
                ),
            })
            .collect::<Result<_, String>>()?;
        let host = host_from_dir(dir)?;
        let hits = host.route_by_field_examples(&examples, 10);
        if hits.is_empty() {
            return Ok("no cell in the library reproduces those field examples".into());
        }
        return Ok(hits
            .iter()
            .map(|m| {
                format!(
                    "{} — {}  ({})",
                    m.id,
                    m.summary,
                    m.signature.to_decl(&m.entry)
                )
            })
            .collect::<Vec<_>>()
            .join("\n"));
    }
    let examples = parse_examples(&example_toks)?;
    if examples.is_empty() {
        return Err("route needs at least one example: <in,..>=<out>".into());
    }
    let mut host = host_from_dir(dir)?;
    host.set_cache(true); // imported facts stamp at load; probe runs memoize
    if let Some(file) = facts_file {
        let f = std::fs::File::open(file).map_err(|e| format!("{file}: {e}"))?;
        let rep = host.import_facts(std::io::BufReader::new(f), &crate::ImportPolicy::default())?;
        if rep.file_failed || !rep.failures.is_empty() {
            return Err(rep.to_human());
        }
    }
    let cells = host.len();
    let rep = host.route_by_examples_facts(&examples, 10)?;
    if json {
        use serde_json::json;
        let results: Vec<_> = rep
            .ranked
            .iter()
            .filter_map(|(hits, id)| {
                host.manifest(id).map(|m| {
                    json!({
                        "id": m.id, "summary": m.summary, "tags": m.tags,
                        "signature": m.signature.to_decl(&m.entry), "hits": hits,
                    })
                })
            })
            .collect();
        return Ok(json!({
            "dir": dir, "cells": cells, "examples": examples.len(), "results": results,
            "probe_runs": rep.probe_runs, "from_facts": rep.from_facts, "local": rep.local,
        })
        .to_string());
    }
    let mut out = format!(
        "routed {cells} cells on {} example(s) → {} match(es):\n",
        examples.len(),
        rep.ranked.len()
    );
    for (hits, id) in &rep.ranked {
        if let Some(m) = host.manifest(id) {
            out += &format!("{}  [{}/{}]\n", render(m), hits, examples.len());
        }
    }
    out += &format!(
        "probe runs: {} — {} answered from imported facts, {} computed locally",
        rep.probe_runs, rep.from_facts, rep.local
    );
    Ok(out)
}

/// Build a warm [`CellHost`] over every cell (`.rs` / `.cell`) under `dir`, discovered
/// recursively (cells live in pack subdirectories).
pub(super) fn host_from_dir(dir: &str) -> Result<CellHost, String> {
    let mut host = CellHost::new();
    for path in crate::discover::discover_cell_files(dir)? {
        if let Some(c) = library_cartridge(&path) {
            host.add(c?);
        }
    }
    Ok(host)
}
