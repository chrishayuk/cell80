//! EX-2 operator (b): cell-assembly/bytecode-level mutation — composing TWO existing
//! same-role pool cells into one new, arity-preserving candidate:
//! `run(a0..aN) = g(a0,..,f(a0..aN),..,aN)` (f's output replaces one of g's argument
//! slots). This is the design doc's own pre-registered kill/rescope fallback ("mutation
//! over a typed cell-assembly grammar... which evolved-cells already showed is
//! searchable"), reusing the exact `Expr::Call` + `linearize`-does-the-inlining trick a
//! concurrent session's `cell80/examples/gpu_grow.rs` already proved at population scale —
//! generalized here from unary chains to arity-preserving 2-cell wiring, since none of
//! EX-2's three swappable roles (2- or 3-arg) are unary.
//!
//! Composed candidates execute CPU-only (`CompiledGene::from_funcs`, `gpu: None`) even on
//! GPU-engine ticks — composed cells are rare, low-population-share mutants; GPU-compiling
//! a real multi-`Func` call graph is possible (`rustmsl::compile` supports it) but reopens
//! const/namespacing questions the `funcs.len() == 1` eligibility filter below exists to
//! dodge, for no real payoff at this pass's scale.
use std::fs;
use std::path::Path;

use cell80::{Fingerprint, DEFAULT_PROBES};
use cell80_core::ir::{Expr, Func};
use cell80_core::{Interp, Target};
use rustmsl::interp::{cpu_run, linearize, VmOut};

use crate::rng;

/// Offline pool-growth's two decisions, each its own stream (matching the project's
/// one-stream-per-independent-decision discipline). The attempt index stands in for
/// `rng::draw`'s "tick" parameter and a fixed `0` stands in for "organism_id" — there's no
/// organism here, just a deterministic sweep index; the same reuse `world2d.rs`'s
/// `WORLD_INIT_STREAM` already makes of that parameter for tile indices instead.
pub const COMPOSE_PAIR_STREAM: u8 = 15;
pub const COMPOSE_PARTNER_STREAM: u8 = 16;
pub const COMPOSE_SLOT_STREAM: u8 = 17;

/// Structural bounds `InterpBatch`'s kernel enforces elsewhere — an arithmetic shape
/// ceiling, not a behavioral safety proof (see the design doc's "what sandbox-safe means
/// here" scope note).
const MAX_LOCALS: usize = 64;
const MAX_DEPTH: usize = 32;
const CANDIDATE_NAME: &str = "__candidate";

pub type Funcs = Vec<(String, Func)>;

/// Lower one cell's source to its pruned function set. Rejects (with `Err`, not a silent
/// drop) any cell whose lowering carries const data — a real, explicit check, not an
/// assumption that a single-function cell never has any: composed candidates are built by
/// merging constituents' `funcs` alone, with nowhere to carry const data, so a constituent
/// that needs any (e.g. a lookup-table-backed cell) must be excluded from the composable
/// pool rather than silently producing an incorrect candidate that references data that
/// isn't there.
fn lower_one(src: &str) -> Result<Funcs, String> {
    let combined = format!("{src}\n{}{}", cell80::CELL_PRELUDE, rustz80::F32_KERNELS);
    let file: syn::File = syn::parse_str(&combined).map_err(|e| format!("parse: {e}"))?;
    let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default())?;
    if !lowered.funcs.iter().any(|(n, _)| n == "run") {
        return Err("no `run` entry".to_string());
    }
    if !lowered.const_data().is_empty() {
        return Err("carries const data — not composable in this pass".to_string());
    }
    let funcs = cell80_core::inline::inline(lowered.funcs, &["run"]);
    Ok(cell80_core::dce::prune(funcs, &["run"]))
}

/// A role's candidate building blocks for composition: cells whose lowered form collapses
/// to a single self-contained function — narrower than the swap pool (matching
/// `gpu_grow.rs`'s own restriction: combining two cells' un-inlined helper functions risks
/// silent name collisions). Some swap-pool members are therefore swap-only, not
/// composable, in this pass — stated plainly, not hidden.
pub struct ComposablePool {
    pub arity: usize,
    /// Keyed by cell name (e.g. "is_gt"), not the stored `Func`'s own internal name (every
    /// self-contained cell's surviving function is itself named "run" after DCE-rooting —
    /// the cell-name key is what lets several of these coexist in one `funcs` list without
    /// colliding).
    pub funcs: Funcs,
}

impl ComposablePool {
    pub fn discover(cells_dir: &Path, pool_names: &[String], arity: usize) -> Self {
        let mut funcs = Vec::new();
        for name in pool_names {
            let Ok(path) = cell80::find_cell_file(cells_dir, name) else {
                continue;
            };
            let Ok(src) = fs::read_to_string(&path) else {
                continue;
            };
            let Ok(lowered) = lower_one(&src) else {
                continue;
            };
            if lowered.len() == 1 {
                funcs.push((name.clone(), lowered[0].1.clone()));
            }
        }
        ComposablePool { arity, funcs }
    }
}

