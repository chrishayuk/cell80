//! CLI for the `rustz80-cell` binary: argument parsing + `run_cli`.
use super::*;

/// CLI usage line, shared by the `rustz80-cell` binary.
pub const USAGE: &str = "usage:\n  \
     rustz80-cell run <file.rs> [--entry NAME] [--cycles N] [--args a,b,c] \
     [--set addr:ty=val,...] [--read name@addr:ty,...] [--json]\n  \
     rustz80-cell compile <file.rs> -o <file.cell> [--entry NAME] [--id ID] \
     [--summary TEXT] [--tags a,b] [safety]\n  \
     rustz80-cell exec <file.cell> [--entry NAME] [--cycles N] [--args a,b,c] \
     [--set addr:ty=val,...] [--read name@addr:ty,...] [--json] [--no-verify]\n  \
     rustz80-cell inspect <file.cell> [--json] [--no-verify]\n  \
     rustz80-cell keygen <out.key>             (new ed25519 signing key)\n  \
     rustz80-cell sign <file.cell> --key <key> (sign the artifact hash in place)\n  \
     rustz80-cell index <dir> [--gate <retrieval.jsonl>] [--json]  (list, or admit/refuse)\n  \
     rustz80-cell search <query> <dir>        (rank library cells by relevance)\n  \
     rustz80-cell serve <dir>                 (persistent stdio session over a warm host)\n  \
     rustz80-cell graph <graph.json> <dir> [--input k=v,...] [--cycles N] [--json]\n  \
     safety (sandboxed by default): [--allow-raw-memory] [--allow-ports] \
     [--max-code-bytes N] [--max-touched N]";

/// Parse a comma-separated arg list — decimal or `0x`-prefixed hex, each a `u16`.
pub fn parse_args(s: &str) -> Result<Vec<u16>, String> {
    s.split(',')
        .filter(|t| !t.trim().is_empty())
        .map(|t| {
            let t = t.trim();
            let v = match t.strip_prefix("0x") {
                Some(h) => u16::from_str_radix(h, 16),
                None => t.parse::<u16>(),
            };
            v.map_err(|_| format!("bad arg `{t}` (want a u16, decimal or 0x..)"))
        })
        .collect()
}

/// Parse `route` example tokens like `"3,7=7"` into `(inputs, expected_output)` pairs.
fn parse_examples(toks: &[&str]) -> Result<Vec<(Vec<u16>, u16)>, String> {
    toks.iter()
        .map(|t| {
            let (lhs, rhs) = t
                .split_once('=')
                .ok_or_else(|| format!("bad example `{t}` (want in,..=out)"))?;
            let inputs = parse_args(lhs)?;
            let want = parse_args(rhs)?;
            match want.as_slice() {
                [out] => Ok((inputs, *out)),
                _ => Err(format!("bad example `{t}` (one output after `=`)")),
            }
        })
        .collect()
}

/// Parse a `--set` spec — comma-separated `addr:ty=value` (addr/value decimal or `0x..`),
/// the typed inputs written into memory before the run.
fn parse_sets(s: &str) -> Result<Vec<(u16, Ty, u64)>, String> {
    let num16 = |t: &str| match t.strip_prefix("0x") {
        Some(h) => u16::from_str_radix(h, 16),
        None => t.parse::<u16>(),
    };
    let num64 = |t: &str| match t.strip_prefix("0x") {
        Some(h) => u64::from_str_radix(h, 16),
        None => t.parse::<u64>(),
    };
    s.split(',')
        .filter(|t| !t.trim().is_empty())
        .map(|t| {
            let t = t.trim();
            let bad = || format!("bad --set `{t}` (want addr:ty=value)");
            let (lhs, val_s) = t.split_once('=').ok_or_else(bad)?;
            let (addr_s, ty_s) = lhs.split_once(':').ok_or_else(bad)?;
            let addr = num16(addr_s).map_err(|_| format!("bad address in `{t}`"))?;
            let val = num64(val_s).map_err(|_| format!("bad value in `{t}`"))?;
            Ok((addr, Ty::parse(ty_s)?, val))
        })
        .collect()
}

