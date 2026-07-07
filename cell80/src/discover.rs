//! Recursive discovery of cell source files under a library directory. Cells live in
//! pack subdirectories (`cell80/cells/<pack>/<id>.rs`), so every "list every cell" or
//! "find this cell by id" caller walks the tree instead of assuming a flat directory —
//! the flat layout `docs/library-growth.md` once described is gone.

use std::path::{Path, PathBuf};

/// Every `.rs`/`.cell` cell source under `dir`, discovered recursively, sorted for
/// determinism. Anything else (a pack's own `README.md`, say) is skipped.
pub fn discover_cell_files(dir: &str) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    walk(Path::new(dir), &mut out).map_err(|e| format!("{dir}: {e}"))?;
    out.sort();
    Ok(out)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            walk(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "rs" || e == "cell") {
            out.push(path);
        }
    }
    Ok(())
}

/// Find a specific cell's `.rs` source by id, searching recursively under `dir` — a
/// caller resolving a call target by name doesn't know which pack subdirectory it
/// landed in.
pub fn find_cell_file(dir: &Path, id: &str) -> Result<PathBuf, String> {
    let target = format!("{id}.rs");
    find(dir, &target).ok_or_else(|| format!("{id}: not found under {}", dir.display()))
}

fn find(dir: &Path, target: &str) -> Option<PathBuf> {
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find(&path, target) {
                return Some(found);
            }
        } else if path.file_name().and_then(|n| n.to_str()) == Some(target) {
            return Some(path);
        }
    }
    None
}
