//! Compiling stdlib gene cells for EX-0's two bodies: `cell80-core::Interp` (the CPU
//! reference oracle) and, on macOS, `rustmsl::GpuBatch` (the Metal body). Both bodies run
//! the *same* lowered IR — "GPU ≡ interpreter" is a comparison of two executors over one
//! IR, not two independent readings of a cell's source.
use std::fs;
use std::path::Path;

use cell80_core::{Func, Interp, Target};

type Funcs = Vec<(String, Func)>;
type Consts = Vec<(String, Vec<u8>)>;

/// The cartridge pipeline up to the IR seam: prelude append, lower, inline, DCE-root at
/// `run` — the same pipeline `cell80/examples/gpu_cells.rs`'s `lower` and
/// `cell80/tests/msl_battery.rs`'s `lower_cell` already use, proven bit-exact against both
/// the Z80 body and Metal at full library scale (docs/14-model-native-cells-spec.md).
fn lower(src: &str) -> Result<(Funcs, Consts), String> {
    let combined = format!("{src}\n{}{}", cell80::CELL_PRELUDE, rustz80::F32_KERNELS);
    let file: syn::File = syn::parse_str(&combined).map_err(|e| format!("parse: {e}"))?;
    let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default())?;
    if !lowered.funcs.iter().any(|(n, _)| n == "run") {
        return Err("no `run` entry".to_string());
    }
    let consts = lowered.const_data();
    let funcs = cell80_core::inline::inline(lowered.funcs, &["run"]);
    let funcs = cell80_core::dce::prune(funcs, &["run"]);
    Ok((funcs, consts))
}

/// One gene cell, lowered once at load time and runnable on both bodies. Value cells only
/// — EX-0's grazer genome has no `&mut self` state cells among its six gene roles (verified
/// by reading `sub_sat`/`is_gt`/`add_sat`/`argmax3`/`is_ge`/`discount_percent`); a genuine
/// state cell would need `rustmsl::GpuBatch::run_with_state`, not `run`, and is out of
/// scope for EX-0 (see the design doc's deferred-items list).
pub struct CompiledGene {
    pub name: String,
    funcs: Funcs,
    consts: Consts,
    // `None` for a cell with no Metal body — currently only EX-2's composed candidates
    // (`from_funcs`), which run CPU-only by design (see the design doc's rationale: composed
    // multi-`Func` candidates reopen const/namespacing questions the disk-loaded path
    // sidesteps, for no real payoff given how rare they are). Every disk-loaded gene
    // (`load`) still gets a real `GpuBatch`, unchanged from EX-0/EX-1.
    #[cfg(target_os = "macos")]
    gpu: Option<rustmsl::GpuBatch>,
}

impl CompiledGene {
    pub fn load(cells_dir: &Path, name: &str) -> Result<Self, String> {
        let path = cell80::find_cell_file(cells_dir, name)?;
        let src =
            fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
        let (funcs, consts) = lower(&src)?;
        Self::from_funcs_impl(name.to_string(), funcs, consts, true)
    }

    /// Build a `CompiledGene` directly from already-lowered IR — EX-2's composed candidates
    /// (a synthetic multi-`Func` call graph, not something loaded from a `.rs` file on
    /// disk). Always CPU-only (`gpu: None` on macOS too), never attempts a Metal compile.
    pub fn from_funcs(name: &str, funcs: Funcs, consts: Consts) -> Result<Self, String> {
        Self::from_funcs_impl(name.to_string(), funcs, consts, false)
    }

    fn from_funcs_impl(
        name: String,
        funcs: Funcs,
        consts: Consts,
        want_gpu: bool,
    ) -> Result<Self, String> {
        #[cfg(target_os = "macos")]
        let gpu = if want_gpu {
            let module = rustmsl::compile(&funcs, &consts, "run")?;
            Some(rustmsl::GpuBatch::new(&module)?)
        } else {
            None
        };
        #[cfg(not(target_os = "macos"))]
        let _ = want_gpu;
        Ok(CompiledGene {
            name,
            funcs,
            consts,
            #[cfg(target_os = "macos")]
            gpu,
        })
    }

