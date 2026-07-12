//! rustmsl — the Metal (MSL) sibling of rustz80/rustrv32 (Phase 6 WS-E).
//!
//! One IR, one oracle: [`compile`] lowers the `cell80-core` typed IR for
//! integer cells — straight-line (E1) and looping/branching (E2), budget-bounded
//! by the interpreter's exact fuel discipline — to a Metal compute kernel, one
//! thread per (cell, input) pair, and the reference interpreter defines every
//! result bit *and every step count*. A GPU result that does not agree
//! bit-for-bit with the interpreter is a defect, never a "GPU difference"
//! (docs 14, non-goals).
//!
//! [`compile_library`] fuses many cells into one translation unit — the
//! library × probe-set megakernel layout (E3), retrieval-by-execution's
//! substrate.
//!
//! The codegen (IR → MSL text) is platform-independent and testable anywhere;
//! the executor ([`GpuBatch`], macOS only) compiles the source with fast-math
//! off and runs batches on the system's Metal device.

mod codegen;

pub use codegen::{
    compile, compile_library, CellMeta, LibraryCell, MslModule, CONST_BASE, FUEL, IN_STRIDE,
    KERNEL_NAME, OUT_STRIDE, SCRATCH, STATE_BASE, STATUS_DIV0, STATUS_FUEL, STATUS_HALT, STATUS_OK,
    STATUS_OOW,
};

#[cfg(target_os = "macos")]
mod runtime;

#[cfg(target_os = "macos")]
pub use runtime::GpuBatch;

/// The bytecode-interpreter backend — the library × probe-set body of the
/// two-body design (this crate's `compile_library`/`GpuBatch` are the single
/// cell × N-inputs body). Bytecode, linearizer and CPU reference VM build
/// everywhere; [`interp::InterpBatch`] is the macOS/Metal library dispatch.
pub mod interp;

/// A thread's decoded step count from its output sextet (`steps_lo`,
/// `steps_hi` — the interpreter-identical IR-step cost, docs 14 Q2).
pub fn steps_of(out: &[u16; OUT_STRIDE]) -> u32 {
    out[4] as u32 | ((out[5] as u32) << 16)
}
