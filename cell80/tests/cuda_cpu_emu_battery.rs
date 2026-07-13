//! The pre-silicon CUDA battery: the full library sweeps of
//! `cuda_battery.rs`, but executed by [`rustmsl::cpu_emu`] — the emitted
//! CUDA text compiled by the **host C++ compiler** — instead of an NVIDIA
//! GPU. Same shared harness (`battery_common/`), same floors, same
//! transcript digests (a hit means the CUDA text reproduced the oracle's
//! sextet stream bit-for-bit, steps included), so the docs/16 cloud session
//! starts with the dialect's semantics already validated end-to-end over
//! the whole library; only NVRTC acceptance and NVIDIA codegen remain.
//!
//! Never cited as silicon verification. `#[ignore]`d: each value/state cell
//! is one host C++ compile (~minutes over the library) — run explicitly
//! before the cloud session:
//!
//! ```sh
//! cargo test -p cell80 --release --test cuda_cpu_emu_battery -- --ignored --nocapture
//! ```

mod battery_common;

use battery_common::*;

/// The CPU-emulation backend: CUDA dialect text, host-compiler execution.
/// Never blesses transcripts (it is graded BY them).
const EMU: Backend = Backend {
    label: "cuda-cpu-emu",
    bless: false,
    compile: rustmsl::compile_cuda,
    compile_library: rustmsl::compile_library_cuda,
    run: rustmsl::cpu_emu::run,
    run_with_state: rustmsl::cpu_emu::run_with_state,
};

/// Every eligible value cell through the CUDA text on the CPU — 512 random
/// inputs each, digest-compared against the blessed transcripts.
#[test]
#[ignore = "pre-silicon deep check (one host C++ compile per cell) — run before the docs/16 session"]
fn e1_e2_battery_cuda_text() {
    value_battery(512, &EMU);
}

/// Every eligible state cell through the CUDA text on the CPU — 256
/// (input, state) pairs each, sextets + final state bytes digest-compared.
#[test]
#[ignore = "pre-silicon deep check (one host C++ compile per cell) — run before the docs/16 session"]
fn state_cells_battery_cuda_text() {
    state_battery(256, &EMU);
}

/// The fused megakernel through the CUDA text on the CPU — the whole
/// library in ONE translation unit, one host compile: the same fused-scale
/// shape where Metal's compiler bug surfaced, checked before NVRTC sees it.
#[test]
#[ignore = "pre-silicon deep check (one large host C++ compile) — run before the docs/16 session"]
fn library_megakernel_cuda_text() {
    megakernel_battery(&EMU);
}
