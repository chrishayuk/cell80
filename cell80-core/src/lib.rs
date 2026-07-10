//! The cell80 compiler core (Phase 5 WS-A/A5 — `docs/13-multi-target-spec.md`
//! §2.5): the **target-independent** half of the cell-family, extracted from
//! rustz80 once the descriptor (A1), the width contract (A2), and the reference
//! interpreter (A4) made it real. Backend zero (rustz80) consumes this crate; new
//! backends (rustrv32, rustthumb) build against it without touching Z80.
//!
//! - [`ir`] — the typed IR and its target-independent semantic contract (widths
//!   explicit per value, the family-wide 2-byte slot ABI, explicit width bridges,
//!   left-to-right observable evaluation order).
//! - [`inline`] / [`dce`] — the IR-to-IR passes (single-call-site inlining,
//!   reachability pruning, the recursion gate).
//! - [`interp`] — the reference IR interpreter: the one executable definition of
//!   IR semantics, standing adversary in every backend's diff battery, and the
//!   semantic anchor for the family hash ("same cell, N bodies").
//! - [`descriptor`] — per-target compilation parameters ([`Target`] +
//!   [`TargetDescriptor`]): no backend may read a property of another backend;
//!   anything two backends need lives there.
//!
//! The cell *contract* layer (cartridge, manifest, capability policy, the family
//! hash field) stays in the `cell80` crate until WS-E generalises it per-target.

pub mod dce;
pub mod descriptor;
pub mod inline;
pub mod interp;
pub mod ir;

pub use descriptor::{ArithStrategy, Target, TargetDescriptor};
pub use interp::Interp;
pub use ir::Func;

/// Default code origin, family-wide default for the Z80 targets (a descriptor
/// value — see [`descriptor`]).
pub const ORG: u16 = 0x8000;
