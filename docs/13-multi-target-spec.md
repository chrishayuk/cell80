# cell80 Multi-Target Extension — Spec v0.2

**Codename:** cell-family (targets: Z80 = backend zero, RV32I/Hazard3, Thumb-1/M0+)
**Status:** Accepted — supersedes the v0.1 draft after a repo-grounded review (2026-07-10)
**Depends on:** cell80/rustz80 as of `b3ec186` (Ins layer, cell contracts, diff harness, warm host)
**Deployment driver:** SOMA reflex organ on antweight bot (RP2350 primary, RP2040 secondary)

v0.2 revises v0.1 where the draft disagreed with the tree: WS-A is re-scoped as a
contract rewrite (not an extraction), the reference IR interpreter becomes a first-class
deliverable (v0.1's A3 acceptance referenced an oracle that does not exist), signed
arithmetic is re-specced as *widening* (i16 already shipped), the cell-layer work gets
its own workstream (WS-E), evaluation order is pinned, "no floats" becomes "no hardware
floats", and Q1–Q5 are resolved.

---

## 0. Thesis

cell80's claim — *verified execution over predicted execution* — is not a Z80 claim.
This extension proves it by making the cell contract (content-addressed,
capability-manifested, cycle-accounted, differentially-verified restricted-Rust cells)
portable across ISAs, with the Z80 backend becoming "backend zero" of a family rather
than the whole system. This is the roadmap's pre-registered "ISA attachment" clause
([roadmap-phases.md](roadmap-phases.md) non-goals) coming due: the Z80 is an
implementation detail behind the `.cell` ABI; the contract survives the chip.

**Success statement (the demo):** one restricted-Rust source cell, compiled through the
shared IR, producing three hash-attested artifacts — Z80, RV32I, Thumb-1 — that exhibit
provably identical behaviour on their reference executors, with per-target cycle
certificates, and with the RV32 artifact's timing co-signed by `mcycle` on real RP2350
silicon at 1kHz loop rate.

**Motivating workload:** the antweight MVP kernel — intent mixer, state estimator,
envelope projector — three straight-line Q-format cells in a 1kHz (target 4–8kHz)
control loop.

## 1. Non-goals (v0.2)

- **No LLVM anywhere in the deployed path.** rustc/LLVM is a differential adversary
  only: compiled artifacts are interrogated, never shipped.
- **No unified machine-instruction IR.** `Ins` stays per-ISA. The shared contract is the
  typed IR above it. Any abstraction that tries to span Z80 accumulator style and RV32
  register style at instruction level is presumed wrong until proven otherwise.
- **No optimising register allocator in v0.2.** Naive correct-first codegen (memory-slot
  register file + working registers), the strategy that shipped rustz80. Real allocation
  over 32 registers is a later, benchmarked upgrade.
- **No *hardware* floats.** *(revised from v0.1's "no floats": the F-waves already
  shipped an owned f32 softfloat surface — the arithmetic five + comparisons,
  bit-identical to rustc, resident kernel bank.)* The softfloat kernels are dialect
  source compiled through the same pipeline, so they port to new backends by
  recompilation; what stays forbidden is any FPU (RV32F, M33 FPU) or any arithmetic we
  don't own. Hazard3 has no FPU; the rule costs nothing on the primary target. The robo
  cells themselves are Q-format integer (§WS-C) — softfloat is *permitted* on new
  targets, not *required* by M2.
- **No RTOS, no heap, no interrupts inside cells.** Static schedule, SRAM-resident,
  single-core (second Hazard3 core parked in v0.2).

## 2. Architecture

### 2.1 Target descriptor (the keystone — new work, not extraction)

A per-target data structure owning everything currently implicit in Z80 codegen. Review
finding: nothing like this exists today — the entire multi-target surface is
`enum Target { Spectrum48, Cell }` (`rustz80/src/codegen/mod.rs`) with inline
`match a.target` forks, and the IR's *contract* (not its imports — `ir.rs` is
import-clean) bakes in 16-bit widths, the HL/DE/BC 3-register ABI, 2-byte slots, and
`ED FE` trap semantics documented on IR nodes. The descriptor is therefore a **contract
rewrite** of `ir.rs` + `codegen/mod.rs::emit_func` + `Imm::Slot` + `lib.rs`
(`ORG`/`FieldLayout`/`Signature`), gated by the existing golden.