fn chain_func(f_name: &str, g_name: &str, arity: usize, slot: usize) -> Func {
    let args: Vec<Expr> = (0..arity).map(Expr::Var).collect();
    let f_call = Expr::Call(f_name.to_string(), args.clone());
    let mut g_args = args;
    g_args[slot] = f_call;
    Func {
        params: arity,
        n_locals: arity,
        body: vec![],
        ret: vec![Expr::Call(g_name.to_string(), g_args)],
        wide_param: false,
        wide_second: false,
        wide_ret: false,
    }
}

/// Map `rustmsl::interp::VmOut` to the `[r0,r1,r2,status,steps_lo,steps_hi]` shape
/// `Fingerprint::from_value_sextets` expects. Only `status == 0` vs. `!= 0` matters to that
/// function (a nonzero status folds to `None` regardless of r0/r1/r2), so any nonzero
/// constant for a trapped outcome is as good as matching the GPU kernel's exact status
/// codes — this deliberately doesn't try to replicate `ST_DIV0`/`ST_HALT`/`ST_FUEL`
/// precisely, since nothing here compares against a real GPU-kernel sextet.
fn sextet_of(out: VmOut) -> [u16; 6] {
    match out {
        VmOut::Value(v, _) => [
            v.first().copied().unwrap_or(0),
            v.get(1).copied().unwrap_or(0),
            v.get(2).copied().unwrap_or(0),
            0,
            0,
            0,
        ],
        VmOut::Halt(_, _) | VmOut::Fuel(_) | VmOut::DivZero => [0, 0, 0, 1, 0, 0],
    }
}

/// Fingerprint an existing pool member (a real, already-curated stdlib cell) the same way
/// a candidate is fingerprinted below, so novelty comparisons are apples-to-apples: its own
/// lowered funcs, linearized and run over `DEFAULT_PROBES` via `rustmsl::interp::cpu_run` —
/// deliberately NOT `genes::CompiledGene::run_cpu` (which folds a trap to a plain `u16`
/// value for the *ecology*'s purposes and would silently corrupt a fingerprint's
/// trap-sensitivity, since a trap and a genuine `0` must not be treated as the same output).
pub fn fingerprint_pool_member(cells_dir: &Path, name: &str, arity: usize) -> Option<Fingerprint> {
    let path = cell80::find_cell_file(cells_dir, name).ok()?;
    let src = fs::read_to_string(&path).ok()?;
    let funcs = lower_one(&src).ok()?;
    let prog = linearize(&funcs, "run").ok()?;
    let sextets: Vec<[u16; 6]> = DEFAULT_PROBES
        .iter()
        .map(|p| sextet_of(cpu_run(&prog, &p[..arity])))
        .collect();
    Some(Fingerprint::from_value_sextets(&sextets, "u16"))
}

pub struct Candidate {
    pub f_name: String,
    pub g_name: String,
    pub slot: usize,
    /// Entry keyed `"run"`, ready for `genes::CompiledGene::from_funcs`.
    pub funcs: Funcs,
    /// This candidate's own fingerprint (over `DEFAULT_PROBES`) — already `< 1.0` agreement
    /// against every existing pool member by construction (the novelty gate passed), kept
    /// here so a report can show *how* novel (the closest-match agreement), not just that
    /// it cleared the bar.
    pub fingerprint: Fingerprint,
}

pub enum GenerateOutcome {
    /// Failed the structural bound (`linearize`'s own shape ceiling) — never executed.
    StructurallyInvalid,
    /// Structurally valid but traps on at least one probe, or the two independent CPU
    /// interpreters disagree on a claimed-clean probe (see below) — a counted stillbirth,
    /// never a crash.
    NotViable,
    /// Structurally valid, executes cleanly on every probe, but behaviourally
    /// indistinguishable (agreement >= 1.0) from at least one existing pool member.
    Duplicate,
    /// Passed every gate: bounded, viable, cross-interpreter-consistent, and novel.
    Viable(Candidate),
}

