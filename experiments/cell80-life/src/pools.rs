//! Role-pool discovery for EX-2's cell-swap mutation (`experiments/deterministic-ecology.md`).
//! Lifted from `main.rs`'s `discover_pools` (the original CPU binary's own mutation model),
//! not a modification of it — `main.rs` stays exactly as it is, untouched.
use std::fs;
use std::path::Path;

use cell80::{Cartridge, CartridgeOpts, CellConfig};

/// The two same-signature candidate pools EX-2's swappable roles draw from: 2-`u16`-arg
/// promoters (`hungry_promoter`/`repro_promoter`) and 3-`u16`-arg movement cells
/// (`sense_move`). Pool order feeds `rng::pick_other_index`'s index choices, so it must be
/// stable and reproducible — sorted by cell id, not directory-walk order (see
/// `discover_pools`'s doc comment).
pub struct Pools {
    pub promoters: Vec<String>,
    pub movement: Vec<String>,
}

impl Pools {
    /// Look up a named cell's index in the promoter pool — the bridge from a
    /// `StartingGenome`'s named `hungry_promoter`/`repro_promoter` cell to the pool-index
    /// form EX-2's per-organism genome uses.
    pub fn promoter_index(&self, name: &str) -> u16 {
        self.promoters
            .iter()
            .position(|n| n == name)
            .unwrap_or_else(|| panic!("`{name}` not found in the discovered promoter pool"))
            as u16
    }

    /// Same as `promoter_index`, for the movement (`sense_move`) pool.
    pub fn movement_index(&self, name: &str) -> u16 {
        self.movement
            .iter()
            .position(|n| n == name)
            .unwrap_or_else(|| panic!("`{name}` not found in the discovered movement pool"))
            as u16
    }
}

/// Scan every `.rs` cell source under `cells_dir` (recursively, cells live in pack
/// subdirectories), compile each, and bucket it by arity into the promoter pool (2 `u16`
/// params, `u16` return) or the movement pool (3 params) — skipping anything that fails to
/// compile, has `&mut self` state (a plain fn call can't read its fields back), or
/// returns/takes anything other than `u16`. Bucketed **by cell id**, sorted, not by
/// directory order, so pool-index-based mutation stays reproducible regardless of how the
/// filesystem happens to enumerate pack subdirectories.
pub fn discover_pools(cells_dir: &Path) -> Pools {
    let mut named_paths: Vec<(String, std::path::PathBuf)> =
        cell80::discover_cell_files(cells_dir.to_str().unwrap())
            .unwrap_or_else(|e| panic!("{e}"))
            .into_iter()
            .filter(|p| p.extension().is_some_and(|ext| ext == "rs"))
            .filter_map(|p| {
                let name = p.file_stem()?.to_string_lossy().into_owned();
                Some((name, p))
            })
            .collect();
    named_paths.sort_by(|a, b| a.0.cmp(&b.0));

    let mut promoters = Vec::new();
    let mut movement = Vec::new();
    for (name, path) in named_paths {
        let Ok(src) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(cart) = Cartridge::compile(
            &src,
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some(name.clone()),
                ..Default::default()
            },
        ) else {
            continue;
        };
        let sig = &cart.manifest.signature;
        if !sig.state.is_empty()
            || sig.ret != "u16"
            || !sig.params.iter().all(|(_, ty)| ty == "u16")
        {
            continue;
        }
        match sig.params.len() {
            2 => promoters.push(name),
            3 => movement.push(name),
            _ => {}
        }
    }
    Pools {
        promoters,
        movement,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cells_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cell80/cells")
    }

    #[test]
    fn discovers_known_pool_members() {
        let pools = discover_pools(&cells_dir());
        // is_gt/is_ge back hungry_promoter/repro_promoter in both shipped genomes — must be
        // discoverable, and sorted (so pool-index mutation is reproducible run to run).
        assert!(pools.promoters.contains(&"is_gt".to_string()));
        assert!(pools.promoters.contains(&"is_ge".to_string()));
        assert!(pools.movement.contains(&"argmax3".to_string()));
        assert!(
            pools.promoters.windows(2).all(|w| w[0] <= w[1]),
            "promoters must be sorted"
        );
        assert!(
            pools.movement.windows(2).all(|w| w[0] <= w[1]),
            "movement must be sorted"
        );
        assert!(
            pools.promoters.len() >= 2,
            "need at least 2 promoters for swap mutation to have a choice"
        );
        assert!(
            pools.movement.len() >= 2,
            "need at least 2 movement cells for swap mutation to have a choice"
        );
    }
}
