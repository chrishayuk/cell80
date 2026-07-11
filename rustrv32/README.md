# rustrv32

The **RV32I(M) sibling backend** of the cell-family
([Phase 5 WS-B](../docs/13-multi-target-spec.md)) — deployment target: the Hazard3
cores in the RP2350, as the antweight robot's reflex organ.

- **`ins`** — the RV32 symbolic instruction layer (per-ISA `Ins`, shared
  *discipline* with rustz80's, not shared code) + an exact encoder, pinned by
  encoding goldens. The RISC-V Sail model joins as the emission adversary in CI.
- **`exec`** — the cycle-accounted RV32IM reference executor: RISC-V-spec-exact
  M semantics (division by zero returns all-ones/dividend, no trap; `MIN/-1`
  wraps), misalignment faults exactly as Hazard3 (no hardware misaligned support),
  and a **provisional** cycle table pinned by test until the RP2350 `mcycle`
  co-sign qualifies it.
- **`codegen`** — naive correct-first lowering of the
  [`cell80-core`](../cell80-core) IR: the family 2-byte slot ABI in a 64 KiB data
  window whose layout mirrors the reference interpreter's memory map byte for
  byte; native signed-32 (`slt`/`div`/`sra` — the ops backend zero gates).

Verified in rustz80's differential battery: every program runs against
release-mode rustc (the oracle), both Z80 targets, the IR interpreter, and this
executor. Demo: `cargo run -p rustz80 --example cell_family`.