/// Parse a `--read` spec — comma-separated `name@addr:ty` (addr decimal or `0x..`).
fn parse_reads(s: &str) -> Result<Vec<(String, u16, Ty)>, String> {
    s.split(',')
        .filter(|t| !t.trim().is_empty())
        .map(|t| {
            let t = t.trim();
            let bad = || format!("bad --read `{t}` (want name@addr:ty)");
            let (name, rest) = t.split_once('@').ok_or_else(bad)?;
            let (addr_s, ty_s) = rest.split_once(':').ok_or_else(bad)?;
            let addr = match addr_s.strip_prefix("0x") {
                Some(h) => u16::from_str_radix(h, 16),
                None => addr_s.parse::<u16>(),
            }
            .map_err(|_| format!("bad address in `{t}`"))?;
            Ok((name.to_string(), addr, Ty::parse(ty_s)?))
        })
        .collect()
}

/// Dispatch `run` / `compile` / `inspect` and return the formatted output. The
/// `rustz80-cell` binary is a shim over this.
pub fn run_cli(args: &[String]) -> Result<String, String> {
    match args.first().map(String::as_str) {
        Some("run") => cmd_run(&args[1..]),
        Some("compile") => cmd_compile(&args[1..]),
        Some("exec") => cmd_exec(&args[1..]),
        Some("inspect") => cmd_inspect(&args[1..]),
        Some("index") => cmd_index(&args[1..]),
        Some("search") => cmd_search(&args[1..]),
        Some("serve") => cmd_serve(&args[1..]),
        Some("graph") => cmd_graph(&args[1..]),
        Some("keygen") => cmd_keygen(&args[1..]),
        Some("sign") => cmd_sign(&args[1..]),
        Some(other) => Err(format!("unknown command `{other}`\n{USAGE}")),
        None => Err(USAGE.into()),
    }
}

/// Parse a cell source's leading `//!` header → `(summary, tags, entry, limits)`.
fn parse_meta(src: &str) -> (String, Vec<String>, Option<String>, Vec<String>) {
    let (mut summary, mut tags, mut entry, mut limits) =
        (String::new(), Vec::new(), None, Vec::new());
    let csv = |s: &str| -> Vec<String> {
        s.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };
    for line in src.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("//!") {
            let rest = rest.trim();
            if let Some(t) = rest.strip_prefix("tags:") {
                tags = csv(t);
            } else if let Some(e) = rest.strip_prefix("entry:") {
                entry = Some(e.trim().to_string());
            } else if let Some(m) = rest.strip_prefix("limits:") {
                // The escalation contract, authorable from the source header — what this
                // cell can't do (`//! limits: floats, inputs > 65535`).
                limits = csv(m);
            } else if summary.is_empty() {
                summary = rest.to_string();
            }
        } else if !l.is_empty() && !l.starts_with("//") {
            break; // first code line — header done
        }
    }
    (summary, tags, entry, limits)
}

/// Build a cartridge from a library `.rs` (id = file stem, metadata from the `//!` header)
/// or load a `.cell`. Returns `None` for any other extension. `pub(crate)` so the admission
/// gate (`crate::admission`) walks a directory the same way `cmd_index`/`host_from_dir` do.
pub(crate) fn library_cartridge(path: &std::path::Path) -> Option<Result<Cartridge, String>> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => Some((|| {
            let src =
                std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
            let (summary, tags, entry, limits) = parse_meta(&src);
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("cell")
                .to_string();
            Cartridge::compile(
                &src,
                CellConfig::sandboxed(),
                CartridgeOpts {
                    id: Some(id),
                    entry,
                    summary,
                    tags,
                    limits,
                },
            )
        })()),
        Some("cell") => Some(
            std::fs::read(path)
                .map_err(|e| format!("{}: {e}", path.display()))
                .and_then(|b| Cartridge::from_bytes(&b)),
        ),
        _ => None,
    }
}

/// Format a manifest as one library/search-result line: `id — summary  [tags]  (signature)`.
fn render(m: &crate::Manifest) -> String {
    format!(
        "  {} — {}  [{}]  ({})",
        m.id,
        if m.summary.is_empty() {
            "(no summary)"
        } else {
            &m.summary
        },
        m.tags.join(", "),
        m.signature.to_decl(&m.entry),
    )
}

