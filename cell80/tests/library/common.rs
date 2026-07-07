//! Shared host-oracle helpers for every pack file under `cell80/tests/library/` — split
//! out of the former monolithic `cell80/tests/library.rs` (2026-07-07) so each pack's
//! tests don't have to redeclare them.

use cell80::{Runner, DEFAULT_CYCLES};
use std::path::PathBuf;

/// Read a library cell's source by id (`cells/<pack>/<id>.rs`, searched recursively —
/// cells live in pack subdirectories).
pub fn cell_src(id: &str) -> String {
    let cells_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cells");
    let p = cell80::find_cell_file(&cells_dir, id).unwrap_or_else(|e| panic!("{e}"));
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

/// Compile + run a cell on `args`, returning its `HL` result.
pub fn run_cell(id: &str, args: &[u16]) -> u16 {
    let mut r = Runner::compile(&cell_src(id)).unwrap_or_else(|e| panic!("compile {id}: {e}"));
    r.run(None, args, DEFAULT_CYCLES)
        .unwrap_or_else(|e| panic!("run {id}: {e}"))
        .result
}