    /// Run on the CPU reference interpreter — a fresh `Interp` per call (cheap: it only
    /// builds an address table and a 64 KiB memory image, no re-parsing), matching the
    /// `interp_run` pattern already proven in `gpu_cells.rs`/`msl_battery.rs`. A trap
    /// (divide-by-zero, fuel exhaustion, or an explicit `halt(code)`) is folded to an r0
    /// value using the exact convention `cell80/tests/msl_battery.rs`'s `interp_quad`
    /// already established as bit-exact against the GPU kernel's own status/r0 encoding —
    /// `0` for divide-by-zero/fuel, `code` for `halt(code)` — rather than panicking. EX-0/
    /// EX-1's six curated gene cells never happened to trap under the inputs exercised, so
    /// this path was latent, not proven, until EX-2's cell-swap pool started calling
    /// arbitrary same-signature stdlib cells (not curated for this use) with arbitrary
    /// organism-supplied inputs — some legitimately trap, and that must fold the same way
    /// on both bodies to keep "GPU ≡ interpreter" bit-exact. Any *other* error is a real
    /// defect (e.g. a malformed program), not a trap, and still panics. Returns `(result,
    /// ir_steps)`.
    pub fn run_cpu(&self, args: &[u16]) -> (u16, u64) {
        let mut interp = Interp::new(
            &self.funcs,
            self.consts.iter().map(|(n, b)| (n.as_str(), b.as_slice())),
            Target::Cell.descriptor(),
        );
        let result = match interp.run("run", args) {
            Ok(v) => v.first().copied().unwrap_or(0),
            Err(e) if e.contains("divide by zero") => 0,
            Err(e) if e.contains("fuel exhausted") => 0,
            Err(e) => e
                .strip_prefix("interp: halt(")
                .and_then(|s| s.strip_suffix(')'))
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or_else(|| panic!("interp run `{}`: {e}", self.name)),
        };
        (result, interp.steps())
    }

    /// Run a whole tick's worth of organisms in one Metal dispatch — the "one cell × N
    /// inputs" batch layout, valid here because every organism sharing this dispatch uses
    /// this one compiled genome (heterogeneous-cell-choice batching happens one level up,
    /// via `batch_run_grouped`). Falls back to the CPU path when this gene has no GPU body
    /// (a composed candidate from `from_funcs`) — still bit-exact, just not GPU-dispatched.
    /// Returns `(result, ir_steps)` per organism, same order as `inputs`.
    #[cfg(target_os = "macos")]
    pub fn run_gpu_batch(&self, inputs: &[[u16; 3]]) -> Vec<(u16, u64)> {
        match &self.gpu {
            Some(gpu) => {
                let outs = gpu
                    .run(inputs)
                    .unwrap_or_else(|e| panic!("gpu run `{}`: {e}", self.name));
                outs.iter()
                    .map(|o| (o[0], rustmsl::steps_of(o) as u64))
                    .collect()
            }
            None => inputs.iter().map(|args| self.run_cpu(args)).collect(),
        }
    }
}

/// The six gene roles EX-0's homogeneous grazer genome needs, each compiled once at
/// startup and reused every tick.
pub struct GeneSet {
    pub decay: CompiledGene,
    pub hungry_promoter: CompiledGene,
    pub eat: CompiledGene,
    pub sense_move: CompiledGene,
    pub repro_promoter: CompiledGene,
    pub split: CompiledGene,
}

impl GeneSet {
    pub fn load(cells_dir: &Path, genes: &crate::StartingGenes) -> Result<Self, String> {
        Ok(GeneSet {
            decay: CompiledGene::load(cells_dir, &genes.decay)?,
            hungry_promoter: CompiledGene::load(cells_dir, &genes.hungry_promoter)?,
            eat: CompiledGene::load(cells_dir, &genes.eat)?,
            sense_move: CompiledGene::load(cells_dir, &genes.sense_move)?,
            repro_promoter: CompiledGene::load(cells_dir, &genes.repro_promoter)?,
            split: CompiledGene::load(cells_dir, &genes.split)?,
        })
    }
}

/// Which body runs a tick's gene calls — shared by `ex0.rs` (1D) and `ex1.rs` (2D) so
/// there's exactly one "how do I dispatch a batch of organisms against one gene" concept,
/// not two copies of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineKind {
    CpuReference,
    #[cfg(target_os = "macos")]
    Gpu,
}