/// `index <dir> [--gate <retrieval.jsonl>] [--json]` — list the cell library (in id order),
/// or, with `--gate`, run the admission gate (roadmap 2.2): admit each cell only if it's
/// behaviourally distinct from every already-admitted cell and carries retrieval-dataset
/// rows, refusing (with a report) the ones that don't.
fn cmd_index(args: &[String]) -> Result<String, String> {
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

    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{dir}: {e}"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    paths.sort();
    let mut manifests = Vec::new();
    for path in paths {
        if let Some(c) = library_cartridge(&path) {
            manifests.push(c?.manifest);
        }
    }
    if json {
        use serde_json::json;
        let cells: Vec<_> = manifests
            .iter()
            .map(|m| {
                json!({
                    "id": m.id, "summary": m.summary, "tags": m.tags,
                    "signature": m.signature.to_decl(&m.entry),
                })
            })
            .collect();
        return Ok(json!({ "dir": dir, "cells": cells }).to_string());
    }
    let rows: Vec<String> = manifests.iter().map(render).collect();
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

/// `search <query> <dir>` — rank the library by relevance to `query`.
fn cmd_search(args: &[String]) -> Result<String, String> {
    let query = args.first().ok_or(USAGE)?;
    let dir = args.get(1).ok_or("search needs a directory")?;
    // Build a warm host so `search` uses the *same* TF-IDF index path as `serve`/MCP.
    let host = host_from_dir(dir)?;
    let hits = host.search(query, 10);
    let mut out = format!(
        "indexed {} cells; query `{query}` → {} match(es):\n",
        host.len(),
        hits.len()
    );
    for m in hits {
        out += &render(m);
        out.push('\n');
    }
    Ok(out)
}

/// Build a warm [`CellHost`] over every cell (`.rs` / `.cell`) in `dir`.
fn host_from_dir(dir: &str) -> Result<CellHost, String> {
    let mut host = CellHost::new();
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{dir}: {e}"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    paths.sort();
    for path in paths {
        if let Some(c) = library_cartridge(&path) {
            host.add(c?);
        }
    }
    Ok(host)
}

/// Run one session command against a warm host, returning the response line. This is the
/// transport-agnostic core of the session: a `serve` stdio loop, an MCP server, or a socket
/// daemon all funnel commands through here.
pub(crate) fn dispatch(host: &mut CellHost, line: &str) -> String {
    let mut it = line.split_whitespace();
    match it.next() {
        None => String::new(),
        Some("help") => {
            "commands: search <query> | route <in,..>=<out> ... | inspect <id> | load <id> | \
             run <handle> [a,b,c] | unload <handle> | help"
                .into()
        }
        Some("search") => {
            let q = it.collect::<Vec<_>>().join(" ");
            let hits = host.search(&q, 10);
            if hits.is_empty() {
                format!("no matches for `{q}`")
            } else {
                hits.iter()
                    .map(|m| render(m))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
        // Discover by *behaviour*: `route 3,7=7 10,3=10` finds the cell(s) reproducing those
        // input→output examples — the phrasing-independent signal that tells `min` from `max`.
        Some("route") => {
            let toks: Vec<&str> = it.collect();
            match parse_examples(&toks) {
                Err(e) => e,
                Ok(ex) if ex.is_empty() => "usage: route <in,..>=<out> [<in,..>=<out> ...]".into(),
                Ok(ex) => {
                    let hits = host.route_by_examples(&ex, 10);
                    if hits.is_empty() {
                        "no cell in the library reproduces those examples".into()
                    } else {
                        hits.iter()
                            .map(|m| render(m))
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                }
            }
        }
        Some("inspect") => match it.next() {
            Some(id) => host
                .manifest(id)
                .map(render)
                .unwrap_or_else(|| format!("no cell `{id}`")),
            None => "usage: inspect <id>".into(),
        },
        Some("load") => match it.next() {
            Some(id) => match host.load(id) {
                Ok(h) => format!("loaded `{id}` → handle {h}"),
                Err(e) => e,
            },
            None => "usage: load <id>".into(),
        },
        Some("run") => {
            let handle = match it.next().and_then(|s| s.parse::<usize>().ok()) {
                Some(h) => h,
                None => return "usage: run <handle> [a,b,c]".into(),
            };
            let args = match it.next() {
                Some(a) => match parse_args(a) {
                    Ok(v) => v,
                    Err(e) => return e,
                },
                None => Vec::new(),
            };
            match host.run_fast(handle, &args, DEFAULT_CYCLES) {
                Ok(f) => format!(
                    "result {} regs [{}, {}, {}] cycles {} trapped_ops {} halt {:?}",
                    f.result, f.regs[0], f.regs[1], f.regs[2], f.cycles, f.trapped_ops, f.halt
                ),
                Err(e) => e,
            }
        }
        Some("unload") => match it.next().and_then(|s| s.parse::<usize>().ok()) {
            Some(h) => match host.unload(h) {
                Ok(()) => format!("unloaded handle {h}"),
                Err(e) => e,
            },
            None => "usage: unload <handle>".into(),
        },
        Some(other) => format!("unknown command `{other}` (try `help`)"),
    }
}

/// The `serve` read/respond loop — split out from [`cmd_serve`] over generic I/O so it can
/// be driven by a test without a real stdin/stdout. The host stays warm across commands.
fn serve_loop(
    host: &mut CellHost,
    dir: &str,
    input: impl std::io::BufRead,
    mut out: impl std::io::Write,
) -> Result<String, String> {
    writeln!(
        out,
        "rustz80-cell session: {} cells from `{dir}`. \
         commands: search/route/inspect/load/run/unload/help; `quit` or ^D to end.",
        host.len()
    )
    .map_err(|e| e.to_string())?;
    out.flush().ok();
    for line in input.lines() {
        let line = line.map_err(|e| e.to_string())?;
        let t = line.trim();
        if t == "quit" || t == "exit" {
            break;
        }
        writeln!(out, "{}", dispatch(host, &line)).map_err(|e| e.to_string())?;
        out.flush().ok();
    }
    Ok(format!(
        "session ended ({} cells, {} still loaded)",
        host.len(),
        host.live_count()
    ))
}

/// `serve <dir>` — a persistent stdio session: load the library once into a warm host, then
/// read one command per line and respond, keeping the index + runners warm across commands
/// (the warm-path a per-invocation CLI can't give). The bare-stdio front; an MCP server or
/// socket daemon would wrap the same [`dispatch`].
fn cmd_serve(args: &[String]) -> Result<String, String> {
    let dir = args.first().ok_or(USAGE)?;
    let mut host = host_from_dir(dir)?;
    serve_loop(
        &mut host,
        dir,
        std::io::stdin().lock(),
        std::io::stdout().lock(),
    )
}

/// `graph <graph.json> <dir> [--input k=v,...] [--cycles N] [--json]` — load the cell library
/// in `<dir>`, then validate + run the JSON `CellGraph` over it, printing the combined trace.
fn cmd_graph(args: &[String]) -> Result<String, String> {
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

/// `compile <file.rs> -o <file.cell> [--entry] [--id] [--summary] [--tags] [safety]` —
/// compile source to a `.cell` cartridge on disk; print the inspection summary.
fn cmd_compile(args: &[String]) -> Result<String, String> {
    let mut it = args.iter();
    let file = it.next().ok_or(USAGE)?;
    let mut out: Option<String> = None;
    let mut opts = CartridgeOpts::default();
    let mut cfg = CellConfig::sandboxed();
    let num = |o: Option<&String>, what: &str| -> Result<usize, String> {
        o.ok_or_else(|| format!("{what} needs a number"))?
            .parse()
            .map_err(|_| format!("bad {what}"))
    };
    while let Some(a) = it.next() {
        match a.as_str() {
            "-o" | "--out" => out = Some(it.next().ok_or("-o needs a path")?.clone()),
            "--entry" => opts.entry = Some(it.next().ok_or("--entry needs a name")?.clone()),
            "--id" => opts.id = Some(it.next().ok_or("--id needs a value")?.clone()),
            "--summary" => opts.summary = it.next().ok_or("--summary needs text")?.clone(),
            "--tags" => {
                opts.tags = it
                    .next()
                    .ok_or("--tags needs a list")?
                    .split(',')
                    .filter(|t| !t.trim().is_empty())
                    .map(|t| t.trim().to_string())
                    .collect()
            }
            "--allow-raw-memory" => cfg.allow_raw_memory = true,
            "--allow-ports" => cfg.allow_ports = true,
            "--max-code-bytes" => cfg.max_code_bytes = Some(num(it.next(), "--max-code-bytes")?),
            "--max-touched" => cfg.max_touched = Some(num(it.next(), "--max-touched")?),
            other => return Err(format!("unknown option `{other}`\n{USAGE}")),
        }
    }
    let out = out.ok_or("compile needs an output path: -o <file.cell>")?;
    let src = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
    let cart = Cartridge::compile(&src, cfg, opts)?;
    std::fs::write(&out, cart.to_bytes()).map_err(|e| format!("{out}: {e}"))?;
    Ok(format!("wrote {out}\n{}", cart.to_human()))
}

/// `inspect <file.cell> [--json]` — load a cartridge and print its manifest/symbols/caps.
fn cmd_inspect(args: &[String]) -> Result<String, String> {
    let mut it = args.iter();
    let file = it.next().ok_or(USAGE)?;
    let rest: Vec<&String> = it.collect();
    let json = rest.iter().any(|a| *a == "--json");
    let bytes = std::fs::read(file).map_err(|e| format!("{file}: {e}"))?;
    let cart = if rest.iter().any(|a| *a == "--no-verify") {
        Cartridge::from_bytes_unverified(&bytes)?
    } else {
        Cartridge::from_bytes(&bytes)?
    };
    Ok(if json {
        cart.to_json()
    } else {
        cart.to_human()
    })
}

/// `run <file.rs> [opts]` — compile source and run it, returning the report (JSON if
/// `--json`, else the human summary).
fn cmd_run(args: &[String]) -> Result<String, String> {
    let mut it = args.iter();
    let file = it.next().ok_or(USAGE)?;
    let mut entry: Option<String> = None;
    let mut cycles = DEFAULT_CYCLES;
    let mut call_args: Vec<u16> = Vec::new();
    let mut sets: Vec<(u16, Ty, u64)> = Vec::new();
    let mut reads: Vec<(String, u16, Ty)> = Vec::new();
    let mut json = false;
    let mut cfg = CellConfig::sandboxed(); // safe by default on the CLI
    let num = |o: Option<&String>, what: &str| -> Result<usize, String> {
        o.ok_or_else(|| format!("{what} needs a number"))?
            .parse()
            .map_err(|_| format!("bad {what}"))
    };
    while let Some(a) = it.next() {
        match a.as_str() {
            "--entry" => entry = Some(it.next().ok_or("--entry needs a name")?.clone()),
            "--cycles" => {
                cycles = it
                    .next()
                    .ok_or("--cycles needs a number")?
                    .parse()
                    .map_err(|_| "bad --cycles (want a positive integer)")?
            }
            "--args" => call_args = parse_args(it.next().ok_or("--args needs values")?)?,
            "--set" => sets = parse_sets(it.next().ok_or("--set needs a spec")?)?,
            "--read" => reads = parse_reads(it.next().ok_or("--read needs a spec")?)?,
            "--allow-raw-memory" => cfg.allow_raw_memory = true,
            "--allow-ports" => cfg.allow_ports = true,
            "--max-code-bytes" => cfg.max_code_bytes = Some(num(it.next(), "--max-code-bytes")?),
            "--max-touched" => cfg.max_touched = Some(num(it.next(), "--max-touched")?),
            "--json" => json = true,
            other => return Err(format!("unknown option `{other}`\n{USAGE}")),
        }
    }
    let src = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
    let mut runner = Runner::compile_with_config(&src, cfg)?;
    run_and_format(
        &mut runner,
        entry.as_deref(),
        &call_args,
        &sets,
        &reads,
        cycles,
        json,
    )
}

/// Run `entry` on a ready `Runner`, decode any `--read` fields, and format the report —
/// the shared tail of `run` (from source) and `exec` (from a `.cell`).
fn run_and_format(
    runner: &mut Runner,
    entry: Option<&str>,
    args: &[u16],
    sets: &[(u16, Ty, u64)],
    reads: &[(String, u16, Ty)],
    cycles: u64,
    json: bool,
) -> Result<String, String> {
    let mut report = runner.run_with_inputs(entry, args, sets, cycles)?;
    if !reads.is_empty() {
        report.reads = runner.read_named(reads); // decode typed fields from post-run memory
    }
    Ok(if json {
        report.to_json()
    } else {
        report.to_human()
    })
}

/// `exec <file.cell> [--entry] [--cycles] [--args] [--set] [--read] [--json]` — run a
/// **precompiled** cartridge (no recompile); the entry defaults to the manifest's. This is
/// the runtime/registry loop (vs `run`, the source dev loop). The cartridge carries its own
/// capability policy, so there are no safety flags here.
fn cmd_exec(args: &[String]) -> Result<String, String> {
    let mut it = args.iter();
    let file = it.next().ok_or(USAGE)?;
    let mut entry: Option<String> = None;
    let mut cycles = DEFAULT_CYCLES;
    let mut call_args: Vec<u16> = Vec::new();
    let mut sets: Vec<(u16, Ty, u64)> = Vec::new();
    let mut reads: Vec<(String, u16, Ty)> = Vec::new();
    let mut json = false;
    let mut no_verify = false;
    while let Some(a) = it.next() {
        match a.as_str() {
            "--entry" => entry = Some(it.next().ok_or("--entry needs a name")?.clone()),
            "--cycles" => {
                cycles = it
                    .next()
                    .ok_or("--cycles needs a number")?
                    .parse()
                    .map_err(|_| "bad --cycles (want a positive integer)")?
            }
            "--args" => call_args = parse_args(it.next().ok_or("--args needs values")?)?,
            "--set" => sets = parse_sets(it.next().ok_or("--set needs a spec")?)?,
            "--read" => reads = parse_reads(it.next().ok_or("--read needs a spec")?)?,
            "--json" => json = true,
            "--no-verify" => no_verify = true, // dev-only: skip the artifact hash check
            other => return Err(format!("unknown option `{other}`\n{USAGE}")),
        }
    }
    let bytes = std::fs::read(file).map_err(|e| format!("{file}: {e}"))?;
    let cart = if no_verify {
        Cartridge::from_bytes_unverified(&bytes)?
    } else {
        Cartridge::from_bytes(&bytes)?
    };
    // Default to the cartridge's own entry (the manifest knows it).
    let entry = entry.unwrap_or_else(|| cart.manifest.entry.clone());
    let mut runner = Runner::new(&cart.program);
    run_and_format(
        &mut runner,
        Some(&entry),
        &call_args,
        &sets,
        &reads,
        cycles,
        json,
    )
}

/// `keygen <out.key>` — write a fresh 32-byte ed25519 seed (from the OS entropy pool)
/// and print the public verifying key. Guard the file like any private key.
fn cmd_keygen(args: &[String]) -> Result<String, String> {
    let out = args.first().ok_or(USAGE)?;
    use std::io::Read;
    let mut seed = [0u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut seed))
        .map_err(|e| format!("no OS entropy source (/dev/urandom): {e}"))?;
    std::fs::write(out, seed).map_err(|e| format!("{out}: {e}"))?;
    let vk = ed25519_dalek::SigningKey::from_bytes(&seed).verifying_key();
    let hex: String = vk.to_bytes().iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!(
        "wrote signing key to {out} (keep it private)\npublic key: ed25519:{hex}"
    ))
}

/// `sign <file.cell> --key <key>` — sign the artifact hash and rewrite the `.cell` with
/// the signature block embedded. Verifiers check it on every load.
fn cmd_sign(args: &[String]) -> Result<String, String> {
    let mut it = args.iter();
    let file = it.next().ok_or(USAGE)?;
    let mut key_path: Option<&String> = None;
    while let Some(a) = it.next() {
        match a.as_str() {
            "--key" => key_path = Some(it.next().ok_or("--key needs a path")?),
            other => return Err(format!("unknown option `{other}`\n{USAGE}")),
        }
    }
    let key_path = key_path.ok_or("sign needs --key <key file>")?;
    let key = std::fs::read(key_path).map_err(|e| format!("{key_path}: {e}"))?;
    let seed: [u8; 32] = key
        .as_slice()
        .try_into()
        .map_err(|_| format!("{key_path}: want exactly 32 bytes (from `keygen`)"))?;
    let bytes = std::fs::read(file).map_err(|e| format!("{file}: {e}"))?;
    let mut cart = Cartridge::from_bytes(&bytes)?; // verified before signing
    cart.sign(&seed);
    std::fs::write(file, cart.to_bytes()).map_err(|e| format!("{file}: {e}"))?;
    let vk = cart.signature.expect("just signed").0;
    let hex: String = vk.iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!("signed {file} (key ed25519:{}…)", &hex[..16]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Cartridge, CartridgeOpts, CellConfig};

    fn host() -> CellHost {
        let mut h = CellHost::new();
        h.add(
            Cartridge::compile(
                "fn run(a: u16, b: u16) -> u16 { a * b }",
                CellConfig::sandboxed(),
                CartridgeOpts {
                    id: Some("mul".into()),
                    summary: "multiply two".into(),
                    tags: vec!["math".into(), "product".into()],
                    entry: None,
                    limits: Vec::new(),
                },
            )
            .unwrap(),
        );
        h
    }

    #[test]
    fn dispatch_covers_every_verb() {
        let mut h = host();
        // discovery.
        assert!(dispatch(&mut h, "search multiply").contains("mul"));
        assert!(dispatch(&mut h, "search zzznotfound").starts_with("no matches"));
        assert!(dispatch(&mut h, "inspect mul").contains("run(a: u16, b: u16)"));
        assert!(dispatch(&mut h, "inspect ghost").contains("no cell"));
        assert!(dispatch(&mut h, "inspect").starts_with("usage"));
        // load → run warm (incl. no-args) → run again → unload.
        assert!(dispatch(&mut h, "load mul").contains("handle 0"));
        assert!(dispatch(&mut h, "load nope").contains("no cell"));
        assert!(dispatch(&mut h, "load").starts_with("usage"));
        assert!(dispatch(&mut h, "run 0 6,7").contains("result 42"));
        assert!(dispatch(&mut h, "run 0 3,3").contains("result 9")); // reused warm
        assert!(dispatch(&mut h, "run 0").contains("result 0")); // no args
        assert!(dispatch(&mut h, "run 0 99999999").contains("bad arg")); // parse error
        assert!(dispatch(&mut h, "run notanum").starts_with("usage"));
        assert!(dispatch(&mut h, "unload 0").contains("unloaded"));
        assert!(dispatch(&mut h, "run 0 1,1").contains("invalid cell handle")); // freed
        assert!(dispatch(&mut h, "unload 0").contains("invalid cell handle"));
        assert!(dispatch(&mut h, "unload x").starts_with("usage"));
        // misc.
        assert!(dispatch(&mut h, "help").contains("search"));
        assert!(dispatch(&mut h, "").is_empty());
        assert!(dispatch(&mut h, "bogus").contains("unknown command"));
    }

    #[test]
    fn serve_loop_runs_a_warm_session() {
        let mut h = host();
        let input =
            std::io::Cursor::new("load mul\nrun 0 6,7\nunload 0\nquit\nignored after quit\n");
        let mut out: Vec<u8> = Vec::new();
        let summary = serve_loop(&mut h, "test", input, &mut out).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("session:") && s.contains("handle 0"));
        assert!(s.contains("result 42") && s.contains("unloaded"));
        assert!(!s.contains("ignored after quit")); // loop stopped at `quit`
        assert!(summary.contains("session ended"));
    }

    #[test]
    fn host_from_dir_loads_the_seed_library() {
        let dir = format!("{}/cells", env!("CARGO_MANIFEST_DIR"));
        let h = host_from_dir(&dir).unwrap();
        assert_eq!(h.len(), 149); // 120 (Phase 2.3 pilot batch) + 8 checked-arithmetic + 6 money/bps + 4 units + 4 verifier/ranker + 3 stateful/RNG + 4 signed-deltas

        // The library now holds a *distance family* (manhattan/chebyshev/euclid_sq), so a
        // bare "grid distance" is ambiguous; the cell-specific name still resolves.
        assert_eq!(h.search("manhattan distance", 3)[0].id, "manhattan");
        assert!(host_from_dir("/no/such/dir").is_err());
    }
}
