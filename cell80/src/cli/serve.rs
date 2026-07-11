//! The warm-session surface: `dispatch` (the transport-agnostic command core),
//! the `serve` stdio loop over it, and the `serve` verb.
use super::*;

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
pub(super) fn serve_loop(
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
pub(super) fn cmd_serve(args: &[String]) -> Result<String, String> {
    let dir = args.first().ok_or(USAGE)?;
    let mut host = host_from_dir(dir)?;
    serve_loop(
        &mut host,
        dir,
        std::io::stdin().lock(),
        std::io::stdout().lock(),
    )
}