/// Run one gene role against every living organism's own input triple in a single batched
/// call: a per-organism `Interp` loop for `CpuReference`, one `GpuBatch::run` dispatch for
/// `Gpu`. Returns `(result, ir_steps)` per organism, same order as `inputs`.
pub fn batch_run(engine: EngineKind, gene: &CompiledGene, inputs: &[[u16; 3]]) -> Vec<(u16, u64)> {
    match engine {
        EngineKind::CpuReference => inputs.iter().map(|args| gene.run_cpu(args)).collect(),
        #[cfg(target_os = "macos")]
        EngineKind::Gpu => gene.run_gpu_batch(inputs),
    }
}

/// Total IR-step cost across a tick's gene-role batches — a single summed aggregate (see
/// `history::TickRecord::total_ir_steps`'s doc comment for why that's a stated
/// simplification, not a per-organism-per-role trace).
pub fn sum_steps(batches: &[&[(u16, u64)]]) -> u64 {
    batches.iter().flat_map(|b| b.iter()).map(|(_, s)| s).sum()
}

/// EX-2's heterogeneous-cell-choice dispatch: `role_idx[i]` names which `pool` member
/// organism `i` currently uses for this role. Partitions `inputs` by that index, issues one
/// `batch_run` per *distinct* value in `role_idx` (not one per organism), and scatters each
/// group's results back to their original positions. Dispatch count per tick is bounded by
/// how many distinct pool members are actually in use, not by population size — cheap while
/// genome diversity stays low, the expected regime early in a mutation-driven run (see the
/// design doc's dispatch-count-as-a-receipt discipline).
///
/// Determinism note (load-bearing for EX-2/EX-4's replay guarantees): the internal
/// `HashMap`'s iteration order is genuinely process-randomized, but every result is
/// scattered back by *original index* (`out[i] = ...`), never appended in group-iteration
/// order — so no element's value can depend on which group the map happens to visit first.
/// A future "simplification" that concatenated group outputs in iteration order instead of
/// scattering by index would silently reintroduce real nondeterminism here.
pub fn batch_run_grouped(
    engine: EngineKind,
    pool: &[CompiledGene],
    role_idx: &[u16],
    inputs: &[[u16; 3]],
) -> Vec<(u16, u64)> {
    debug_assert_eq!(role_idx.len(), inputs.len());
    let mut out = vec![(0u16, 0u64); inputs.len()];
    let mut positions: std::collections::HashMap<u16, Vec<usize>> = std::collections::HashMap::new();
    for (i, &idx) in role_idx.iter().enumerate() {
        positions.entry(idx).or_default().push(i);
    }
    for (idx, group_positions) in positions {
        let gene = &pool[idx as usize];
        let group_inputs: Vec<[u16; 3]> = group_positions.iter().map(|&i| inputs[i]).collect();
        let group_out = batch_run(engine, gene, &group_inputs);
        for (&i, result) in group_positions.iter().zip(group_out) {
            out[i] = result;
        }
    }
    out
}

#[cfg(test)]
mod grouped_tests {
    use super::*;
    use std::path::Path;

    fn cells_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cell80/cells")
    }

    #[test]
    fn grouped_matches_naive_per_organism_dispatch() {
        let pool = vec![
            CompiledGene::load(&cells_dir(), "is_gt").unwrap(),
            CompiledGene::load(&cells_dir(), "is_ge").unwrap(),
        ];
        // A handful of organisms, some sharing a pool index, none in a tidy sorted order —
        // exercises the grouping/scatter step, not just the trivial single-group case.
        let role_idx: Vec<u16> = vec![0, 1, 0, 0, 1, 0, 1];
        let inputs: Vec<[u16; 3]> = (0..role_idx.len() as u16)
            .map(|i| [i * 3, i * 3 + 1, 0])
            .collect();

        let grouped = batch_run_grouped(EngineKind::CpuReference, &pool, &role_idx, &inputs);
        let naive: Vec<(u16, u64)> = role_idx
            .iter()
            .zip(&inputs)
            .map(|(&idx, args)| pool[idx as usize].run_cpu(args))
            .collect();

        assert_eq!(grouped, naive);
    }
}
