//! CN-1 slice-0 pilot glue (`experiments/cell-native-architectures.md`): print each named
//! cell's `Fingerprint` (over `DEFAULT_PROBES`) as JSON, so a Python driver can project it
//! into an embedding-init vector without needing a `cell80-py` binding for `Fingerprint`
//! itself (which doesn't exist yet — see the pilot's own findings for why a subprocess call
//! is the pilot's stand-in, not the eventual shape). A thin CLI wrapper around an
//! already-public primitive, not new fingerprinting logic.
//!
//! Usage: `cargo run --release --example dump_fingerprints -- <cell_name> [<cell_name> ...]`
//! Prints one JSON object to stdout: `{"cell_name": [123, null, 45, ...], ...}` — one entry
//! per `DEFAULT_PROBES` row, in probe order; `null` is a probe that didn't return cleanly
//! (a trap/halt outcome — itself a distinguishing signal, per `Fingerprint`'s own doc
//! comment, not an error to hide).
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use cell80::{Cartridge, CartridgeOpts, CellConfig, Fingerprint, DEFAULT_PROBES};

fn cells_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("cells")
}

fn main() {
    let names: Vec<String> = env::args().skip(1).collect();
    if names.is_empty() {
        eprintln!("usage: dump_fingerprints <cell_name> [<cell_name> ...]");
        std::process::exit(1);
    }

    let mut out = String::from("{");
    for (i, name) in names.iter().enumerate() {
        let path = cell80::find_cell_file(&cells_dir(), name)
            .unwrap_or_else(|e| panic!("finding `{name}`: {e}"));
        let src = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        let cart = Cartridge::compile(
            &src,
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some(name.clone()),
                ..Default::default()
            },
        )
        .unwrap_or_else(|e| panic!("compiling `{name}`: {e}"));

        let fp = Fingerprint::compute(&cart, DEFAULT_PROBES, 100_000);
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!("{name:?}:["));
        for (j, v) in fp.outputs.iter().enumerate() {
            if j > 0 {
                out.push(',');
            }
            match v {
                Some(x) => out.push_str(&x.to_string()),
                None => out.push_str("null"),
            }
        }
        out.push(']');
    }
    out.push('}');
    println!("{out}");
}
