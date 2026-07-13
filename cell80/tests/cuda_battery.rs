//! The CUDA library battery (`--features cuda`, docs/16 runbook): the exact
//! sweep `msl_battery.rs` runs on Metal, on an NVIDIA box instead — every
//! eligible value cell and state cell compiled in the CUDA dialect, run
//! through [`rustmsl::CudaBatch`], and graded bit-exact (values + trap
//! status + IR-step counts, and final state bytes for state cells) against
//! the reference interpreter. The battery loops themselves live in
//! `battery_common/` and are shared verbatim with the Metal battery, so the
//! two gates cannot drift.
//!
//! **Transcripts are read, never written, here**: the oracle-transcript book
//! (`tests/golden/msl_oracle_transcripts.json`) records interpreter-side
//! facts — key, source hash, seed, oracle digest — so it is
//! backend-independent by construction, and this battery consumes the same
//! entries the Metal gate blessed. A hit costs one GPU dispatch + a digest
//! compare; a miss (or any disagreement) grades against the live oracle
//! fanned across the box's cores. Blessing stays a macOS/Metal activity.
//!
//! The pre-registered CUDA gate (docs 14 ledger): the `*_one_million` tests,
//! run in release on the pinned box per `docs/16-cuda-gate-runbook.md`.

#![cfg(feature = "cuda")]

mod battery_common;

use battery_common::*;
use rustmsl::CudaBatch;

/// The CUDA backend: CUDA dialect + `CudaBatch`. Never blesses transcripts.
const CUDA: Backend = Backend {
    label: "cuda",
    bless: false,
    compile: rustmsl::compile_cuda,
    compile_library: rustmsl::compile_library_cuda,
    run: cuda_run,
    run_with_state: cuda_run_with_state,
};

fn cuda_run(m: &rustmsl::GpuModule, inputs: &[[u16; 3]]) -> Result<Vec<[u16; 6]>, String> {
    CudaBatch::new(m)?.run(inputs)
}

fn cuda_run_with_state(
    m: &rustmsl::GpuModule,
    inputs: &[[u16; 3]],
    state_in: &[u8],
) -> Result<(Vec<[u16; 6]>, Vec<u8>), String> {
    CudaBatch::new(m)?.run_with_state(inputs, state_in)
}

/// The CI-speed battery: every eligible cell, a corner sweep + 512 random
/// inputs each. The full pre-registered gate is [`gate_one_million_cuda`].
#[test]
fn e1_e2_battery_cuda() {
    println!(
        "cuda toolchain: {}",
        rustmsl::toolchain_info().unwrap_or_else(|e| e)
    );
    let n = std::env::var("CELL80_CUDA_FUZZ_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(512);
    value_battery(n, &CUDA);
}

/// The pre-registered CUDA E1+E2 gate: 10⁶ random inputs per admitted value
/// cell, values + status + steps bit-exact. Run in release on the gate box.
#[test]
#[ignore = "the 10^6-input gate — run explicitly in release on the CUDA box"]
fn gate_one_million_cuda() {
    value_battery(1_000_000, &CUDA);
}

/// E3 on CUDA: the whole library fused into one translation unit (one NVRTC
/// compile), one launch — this is also where a CUDA compiler quirk at fused
/// scale would surface, the analogue of the Metal branch-inversion find.
#[test]
fn library_megakernel_matches_interpreter_cuda() {
    megakernel_battery(&CUDA);
}

/// The CI-speed state battery: corner sweep + 256 random (input, state)
/// pairs per cell. The full gate is [`state_gate_one_million_cuda`].
#[test]
fn state_cells_battery_cuda() {
    let n = std::env::var("CELL80_CUDA_FUZZ_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(256);
    state_battery(n, &CUDA);
}

/// The pre-registered CUDA state gate: 10⁶ random (input, state) pairs per
/// cell — values, status, steps, AND final state bytes bit-exact.
#[test]
#[ignore = "the 10^6-input state gate — run explicitly in release on the CUDA box"]
fn state_gate_one_million_cuda() {
    state_battery(1_000_000, &CUDA);
}
