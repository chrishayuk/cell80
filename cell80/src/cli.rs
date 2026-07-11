//! CLI for the `rustz80-cell` binary: the `USAGE` line, the `run_cli` dispatcher, and
//! one submodule per verb family — [`parse`] (token parsers), [`meta`] (cell-header
//! parsing + manifest rendering), [`library`] (index/search/route + the warm-host
//! builder), [`serve`] (the transport-agnostic session `dispatch` + stdio loop),
//! [`compose`] (compose/solve/graph), [`facts`], [`cell`] (run/compile/exec/inspect),
//! and [`keys`] (keygen/sign).
use super::*;

mod cell;
mod compose;
mod facts;
mod keys;
mod library;
mod meta;
mod parse;
mod serve;
#[cfg(test)]
mod tests;

// The crate-facing surface (`lib.rs` re-exports `parse_args`/`run_cli`/`USAGE`;
// `crate::admission` walks directories through `library_cartridge`; `dispatch`
// stays reachable as `cli::serve::dispatch` for an MCP server / socket daemon).
pub(crate) use meta::library_cartridge;
pub use parse::parse_args;

// Shared vocabulary for the submodules: each starts with `use super::*;`, so the
// imports below are what lets a verb in one family call a helper from another
// (e.g. `cmd_search` → `host_from_dir` + `render` + the example parsers).
use cell::{cmd_compile, cmd_exec, cmd_inspect, cmd_run};
use compose::{cmd_compose, cmd_graph, cmd_solve};
use facts::cmd_facts;
use keys::{cmd_keygen, cmd_sign};
use library::{cmd_index, cmd_route, cmd_search, host_from_dir};
use meta::render;
use parse::{parse_examples, parse_field_examples, parse_reads, parse_sets};
use serve::cmd_serve;

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
     rustz80-cell search <query> <dir> [<in,..>=<out> | <f:v,..>=<out>[,f:v..] ...]\n  \
     \x20                                        (rank by relevance; examples fuse behaviour into the ranking)\n  \
     rustz80-cell route <dir> <in,..>=<out> [more examples] [--facts <file.facts>] [--json]\n  \
     \x20                                        (rank cells by BEHAVIOUR; facts answer probes without executing)\n  \
     rustz80-cell serve <dir>                 (persistent stdio session over a warm host)\n  \
     rustz80-cell graph <graph.json> <dir> [--input k=v,...] [--cycles N] [--json]\n  \
     rustz80-cell facts export <dir> --calls <file> [--producer P]  (run calls, print .facts)\n  \
     rustz80-cell facts import <file.facts> <dir> [--verify-fraction F] [--quarantine] [--json]\n  \
     rustz80-cell facts verify <file.facts> <dir> [--json]  (re-execute every line; CI-able)\n  \
     rustz80-cell solve <plans.json> [--cycles N] [--json]  (render/compile/verify candidate plans)\n  \
     rustz80-cell compose <dir> <src.rs> [<src2.rs> ...] [--args a,b,..] [--cycles N] [--facts <file>] [--dump-canon] [--json]\n  \
     \x20                                        (canonicalize + link against the library + run; N sources = the agreement gate)\n  \
     safety (sandboxed by default): [--allow-raw-memory] [--allow-ports] \
     [--max-code-bytes N] [--max-touched N]";

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
        Some("route") => cmd_route(&args[1..]),
        Some("serve") => cmd_serve(&args[1..]),
        Some("graph") => cmd_graph(&args[1..]),
        Some("facts") => cmd_facts(&args[1..]),
        Some("solve") => cmd_solve(&args[1..]),
        Some("compose") => cmd_compose(&args[1..]),
        Some("keygen") => cmd_keygen(&args[1..]),
        Some("sign") => cmd_sign(&args[1..]),
        Some(other) => Err(format!("unknown command `{other}`\n{USAGE}")),
        None => Err(USAGE.into()),
    }
}