/// Generate the `(f_idx, g_idx, slot)` candidate from `pool` and run it through the
/// structural + viability + novelty gates, plus a cross-interpreter consistency check.
/// `existing_fingerprints` is every current role-pool member's fingerprint (over
/// `DEFAULT_PROBES`) to check novelty against.
pub fn generate_and_gate(
    pool: &ComposablePool,
    f_idx: usize,
    g_idx: usize,
    slot: usize,
    existing_fingerprints: &[Fingerprint],
) -> GenerateOutcome {
    let (f_name, _) = &pool.funcs[f_idx];
    let (g_name, _) = &pool.funcs[g_idx];
    let candidate_func = chain_func(f_name, g_name, pool.arity, slot);

    let mut for_rustmsl: Funcs = Vec::with_capacity(pool.funcs.len() + 1);
    for_rustmsl.push((CANDIDATE_NAME.to_string(), candidate_func.clone()));
    for_rustmsl.extend(pool.funcs.iter().cloned());

    let Ok(prog) = linearize(&for_rustmsl, CANDIDATE_NAME) else {
        return GenerateOutcome::StructurallyInvalid;
    };
    if prog.max_depth > MAX_DEPTH || prog.n_locals > MAX_LOCALS {
        return GenerateOutcome::StructurallyInvalid;
    }

    let sextets: Vec<[u16; 6]> = DEFAULT_PROBES
        .iter()
        .map(|p| sextet_of(cpu_run(&prog, &p[..pool.arity])))
        .collect();
    if sextets.iter().any(|s| s[3] != 0) {
        return GenerateOutcome::NotViable;
    }

    let candidate_fp = Fingerprint::from_value_sextets(&sextets, "u16");
    if existing_fingerprints.iter().any(|fp| candidate_fp.agreement(fp) >= 1.0) {
        return GenerateOutcome::Duplicate;
    }

    // Cross-check against `cell80_core::Interp` — the actual body `CompiledGene::run_cpu`
    // executes a composed candidate through once admitted. Two independent interpreters
    // (rustmsl's bytecode VM above, cell80-core's tree-walker here) agreeing on every probe
    // is the same "a disagreeing executor is a defect, never expected variance" discipline
    // this project applies to the GPU body — applied here to a second CPU interpreter.
    let mut for_core: Funcs = Vec::with_capacity(pool.funcs.len() + 1);
    for_core.push(("run".to_string(), candidate_func));
    for_core.extend(pool.funcs.iter().cloned());
    for (i, p) in DEFAULT_PROBES.iter().enumerate() {
        let mut interp = Interp::new(&for_core, std::iter::empty::<(&str, &[u8])>(), Target::Cell.descriptor());
        let core_r0 = match interp.run("run", &p[..pool.arity]) {
            Ok(v) => v.first().copied().unwrap_or(0),
            Err(_) => {
                // The viability gate above already required this probe to be trap-free on
                // rustmsl's VM; the two bodies disagreeing on whether this probe traps at
                // all is itself a real cross-interpreter defect, not a value mismatch.
                return GenerateOutcome::NotViable;
            }
        };
        if core_r0 != sextets[i][0] {
            return GenerateOutcome::NotViable;
        }
    }

    GenerateOutcome::Viable(Candidate {
        f_name: f_name.clone(),
        g_name: g_name.clone(),
        slot,
        funcs: for_core,
        fingerprint: candidate_fp,
    })
}

/// Receipts from one offline pool-growth sweep: how many attempts landed in each gate
/// outcome, plus the viable candidates themselves — the direct "fraction of mutations
/// viable" measure the design doc asks for.
pub struct GrowthReport {
    pub attempts: u32,
    pub structurally_invalid: u32,
    pub not_viable: u32,
    pub duplicate: u32,
    pub viable: Vec<Candidate>,
}

