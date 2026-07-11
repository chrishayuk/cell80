//! rustmsl — the Metal (MSL) sibling of rustz80/rustrv32 (Phase 6 WS-E).
//!
//! One IR, one oracle: [`compile`] lowers the `cell80-core` typed IR for
//! **straight-line integer cells** (E1) to a Metal compute kernel — one thread
//! per input triple — and the reference interpreter defines every result bit. A
//! GPU result that does not agree bit-for-bit with the interpreter is a defect,
//! never a "GPU difference" (docs 14, non-goals).
//!
//! The codegen (IR → MSL text) is platform-independent and testable anywhere;
//! the executor ([`GpuBatch`], macOS only) compiles the source with fast-math
//! off and runs batches on the system's Metal device.

mod codegen;

pub use codegen::{
    compile, MslModule, CONST_BASE, IN_STRIDE, KERNEL_NAME, OUT_STRIDE, SCRATCH, STATUS_DIV0,
    STATUS_HALT, STATUS_OK, STATUS_OOW,
};

#[cfg(target_os = "macos")]
mod runtime;

#[cfg(target_os = "macos")]
pub use runtime::GpuBatch;
