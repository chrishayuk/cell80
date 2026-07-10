//! rustrv32 — the RV32I(M) sibling of rustz80 (Phase 5 WS-B,
//! `docs/13-multi-target-spec.md` §3): its own symbolic instruction layer under the
//! shared peephole-testing *discipline*, a cycle-accounted reference executor, and
//! restricted-Rust codegen over the `cell80-core` IR. Deployment driver: Hazard3 on
//! RP2350 — the `mcycle` co-sign at B4 is what certifies the cycle model.
//!
//! **WS-B lands in slices.** This is the first: the [`ins`] layer + exact encoder
//! (pinned by encoding goldens, `tests/encodings.rs`) and the cycle-accounted
//! RV32IM executor ([`exec`], B2 — semantics + determinism tests). Next: the Sail
//! adversary as a linux-only CI job (B1's emission oracle — spec §6 risk 2), then
//! codegen (B1) joining the diff harness's `TARGETS` matrix (B3).

pub mod exec;
pub mod ins;

pub use exec::{run_fn, Rv32, Stop, RETURN_SENTINEL, SRAM_BASE};
pub use ins::{encode, Alu, AluI, Bcc, Ins, LoadW, Reg, StoreW};
