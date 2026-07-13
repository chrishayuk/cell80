//! CN-1 real-build glue (`experiments/cell-native-architectures-cn1-preregistration.md`):
//! walk the whole cell library and print, per cell, the stable identity and behavioural
//! signature the build needs — one JSONL object per line, in `discover_cell_files` (sorted)
//! order so the output is deterministic and diffable.
//!
//! This single artifact feeds two pre-registered steps at once:
//!   - **step 3** (axis-A held-out draw) needs `name` + `pack` (stratification stratum) +
//!     `family_hash` (the identity-grade SHA-256 over canonical source — `source_hash` is
//!     explicitly non-cryptographic and is *not* recorded as identity);
//!   - **step 2c / step 4** (the `W_f` fingerprint projection) needs `fingerprint` for every
//!     cell, seen and held-out alike.
//!
//! Usage: `cargo run --release --example dump_library`  (walks `cell80/cells`)
//!        `cargo run --release --example dump_library -- <cells_dir>`
//! Cells that fail to compile are reported to stderr with a running count and skipped (they
//! can't become tokens either way); a nonzero skip count is printed to stderr at the end so a
//! silent drop can't masquerade as full coverage.
use std::env;
use std::path::{Path, PathBuf};

use cell80::{discover_cell_files, library_cartridge, Fingerprint, DEFAULT_PROBES};

fn default_cells_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("cells")
}

/// The pack is the cell's immediate parent directory name (`cells/<pack>/<name>.rs`) —
/// the stratification stratum for the axis-A draw. Falls back to `"_root"` for any cell
/// sitting directly under the cells dir.
fn pack_of(path: &Path, cells_dir: &Path) -> String {
    path.parent()
        .filter(|p| *p != cells_dir)
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "_root".to_string())
}

fn hex32(bytes: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn json_str(s: &str) -> String {
    // Cell names / pack names are simple identifiers, but escape defensively anyway.
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

fn main() {
    let cells_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(default_cells_dir);

    let paths = discover_cell_files(&cells_dir.to_string_lossy())
        .unwrap_or_else(|e| panic!("discovering cells under {}: {e}", cells_dir.display()));

    let mut ok = 0usize;
    let mut skipped = 0usize;
    for path in &paths {
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        // `library_cartridge` is the exact CLI/admission compile path: it parses the cell's
        // `//!` metadata (entry included), so state cells and non-`run`/`main` entries — 541
        // of the 790 with the naive auto-detect — resolve correctly.
        let cart = match library_cartridge(path) {
            Some(Ok(c)) => c,
            Some(Err(e)) => {
                eprintln!("skip {name}: compile: {e}");
                skipped += 1;
                continue;
            }
            None => {
                eprintln!("skip {name}: not a cell file");
                skipped += 1;
                continue;
            }
        };

        let m = &cart.manifest;
        let family_hash = m
            .family_hash
            .as_ref()
            .map(hex32)
            .unwrap_or_else(|| "null".to_string());
        let arity = m.signature.params.len();
        let pack = pack_of(path, &cells_dir);

        let fp = Fingerprint::compute(&cart, DEFAULT_PROBES, 100_000);
        let mut fp_json = String::from("[");
        for (j, v) in fp.outputs.iter().enumerate() {
            if j > 0 {
                fp_json.push(',');
            }
            match v {
                Some(x) => fp_json.push_str(&x.to_string()),
                None => fp_json.push_str("null"),
            }
        }
        fp_json.push(']');

        println!(
            "{{\"name\":{},\"pack\":{},\"family_hash\":{},\"source_hash\":\"0x{:016x}\",\"arity\":{},\"ret\":{},\"fingerprint\":{}}}",
            json_str(&name),
            json_str(&pack),
            json_str(&family_hash),
            m.source_hash,
            arity,
            json_str(&m.signature.ret),
            fp_json,
        );
        ok += 1;
    }
    eprintln!(
        "dumped {ok} cells, skipped {skipped} (of {} discovered)",
        paths.len()
    );
}