| Field | Z80 (backend zero) | RV32I/Hazard3 | Thumb-1/M0+ |
|---|---|---|---|
| Native word | 16-bit (u32 as HL:DE pair) | 32-bit | 32-bit |
| Register file | RAM scratch slots + HL/DE/BC | x1–x31 (v0.2: memory slots + 2–3 working regs) | r0–r7 low (+ high, restricted) |
| Calling convention | ≤3 args in HL/DE/BC | a0–a2 (mirror ≤3-arg rule) | r0–r2 |
| Return / tuples | HL / HL+DE+BC | a0 / a0–a2 | r0 / r0–r2 |
| Trap mechanism | ED FE prefix | `ecall` (executor) / forbidden on HW | `bkpt` (executor) / forbidden on HW |
| Divide-by-zero | per-target today *by design*: Spectrum48 saturates, Cell halts (image flag) | HW `div` by 0 = all-ones, **no trap** (matches legacy `Saturate`); executor policy row required | owned kernel decides; policy row required |
| Memory discipline | ORG 0x8000, locals above code | SRAM-resident, linker-enforced; XIP forbidden for cells | SRAM-resident; XIP-cache explicitly forbidden |
| Kernel bank | BANK_ORG 0xC000 / BANK_SCRATCH 0xB800, SHA-256-pinned (.cell v9) | per-target ORG + per-target bank image ⇒ per-target pin | same |
| Cycle model source | Emulator T-states (measured; certificates use **Spectrum48** target — see §4) | Hazard3 Verilog-derived table (static), qualified on RP2350 SRAM timing | ARM M0+ TRM cycle table (static) |
| Formal oracle | ZEXALL-heritage emulator + rustc diff | **RISC-V Sail model** | ARM machine-readable pseudocode / QEMU-Unicorn diff |

**Rule:** no backend may read a property of another backend. Anything two backends need
lives in the descriptor. The Z80 backend must be refactored *onto* the descriptor (the
assumption-flushing step, deliberately first). The existing Spectrum48/Cell fork
becomes two descriptor instances (or one descriptor + an arithmetic-strategy field),
proving the mechanism on backend zero before any new ISA exists.

### 2.1a Backends, cores, platforms — three layers, not one (amendment, 2026-07-11)

The flat table above conflates three things the family must keep separate, or every
board becomes a new compiler:

```
cell source
    ↓
ISA backend         rustz80 / rustrv32 / rustthumb (v6-M)      — one compiler per ISA
    ↓
core timing model   Z80 / Hazard3 / Cortex-M0+ / Cortex-M33    — one cycle table per core
    ↓
platform            Spectrum48 / RP2350 / RP2040 / STM32G0 …   — linker layout, SRAM rules,
                                                                  peripherals, the co-sign rig
```

A **certified target** is the full triple, named as such —
`rv32im-hazard3-rp2350-sram`, `thumbv6m-cortexm0plus-rp2040-sram`,
`thumbv6m-cortexm0plus-stm32g031-sram` — because identical instructions do not imply
identical timing across cores or buses: pipeline and bus behaviour are part of the
claim. The same silicon can host two certified targets (RP2350 boots either Hazard3
or Cortex-M33: the controlled same-chip ISA experiment — same SRAM, clock,
peripherals, cell, inputs; only the architecture differs). ISA backends may grow
**profiles** rather than siblings where the ISA itself is parameterised: `rustrv32`
carries `rv32im` today and earns `rv32i` (owned mul/div kernels) and `rv32e`
(x0–x15 — the CH32V003-class 2 KiB-SRAM proof) as descriptor-selected profiles, not
new crates.