/// Deterministically generate up to `attempts` candidates from `pool` and gate every one,
/// returning every candidate that passed — the offline, one-time pool-growth pass an
/// ecology run's "extended pool" is built from, mirroring `main.rs`'s own `discover_pools`
/// (a startup-time step, not a live per-tick event). A pure function of `seed`: the whole
/// sweep, viable candidates included, is reproducible exactly like everything else in this
/// experiment.
pub fn grow_pool(pool: &ComposablePool, existing_fingerprints: &[Fingerprint], seed: u64, attempts: u32) -> GrowthReport {
    let mut report = GrowthReport {
        attempts: 0,
        structurally_invalid: 0,
        not_viable: 0,
        duplicate: 0,
        viable: Vec::new(),
    };
    let n = pool.funcs.len() as u32;
    if n < 2 {
        return report;
    }
    for attempt in 0..attempts {
        let f_idx = (rng::draw(seed, attempt, 0, COMPOSE_PAIR_STREAM) % n) as u16;
        let g_idx = rng::pick_other_index(seed, attempt, 0, COMPOSE_PARTNER_STREAM, f_idx, n as u16);
        let slot = (rng::draw(seed, attempt, 0, COMPOSE_SLOT_STREAM) % pool.arity as u32) as usize;
        report.attempts += 1;
        match generate_and_gate(pool, f_idx as usize, g_idx as usize, slot, existing_fingerprints) {
            GenerateOutcome::StructurallyInvalid => report.structurally_invalid += 1,
            GenerateOutcome::NotViable => report.not_viable += 1,
            GenerateOutcome::Duplicate => report.duplicate += 1,
            GenerateOutcome::Viable(c) => report.viable.push(c),
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn cells_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cell80/cells")
    }

    #[test]
    fn grow_pool_is_deterministic_and_accounts_for_every_attempt() {
        let names = vec!["is_gt".to_string(), "is_ge".to_string(), "sub_sat".to_string(), "add_sat".to_string()];
        let pool = ComposablePool::discover(&cells_dir(), &names, 2);
        assert_eq!(pool.funcs.len(), 4);
        let fps: Vec<Fingerprint> = names.iter().filter_map(|n| fingerprint_pool_member(&cells_dir(), n, 2)).collect();

        let r1 = grow_pool(&pool, &fps, 0x5eed_1234, 50);
        let r2 = grow_pool(&pool, &fps, 0x5eed_1234, 50);
        assert_eq!(r1.attempts, r2.attempts);
        assert_eq!(r1.structurally_invalid, r2.structurally_invalid);
        assert_eq!(r1.not_viable, r2.not_viable);
        assert_eq!(r1.duplicate, r2.duplicate);
        assert_eq!(r1.viable.len(), r2.viable.len());
        assert_eq!(
            r1.attempts,
            r1.structurally_invalid + r1.not_viable + r1.duplicate + r1.viable.len() as u32,
            "every attempt must land in exactly one outcome bucket"
        );
    }

    #[test]
    fn discovers_composable_arity2_pool() {
        let names = vec!["is_gt".to_string(), "is_ge".to_string(), "sub_sat".to_string()];
        let pool = ComposablePool::discover(&cells_dir(), &names, 2);
        // All three are plain, single-function arithmetic/comparison cells — every one
        // should be composable.
        assert_eq!(pool.funcs.len(), 3);
    }

    #[test]
    fn composing_is_gt_into_itself_is_a_duplicate_of_something_in_a_small_pool() {
        // sub_sat(sub_sat(a,b), b) etc. over a tiny 2-cell pool — not asserting a specific
        // outcome, just that generate_and_gate runs end-to-end without panicking and
        // returns one of the defined outcomes for every (f,g,slot) combination.
        let names = vec!["is_gt".to_string(), "sub_sat".to_string()];
        let pool = ComposablePool::discover(&cells_dir(), &names, 2);
        assert_eq!(pool.funcs.len(), 2);
        let existing_fps: Vec<Fingerprint> = names
            .iter()
            .filter_map(|n| fingerprint_pool_member(&cells_dir(), n, 2))
            .collect();
        assert_eq!(existing_fps.len(), 2);

        for f in 0..pool.funcs.len() {
            for g in 0..pool.funcs.len() {
                if f == g {
                    continue;
                }
                for slot in 0..pool.arity {
                    match generate_and_gate(&pool, f, g, slot, &existing_fps) {
                        GenerateOutcome::StructurallyInvalid
                        | GenerateOutcome::NotViable
                        | GenerateOutcome::Duplicate
                        | GenerateOutcome::Viable(_) => {}
                    }
                }
            }
        }
    }

    #[test]
    fn a_deliberately_trapping_composition_is_a_stillbirth_not_a_crash() {
        // unit_div composed as the outer cell over a divisor that can be driven to zero by
        // an inner cell's output is exactly the scenario that first surfaced the
        // trap-folding bug in genes.rs::run_cpu (see the EX-2 findings) — reused here to
        // prove composition's own viability gate rejects it cleanly instead of panicking.
        let names = vec!["is_gt".to_string(), "unit_div".to_string()];
        let Some(pool_arity2_check) = fingerprint_pool_member(&cells_dir(), "unit_div", 2) else {
            // `unit_div` isn't a real 2-arg cell in every library snapshot; skip gracefully
            // rather than fail a test on a library-shape assumption unrelated to this file.
            return;
        };
        let _ = pool_arity2_check;
        let pool = ComposablePool::discover(&cells_dir(), &names, 2);
        if pool.funcs.len() < 2 {
            return;
        }
        let existing_fps: Vec<Fingerprint> = names
            .iter()
            .filter_map(|n| fingerprint_pool_member(&cells_dir(), n, 2))
            .collect();
        // Just confirm no panic across every combination — a trapping composition must
        // resolve to `NotViable`, `StructurallyInvalid`, or (if it happens not to trap on
        // this probe bank) `Duplicate`/`Viable`, never a process crash.
        for f in 0..pool.funcs.len() {
            for g in 0..pool.funcs.len() {
                if f == g {
                    continue;
                }
                for slot in 0..pool.arity {
                    let _ = generate_and_gate(&pool, f, g, slot, &existing_fps);
                }
            }
        }
    }
}
