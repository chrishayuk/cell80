# cell80-core

The **target-independent compiler core** of the cell-family
([Phase 5](../docs/13-multi-target-spec.md)): everything shared between backends,
with nothing of any backend inside.

- **`ir`** — the typed IR and its semantic contract: widths explicit per value, the
  family-wide 2-byte little-endian slot ABI, explicit width bridges
  (truncate / zero-extend / sign-extend), left-to-right observable evaluation order.
- **`inline` / `dce`** — the IR-to-IR passes (single-call-site inlining,
  reachability pruning, the recursion gate).
- **`interp`** — the **reference IR interpreter**: the one executable definition of
  IR semantics. Every backend's diff battery runs against it, beside release-mode
  rustc.
- **`descriptor`** — per-target compilation parameters (`Target`,
  `TargetDescriptor`). The rule: no backend may read a property of another backend;
  anything two backends need lives here.

Deliberately dependency-free. Backends: [`rustz80`](../rustz80) (backend zero),
[`rustrv32`](../rustrv32) (RV32I(M), Hazard3/RP2350).