**Hardware ladder (post-B4 priority):** RP2040/M0+ (the genuinely different ISA —
WS-D as specced), then RP2350-M33 (the same-silicon comparison), then CH32V003/rv32e
(the minimality proof). Connected deployment (ESP32-C3-class) splits into
*connected mode* (deterministic behaviour, uncertified timing) vs *certified mode*
(SRAM-resident, radio and unrelated interrupts quiesced, bounded window) — the same
posture §6 risk 3 pre-registers for RP2350. Open-RTL cores (PicoRV32, SERV) are the
eventual research targets: RTL simulation as one more adversary, and the SERV-fabric
shape (one tiny isolated core per reflex organ) is the SOMA model in silicon.
Linux/Wasm/eBPF distribution bodies stay out until the embedded family is convincing
(§1's non-goal discipline).

### 2.2 Typed IR changes

1. **Width generalisation — the heaviest WS-A item.** Today `Width` is a closed
   `{Byte, Word, SWord, DWord, F32}` ladder with split `Lit(u16)`/`Lit32(u32)` node
   variants and u16-default semantics ("Stage 0 is u16 throughout"). Widths become
   explicit per-value with target-independent semantics; `extend`/`truncate`/
   `sign_extend` become IR ops (today: `Trunc`, `Trunc32`, zero-extend `Widen`, and
   **no sign-extend anywhere** — `i16 as u32` is a deliberate compile error). Z80 keeps
   16-bit-natural lowering; RV32/Thumb get 32-bit-natural lowering with u8/u16 as masked
   cases.
   *Landed (A2b, 2026-07-10) with two recorded decisions.* (a) **The node-family
   split stays**: the 16/32-bit sibling nodes *are* the width explicitness (every
   node's width is statically known without an IR type checker); merging them into
   width-parameterised nodes is deferred until WS-B/WS-D supply a second backend's
   evidence — the §1 rule that an abstraction fitting only backend zero hasn't earned
   it. What landed: `SignExtend` completes the explicit bridge family, `i16 as u32`
   unfreezes (rustc semantics, width-stress diff corpus), and the IR module doc now
   *specifies* the target-independent contract. (b) **The 2-byte slot ABI is
   family-wide**: locals, elements, and fields stay 2-byte little-endian slots on
   every target (it is the frozen `StateCell`/manifest ABI, and it keeps manifests
   and memory images portable) — a wider-word backend loads 2-byte slots and computes
   at native width with wrap-at-width masking; it does not get a wider slot.
2. **Signed *widening*, not signed introduction.** *(revised: v0.1 specced signed from
   scratch; `i16` is fully shipped on Z80 — `Width::SWord`, S⊕V compare, `__sdivmod16`,
   arithmetic `>>`, sign-boundary diff tests.)* The critical-path work is **i32** (and
   i8 if profiling wants it) + `sign_extend` as a first-class IR op — which unfreezes a
   deliberate dialect rejection, so the diff corpus must add exactly the sign-boundary
   cases the current dialect was designed to dodge. RV32/Thumb get signed natively
   (`slt`/`blt`, signed condition codes); Z80 i32 codegen may lag indefinitely (the robo
   targets don't need it there).
3. **Evaluation order is pinned left-to-right in the IR.** *(new — v0.1 omission.)*
   Today the right operand of `-`, `/`, `%`, and 16-bit `*` evaluates first — an
   accumulator-scheme artifact, observable with effectful operands
   ([10-dialect-semantics.md](10-dialect-semantics.md) §Evaluation order). Cross-target
   equivalence (§4) cannot hold with an unpinned order. Decision: **canonicalize to
   left-to-right in lowering** and let Z80 pay a temp where it must; the resulting Z80
   golden churn is a pre-registered, reviewed break (§5 M0 note), not a regression.
   Freezing a Z80 quirk into every future backend was the worse trade.
4. **Wide multiply.** IR op with (lo, hi) result. RV32M: `mul`/`mulh`. Thumb-1: `muls` +
   owned widening kernel. Z80: existing trap path (unchanged).
5. **Divide policy (contract-level):** divide is *not* a primitive in bounded cells.
   Options per cell manifest: (a) forbidden, (b) constant-time owned kernel
   (shift-subtract, fixed iterations), (c) unbounded-cell only. Hazard3's iterative
   divider is data-dependent timing — a contract violation, same rule as variable-time
   crypto. Divide-by-zero is a **descriptor row** (§2.1): the existing per-target
   divergence (Spectrum saturate / Cell halt) is already deliberate and flag-carried;
   RV32 hardware's all-ones-no-trap answer happens to equal the legacy `Saturate`
   convention, which the executor must reproduce exactly when policy = Saturate.

### 2.3 The reference IR interpreter (new deliverable)

*(v0.1's A3 acceptance cited "the reference interpreter" — no such thing exists; every
verification path today executes Z80 bytes on `z80::Cpu`.)* Build a direct executor for
the typed IR, backend-independent, living in `cell-core`. It earns its keep three ways:

- **The only way to check IR semantics before a backend exists** (signed widening lands
  against it, then backends catch up).
- **The semantic anchor for the family hash** (§2.6): "same cell, three bodies" is
  checkable against one executable definition, not three-way agreement alone.
- **A standing adversary in the verification matrix** (§4) — every target executor must
  agree with it on the shared vector corpus.

It is *not* a deployment target, has no cycle model, and stays deliberately naive.

### 2.4 Per-target Ins siblings

Each ISA gets its own symbolic instruction layer with its own peephole suite under the
shared peephole-testing **discipline** — per-rule fire counts, exact-byte shape tests,
behavioural diff per rule *(revised from v0.1's "shared framework": the review found the
existing framework inseparable from Z80 — `Ins` is Z80, rules argue via Z80 flags, shape
tests assert opcode bytes. What's shared is the discipline and the cycle-accounting
hooks, not code — budget WS-D accordingly).*

### 2.5 Repository layout (decided: monorepo, sibling compilers)

```
cell80/
  cell80-core/      # typed IR, IR interpreter, IR passes, target descriptors (landed, A5);
                    # cell contracts + family hash join at WS-E
  rustz80/          # backend zero: Z80 Ins + codegen (+ emulator glue)
  rustrv32/         # RV32I(M) Ins + codegen + reference executor
  rustthumb/        # Thumb-1 Ins + codegen + reference executor
  platforms/
    rp2350/         # crt0, SRAM linker script, mcycle co-sign harness (M33+Hazard3 board)
    rp2040/         # crt0, SRAM-not-XIP linker, timing harness (M0+ board)
  cells/            # shared target-independent cell corpus (already true today — dialect source)
  harness/          # diff adversaries (rustc, Sail, QEMU/Unicorn), fuzzers, CI
```

Rationale: the verification story is cross-compiler — descriptor/IR changes must land
atomically across backends with one CI run proving hash-stability everywhere; the family
hash needs a single home. WS-A's true deliverable is the extraction of `cell-core` out
of rustz80, with rustz80 becoming a consumer; the M0 hash-stability gate proves the
extraction changed nothing. New crates join the workspace members (fmt/clippy gates are
workspace-wide; `cell80-py` keeps its own workspace and has broken before on
`CartridgeOpts` changes — check it on every cell-layer signature change).

### 2.6 Identity: per-target artifact hashes + the family hash (Q4, resolved)

Today `artifact_hash()` = SHA-256(manifest ‖ compiled Z80 image) — the machine code is
load-bearing in the identity, so identity is intrinsically per-target. That stays. On
top:

- **The family hash** — SHA-256 over the **canonical source text** (M2.5 canon,
  Light mode; the precipitation property already makes structurally identical programs
  byte-identical). This is what "same cell, three bodies" formally means. If/when the IR
  interpreter stabilises a canonical IR serialization, the family hash may migrate to
  it; source-text SHA-256 is the pragmatic v0.2 anchor.
- The existing `source_hash: u64` (`DefaultHasher`) is a provenance/cache key and is
  **not identity-grade**; it is not promoted, the family hash is a new field.
- **Cartridge format:** one `.cell` per target (three cartridges), each carrying a new
  **target id** field (the v9 manifest has none — Z80 is currently implicit) + the
  family hash tying siblings together. One-cartridge-N-images was rejected: it bloats
  the loader and breaks the "verify exactly what you run" story.

## 3. Workstreams

### WS-A: cell-core + descriptor (do first, ~3 weeks — was 2; re-sized by the review)
- A1. Introduce the target descriptor; port Z80 codegen onto it (Spectrum48/Cell become
  descriptor instances). **Acceptance:** rustz80 suite green; `codegen_golden`
  byte-identical (the gate exists: 347 programs pinned as image hex).
- A2. Width generalisation + explicit extend/truncate/sign_extend ops + evaluation-order
  canonicalization. **Acceptance:** corpus hash-stable on Z80 *except* the
  pre-registered evaluation-order break (reviewed byte-delta, regenerated golden);
  new width-stress tests pass under rustc diff.
- A3. Signed widening (i32; sign_extend). **Acceptance:** signed corpus passes under
  rustc diff **on the IR interpreter (A4)** before any backend emits it.
  *Landed 2026-07-10:* `Width::SDWord` shares DWord storage; signedness travels as
  flags on `Bin32`/`Cmp32`/`Shift32` (only compare, `/`/`%`, and arithmetic `>>`
  differ — the i16 precedent). Scope: scalars, params/returns (the wide convention),
  arithmetic, comparisons, casts (`i16 as i32` sign-extends; `i32 ↔ u32` is a bit
  identity; i32/u32 never mix, as in rustc). Deferred with instructive rejections:
  i32 struct fields (WS-E owns the manifest `Ty` signedness), `[i32; N]` arrays,
  i32 consts, saturating/bit methods. The `reject_signed32` gate refuses signed-32
  ops at every codegen entry with a WS-B pointer; `check_ir!` is the
  interpreter-only harness leg; corpus in `tests/diff/signed32.rs`.
- A4. The reference IR interpreter (§2.3). **Acceptance:** agrees with the Z80 emulator
  across the existing diff battery on both Z80 targets.
- A5. `cell-core` crate extraction (ir + passes + interpreter + contracts + family
  hash). Mechanically easy (ir.rs has zero imports; z80 is dev-only) — the contract
  rewrite is A1/A2, not this move.
  *Landed 2026-07-11 as **`cell80-core`*** (crates.io-ready, dependency-free): the
  typed IR, inline/DCE, the interpreter *engine* (neutral `(name, bytes)` const
  pool — no lowering types cross the boundary), and the descriptors + `Target`.
  rustz80 consumes it behind root re-exports, so `crate::ir`-style paths and the
  public API are unchanged; the syn/lowering interp entries stay in rustz80. The
  cell *contract* layer (cartridge, manifest, capability policy, family-hash
  field) deliberately stays in `cell80` until WS-E generalises it per-target —
  extracting it now would just move Z80-shaped state addresses into a crate named
  "core". CI publishes `cell80-z80 → cell80-core → rustz80 → cell80`.

### WS-B: RV32 backend + executor (critical path, ~4–5 weeks)
- B1. RV32I codegen, naive strategy (memory-slot regfile). Emission tests against
  **Sail** from the first instruction.
  *Slices 1–2 landed 2026-07-11 (`02ef7d0`, `ee4e17f`):* the RV32 `Ins` layer +
  exact encoder (encoding goldens local; **Sail CI job still owed** — the risk-2
  budget), and full codegen over the cell80-core IR: the family 2-byte slot ABI in
  a 64 KiB window mirroring the interpreter's map, mask-at-every-op width
  discipline, native signed-32 (the ops rustz80 gates), inline `__bits_*` kernels,
  alignment-safe byte pairs where packed addresses can be odd (the executor faults
  on misalignment like Hazard3, so the battery proves it).
- B1a. *Emission adversary landed 2026-07-11:* **GNU gas** (binutils' RISC-V
  assembler — a fully independent implementation) re-encodes every instruction
  shape, immediate edge, and label-resolved displacement, compared byte-for-byte
  against the encoder (`rustrv32/tests/gas_adversary.rs`; teeth proven by a
  deliberate funct7 corruption). Linux CI installs the cross binutils and sets
  the adversary *required* — it can skip locally, never on the gate. **Still
  owed:** the Sail model (or spike, the risk-2 fallback) as the *execution*
  adversary — gas checks what bytes mean to an assembler, not what they do; and
  the RV32 peephole suite (its own rules under the shared discipline).
- B2. RV32 reference executor: RV32IM interpreter, cycle-accounted from the Hazard3
  model **as qualified on RP2350 SRAM** (fetch/load-to-use timing is a platform
  property, not a core property), differentially tested against Sail. Determinism
  fuzzing (rerun / fresh instance / image-roundtrip) — extend `cell_fuzz.rs`'s
  `Snapshot` discipline.
  *Landed in part 2026-07-11 (`02ef7d0`):* the executor with RISC-V-exact M
  semantics (div-by-zero all-ones/dividend, MIN/-1 wrap), misalignment faults
  (Hazard3 truth), determinism fingerprint tests, and an explicitly **provisional**
  cycle table pinned by test (gcd = 160 cycles, hand-verified) — the numbers await
  the B4 co-sign. Owed: Sail differential, the full fuzz battery.
- B3. rustc adversary wiring: extend the single-source `check!` matrix
  (`rustz80/tests/diff/harness.rs` `TARGETS`) — same block under host rustc (oracle),
  Z80 targets, and the RV32 executor.
  *Landed 2026-07-11 (`ee4e17f`), ahead of schedule:* `check!`/`check_str!`/
  `check_ir!` each carry an RV32 leg — every battery program agrees across rustc,
  both Z80 targets, the IR interpreter, and the RV32 executor (195/195), and the
  signed-32 corpus runs natively on a machine backend for the first time. This
  exceeds M1's "first gcd-class cell" gate. *Completed 2026-07-11 (`09fc6eb`):*
  the `run_program*`/`run_program_regs`/`run_program_pruned` legs landed
  (`Lowered::const_data()` carries the pool), and `run_to_memory` compares the
  RV32 data window against the reference interpreter's image **byte for byte,
  unmasked** — one memory map, no execution substrate inside either. Per-file
  coverage ≥90% across both new crates.
- B4. RP2350 bring-up: crt0, linker script enforcing SRAM residency for cells, one
  Hazard3 core active, `mcycle` harness. **Acceptance (the co-sign):** executor cycle
  prediction vs silicon `mcycle` agreement within a documented bound (target: exact for
  straight-line cells; any divergence is a filed finding, not a shrug).

### WS-C: Robo dialect (~3 weeks, overlaps WS-B tail)
- C1. **Q-format library.** *(revised: Q8.8 is live today — `//! scale:` manifest
  header, `q_mul`/`q_div` at scale 8, the i16 fixed-point idiom.)* **Gate: Q8.8 on
  existing machinery.** **Stretch: Q16.16**, which requires i32 (A3) *and* relaxed
  wide-value call rules (Q1 note below) — if i32-across-calls slips, M3 ships on Q8.8
  and Q16.16 moves to M4. Property tests vs rustc f64 reference within documented
  ULP-style bounds (bounds owned, not inherited).
- C2. **WCET sub-dialect:** cell-manifest flag `bounded`. Bounded cells admit
  straight-line code + `for` over compile-time-constant ranges only; `while` requires a
  `#[bound(N)]`-style annotation checked at IR level. Compositional static cycle bound
  emitted into the manifest — **new machinery** (today cycles are emulated-only;
  nothing static exists). Divide per §2.2.5.
- C3. **Sensor typing:** inputs carry explicit staleness/validity fields; non-finite /
  out-of-range at boundaries is typed escalation, never silent propagation (the
  `finite_result`/escalation-band pattern restated for the sensor edge).

### WS-D: Thumb-1 sibling (deliberately lagging, ~3–4 weeks)
- D1. Thumb-1 codegen (naive strategy; low-register discipline; owned
  soft-div/widening-mul kernels).
- D2. M0+ reference executor (~56 instructions), cycle table from ARM TRM; diff vs
  QEMU/Unicorn as adversary.
- D3. RP2040 bring-up; SRAM-not-XIP enforced by linker; cycle co-sign via
  SysTick/DWT-equivalent.
- **Purpose beyond deployment:** second concurrent target forces descriptor honesty;
  Z80 serves as adversarial third (any abstraction that still fits backend zero has
  earned it).

### WS-E: Cell-layer multi-target (new — unowned in v0.1; after M2, before M4)
The cell contract lives in the **cell80** crate, not rustz80, and no v0.1 workstream
owned it:
- E1. Manifest target id + family hash fields (`.cell` v10); loader refuses a
  target-id mismatch the way it refuses a bank-pin mismatch today.
  *Landed 2026-07-11:* v10 carries `target` (the machine-body family — `z80-cell`
  for every cartridge this crate makes) and `family_hash` (SHA-256 over the same
  canonical text the u64 source-hash digests — identity-grade, sibling bodies
  share it). The loader refuses a foreign body up front, naming both; pre-v10
  cartridges read back as `z80-cell` with no family hash. Both fields sit inside
  the artifact-hash-covered prefix, so they are part of each body's identity.
- E2. Per-target memory-map story for `state_addrs` (today: u16 byte addresses at
  `STATE_BASE = 0xB000`, 2-byte slots, baked into `CellHost::run_state` and the
  register-probe router). Decision deferred to E-design: per-target address tables vs
  a name+type-only manifest with layout resolved per target.
- E3. Host/runner generalisation (a `Runner` per executor behind one host surface).
- **M2 explicitly does *not* wait for WS-E:** the demo runs RV32 through a thin
  `rv32 exec` harness that bypasses `CellHost`. WS-E is what makes the family a
  *product* rather than a demo.

## 4. Verification matrix (per cell, per release)

| Check | Mechanism |
|---|---|
| Behavioural correctness | rustc-adversary differential (all targets, via executors; single-source `check!` pattern) |
| IR semantics | **reference IR interpreter** (A4) — every executor agrees with it on the shared vector corpus |
| ISA-semantics correctness | Sail (RV32) / ARM pseudocode-QEMU (Thumb-1) / ZEXALL-heritage emulator (Z80) |
| Determinism | rerun / fresh-instance / image-roundtrip / fast-vs-authentic fuzz (`Snapshot` fingerprint) |
| Timing | static bound (bounded cells) + executor cycle count + silicon co-sign (`mcycle`). **Z80 certificates use the Spectrum48 target** — Cell-target `cycles` charges traps ~4 T-states and is documented as not hardware-faithful; it must never appear beside an `mcycle` co-sign. |
| Identity | per-target machine-code hash (= cell identity, as today) + **family hash** (§2.6); manifest signed |
| Cross-target equivalence | same IR, N-target executors + IR interpreter, identical observable behaviour on shared vector corpus (evaluation order pinned, §2.2.3) |

## 5. Milestones

- **M0 (wk 2–3):** descriptor refactor complete, Z80 hash-stable. *Kill criterion: if
  Z80 cannot be made hash-stable on the descriptor without semantic change, the
  descriptor design is wrong — redesign before proceeding.* **Golden-break policy
  (pre-registered so the gate keeps its teeth):* A1 and A5 admit **zero** byte deltas;
  A2's evaluation-order canonicalization is the *only* sanctioned break, lands as its
  own commit with the reviewed byte-delta, and any other regeneration during WS-A is a
  kill-criterion event, not a shrug.*
- **M1 (wk 4–5):** width + signed-widening IR landed against the IR interpreter; first
  RV32I cell (gcd-class) behaviourally identical to rustc on the executor.
- **M2 (wk 7–8):** RP2350 runs a hash-attested cell; `mcycle` co-signs the executor's
  number. **First public demo:** same-source cell on Spectrum and Pico 2, same answers,
  two cycle certificates (Z80 side: Spectrum48 target).
- **M3 (wk 9–10):** Q-format + WCET sub-dialect; the three MVP robo cells compile as
  `bounded`, static WCET ≤ 50µs each @150MHz documented in manifests. 1kHz HIL loop
  runs from synthetic sensor streams. Q8.8 is the gate; Q16.16 per WS-C1.
- **M4 (wk 13–15):** Thumb-1 sibling at M2-equivalence; WS-E landed; three-ISA demo
  (Z80 + RV32 + Thumb-1, one family hash, three certificates).
- **M5:** robot MVP integration (separate spec: kernel contract / bot MVP doc).

## 6. Risks & pre-registered responses

1. **Hidden Z80 assumptions surface late.** Mitigation: WS-A first, hash-stability gate
   at M0, and the golden-break policy above. The known ones are now written down
   (§2.1/§2.2); the risk is the unknown ones.
2. **Sail toolchain friction.** Budget: one lost weekend *plus CI wiring*; Sail runs as
   a **linux-only CI job** (the 3-OS matrix would rot on Windows). Fallback oracle:
   spike (riscv-isa-sim) with Sail deferred to M4, finding filed either way.
3. **RP2350 timing nondeterminism (bus contention, XIP, second core).** v0.2 posture:
   one core, SRAM-only cells, peripherals quiesced during certified loop. Any residual
   `mcycle` variance is a first-class finding with its own investigation, not tolerated
   noise.
4. **Naive codegen too slow for 4–8kHz ambition.** Not a v0.2 risk at MVP cell sizes
   (budget ≈150k cycles/ms; cells ≈ thousands); if profiling disproves this, register
   allocation moves forward in priority — benchmarked, not assumed.
5. **Scope creep from the robot.** Firewall: bite denial, acoustic ledger, FOC driving
   etc. are *cells to be written later against this platform*, not platform
   requirements. Platform ships at M3.
6. **i32 unfreezes deliberate dialect rejections** (`i16 as u32`, no sign-extend). The
   sign-boundary corners the dialect dodged become live; A3's corpus must target them
   explicitly, and the IR interpreter (A4) is the pre-backend safety net.

## 7. Resolved questions (were Q1–Q5)

- **Q1 — calling convention: keep ≤3-arg family-wide.** Uniform *source-level* rules
  are what keep one corpus compiling everywhere; per-target lifts fracture "same cell,
  three bodies". The sharper sub-question v0.1 missed is the **wide-value rules**
  (Tier-2: one u32, first position; the two-u32 gcd convention) — also kept uniform for
  now; Q16.16's appetite for free i32 flow is WS-C1's stretch trigger for revisiting,
  as a family-wide relaxation or not at all.
- **Q2 — struct-in/struct-out is *the* reflex cell ABI.** `StateCell`/`state_addrs`/
  by-name binding already is that pattern, differentially verified
  (`struct_field_state_matches_host`); WS-E1's manifest revision freezes it formally.
- **Q3 — traps on deployed hardware: forbidden in bounded cells.** Escalation is a
  *returned code* (the `0xFF00–0xFFFF` band + `finite_result` pattern) — typed,
  WCET-accounted for free because it is just a return path. Spectrum48 already proves
  the no-trap-surface posture works.
- **Q4 — yes: per-target artifact hashes + the family hash.** Design in §2.6
  (SHA-256 over canonical source; the u64 `source_hash` is not promoted).
- **Q5 — one cell80 brand with targets.** The identity model (one family hash, N
  per-target bodies) argues against separate cellV/cellT brands; the monorepo decision
  already made this call implicitly. "cellV"/"cellT" survive as informal target
  nicknames only.
