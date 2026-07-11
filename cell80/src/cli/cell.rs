//! Single-cell verbs: `run` (source dev loop), `compile`, `exec` (precompiled
//! runtime loop), `inspect` — plus their shared run-and-format tail.
use super::*;

/// `compile <file.rs> -o <file.cell> [--entry] [--id] [--summary] [--tags] [safety]` —
/// compile source to a `.cell` cartridge on disk; print the inspection summary.
pub(super) fn cmd_compile(args: &[String]) -> Result<String, String> {
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
pub(super) fn cmd_inspect(args: &[String]) -> Result<String, String> {
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
pub(super) fn cmd_run(args: &[String]) -> Result<String, String> {
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
pub(super) fn cmd_exec(args: &[String]) -> Result<String, String> {
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
    let mut runner = Runner::new(cart.z80()?);
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
