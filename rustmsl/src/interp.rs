//! The **bytecode interpreter** backend: a fixed-size MSL kernel that reads each
//! cell's IR from a data buffer, so a library dispatch's kernel size is constant
//! in the number of cells — the library-dispatch body of the two-body design
//! ([`crate::GpuBatch`]/[`crate::compile_library`] compile one cell × N inputs
//! for single-cell batch; this interprets a whole library × a probe set).
//!
//! Pricing (`cell80/examples/library_launch_cost.rs`) found that *compiling*
//! cells makes the kernel grow with the library and hits a kernel-size cliff at
//! ~64→128 fused cells (~44×); this backend has no such cliff — flat/no-cliff to
//! 500k distinct entries (153 MiB) at ~23 ns/eval on a representative corpus.
//! The trade is per-eval speed: at small scale the compiled path wins (~an order
//! of magnitude), so the two bodies hand off around 10²–10³ cells. What this
//! backend buys is *scale* — the compiled path cannot exist at thousands of cells.
//!
//! Everything here is **bit-identical to `cell80_core::Interp`** — values *and*
//! IR-step counts. Three design points are baked into the bytecode format:
//! 1. **Step parity via emitted `Step` markers**, placed at the tree-walker's
//!    exact charge points (per statement, per loop-iteration attempt, per
//!    expression node — except unrolled shift-amount literals), coalesced within
//!    basic blocks but never across a jump target, so completed-run counts match
//!    and every loop back-edge stays a fuel-check point.
//! 2. **No call stack** — calls are fully inlined at linearize time (the
//!    recursion gate makes that total); `__bits_*` builtins are intrinsic ops.
//! 3. **Per-cell offset table + concatenated code**, dispatched one cell per
//!    threadgroup with probes across lanes, so bytecode fetch is group-uniform.
//!
//! Supported subset (the rest bail with a typed [`Bail`], reported by callers):
//! value cells over u8/u16/i16/u32/i32/bool, incl. control flow, short-circuit
//! logic, `halt`, div/shift/compare at both widths, and inlined helper calls.
//! Not yet: state cells (state-window addressing is reserved), runtime-amount
//! shifts, memory/array ops, wide-returning inlined calls, `wide_second`.
//!
//! Split by concern (each file is one stage of `Func` → bytecode → result):
//! [`bytecode`] the shared `Inst`/`CellProgram`/`Bail` types plus the
//! opcode/`pack` encoder every dispatch backend shares, [`source`] the
//! MSL/CUDA kernel-source generators built on those opcodes, [`linearize`]
//! the lowering pass, [`cpu`] the CPU reference VM, [`gpu`] the Metal
//! executor (macOS only). `cuda.rs` and `cpu_emu.rs` (crate siblings, not
//! submodules here) reach `bytecode::pack` and `source::interp_source_cuda`
//! to build their own dispatch batches over the same bytecode. The public
//! surface (`linearize`, `cpu_run`, `CellProgram`, `VmOut`, `InterpBatch`,
//! `interp_source_msl`, `interp_source_cuda`) is unchanged by the split —
//! all still reachable at `rustmsl::interp::*`.

pub(crate) mod bytecode;
mod cpu;
#[cfg(target_os = "macos")]
mod gpu;
mod linearize;
mod source;

pub use bytecode::{Bail, CellProgram, STACK_CAP};
pub use cpu::{cpu_run, VmOut};
#[cfg(target_os = "macos")]
pub use gpu::InterpBatch;
pub use linearize::linearize;
pub use source::{interp_source_cuda, interp_source_msl};

/// Inputs consumed per thread (the register-arg triple).
pub use crate::IN_STRIDE;
/// Outputs produced per (cell, probe): `[r0, r1, r2, status, steps_lo, steps_hi]`
/// — the same sextet the compiled backend produces.
pub use crate::OUT_STRIDE;

#[cfg(test)]
mod tests;
