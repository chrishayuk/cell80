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
    #[cfg(target_os = "macos")]
    gpu: rustmsl::GpuBatch,
}

impl CompiledGene {
    pub fn load(cells_dir: &Path, name: &str) -> Result<Self, String> {
        let path = cell80::find_cell_file(cells_dir, name)?;
        let src =
            fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
        let (funcs, consts) = lower(&src)?;
        #[cfg(target_os = "macos")]
        let gpu = {
            let module = rustmsl::compile(&funcs, &consts, "run")?;
            rustmsl::GpuBatch::new(&module)?
        };
        Ok(CompiledGene {
            name: name.to_string(),
            funcs,
            consts,
            #[cfg(target_os = "macos")]
            gpu,
        })
    }

    /// Run on the CPU reference interpreter — a fresh `Interp` per call (cheap: it only
    /// builds an address table and a 64 KiB memory image, no re-parsing), matching the
    /// `interp_run` pattern already proven in `gpu_cells.rs`/`msl_battery.rs`. Returns
    /// `(result, ir_steps)`.
    pub fn run_cpu(&self, args: &[u16]) -> (u16, u64) {
        let mut interp = Interp::new(
            &self.funcs,
            self.consts.iter().map(|(n, b)| (n.as_str(), b.as_slice())),
            Target::Cell.descriptor(),
        );
        let out = interp
            .run("run", args)
            .unwrap_or_else(|e| panic!("interp run `{}`: {e}", self.name));
        (out[0], interp.steps())
    }

    /// Run a whole tick's worth of organisms in one Metal dispatch — the "one cell × N
    /// inputs" batch layout, valid here because every organism in EX-0 shares this one
    /// compiled genome (heterogeneous-genome batching is explicitly out of scope, see the
    /// design doc). Returns `(result, ir_steps)` per organism, same order as `inputs`.
    #[cfg(target_os = "macos")]
    pub fn run_gpu_batch(&self, inputs: &[[u16; 3]]) -> Vec<(u16, u64)> {
        let outs = self
            .gpu
            .run(inputs)
            .unwrap_or_else(|e| panic!("gpu run `{}`: {e}", self.name));
        outs.iter()
            .map(|o| (o[0], rustmsl::steps_of(o) as u64))
            .collect()
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
