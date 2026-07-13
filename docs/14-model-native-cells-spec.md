# 14 — Model-Native Cells (draft v0.1)

**Status:** draft for review · **Depends on:** cell80-core (A1–A5), WS-B pattern, fingerprints + admission gate, cell80-py · **External deps:** LARQL/Lazarus residual-stream injection, Gemma arithmetic-circuit localisation

## 0. Position

The model provides judgment; the cells provide guarantees; the interface between them is fast enough to sit inside the model's own thought. This spec covers the programme that makes that literal: a GPU backend for cells (WS-E), retrieval by execution (WS-F), decode-time and residual-stream wiring (WS-G), trained invocation (WS-H), and the circuit-prosthetic experiment (WS-I).

**Non-goals.** This is not a general sandbox and does not compete with code-mode execution for arbitrary workloads. No tolerance of float nondeterminism is introduced anywhere: a GPU result that does not agree bit-for-bit with the reference interpreter is a defect, never a "GPU difference."

**One structural note promoted here from WS-E design work:** the canonical abstract cost of a cell becomes **IR steps as counted by the reference interpreter**. Z80 T-states, RV32 cycles, and GPU wall-time are per-target refinements recorded in the descriptor. This makes cost target-honest before a target with no cycle notion joins the family.

## 1. What the tree already provides

- **cell80-core:** typed IR with explicit widths, family slot ABI, target descriptor, reference interpreter as the one executable definition of IR semantics. A GPU backend is a descriptor + codegen over this seam — the same shape rustrv32 just proved lands cleanly.
- **Differential battery:** the five-system pattern extends to a sixth witness unchanged for integer cells.
- **Fingerprints + admission gate:** proven to catch behavioural duplicates that per-candidate verification misses (checkpoint 17). Reused as the key space for behavioural routing.
- **cell80-py:** in-process PyO3 path; no IPC between a sampler loop and a cell run.
- **External:** Lazarus injection machinery (residual-stream read/inject at a chosen layer); prior localisation of arithmetic circuits in Gemma 3 4B.

## 2. Workstreams

### WS-E: GPU target family (~2 weeks to E3 on Metal; CUDA before WS-H3)

WS-E is target-parameterised from the start, WS-B-style: one IR, one interpreter oracle, N GPU bodies, each with its own descriptor. The target matrix, with each target's *role* rather than just its status:

| target | role | priority |
|---|---|---|
| **Metal (MSL)** | dev loop on M3; LARQL/unified-memory path for G1/G3 | first — spike target |
| **CUDA (PTX or NVRTC)** | serving + training hardware; required for H3 batch rewards and G3 residency on NVIDIA | second — **critical path for WS-H3**, not portability |
| **WGSL (WebGPU)** | portable body; browser demo of retrieval-by-execution (the public artifact) | third |
| **ROCm/HIP** | deferred until a machine exists to gate it on; no untestable claims | parked |

- **E1 — Straight-line integer cells, per target.** IR→{MSL, CUDA, WGSL} codegen for loop-free cells: the robo family shapes (deadband, clamp, scale). Equivalence discipline: typed outputs + slot-file readback vs the interpreter. *Pre-registered weakening:* this is not `run_to_memory`'s raw image diff — the 64 KiB mirrored window is a per-target choice, not an IR requirement; the interpreter oracle covers the gap. **Gate (per target):** bit-exact agreement on every admitted straight-line integer cell × 10⁶ random inputs.
- **E2 — Loops and branches.** Budget-bounded iteration on GPU; measure warp/subgroup divergence on data-dependent loop counts. Working hypothesis to confirm: WCET-friendly ≈ SIMT-friendly (branch-light, fixed-iteration cells diverge least).
- **E3 — Batch megakernel.** Two layouts: one cell × N inputs (fuzzing, reward organs) and library × probe-set (retrieval). Throughput targets (to be benchmarked, not assumed): ≥10⁸ evals/s on M3; the CUDA figure is recorded on the first available card and entered in its descriptor, never extrapolated from Metal.
- **E4 — f32 kernel bank, per target.** Strict IEEE, contraction forbidden at codegen (`-fmad=false` / PTX without fused ops on CUDA; equivalent pinning on MSL and WGSL); per-thread execution is sequential so reduction-order nondeterminism cannot arise. **Gate:** bit-exact vs the CPU f32 reference on the full kernel-bank battery, per vendor.
- **E5 — CUDA residency.** Persistent megakernel + CUDA Graphs capture so a G3 call is a graph node, not a host callback; route→run→inject stays device-side end to end. This is what retires R5 on NVIDIA hardware.
- **E6 — Cross-target battery.** The N-body discipline extended: every admitted cell runs on interpreter + all live GPU targets with identical inputs; any pairwise byte disagreement is a filed defect, and the family hash attests the GPU bodies too.

### WS-F: Retrieval by execution (~1–2 weeks, after E3)

- **F1 — Probe protocol.** Extend the fingerprint probe set into a routing probe set; a query with I/O examples is answered by executing the *entire library* against the probes in one launch (395 cells × 8 probes ≈ 3k evals; still ~ms at 10⁶ cells). Scoring: exact match, ranked by match count then IR-step cost.
- **F2 — Hybrid router.** Behavioural rank where probes exist; TF-IDF remains for pure-text queries and as the candidate gate. Wire into cell80-mcp behind `cell_route_by_example`.
- **Precondition (owed before F2 is scored):** grow the adversarial retrieval split to n ≥ 100; at n = 36 the kill-gate's most safety-relevant category has coin-flip power.
- **Falsifiable gate:** on paraphrase cases equipped with probe pairs, P@1 ≥ 0.80 (text baseline: 0.389). If behavioural routing does not clear this by a wide margin, the central thesis of this spec is wrong and WS-H should not proceed on fingerprint-keyed routing.

### WS-G: Native wiring (G1 ~1 week; G3 is research)

- **G1 — Decode-time, in-process.** Sampler-loop integration (LARQL first; Apple unified memory makes host-resident cells nearly free to reach, so GPU residency is *not* required for G1). Structured call-span grammar; constrained decoding restricted to the known hash vocabulary — an unknown hash is a trap, counted, never a guess. **Gate:** end-to-end call (parse → route → run → splice) ≤ 1% of per-token latency at 30 tok/s.
- **G2 — Verified decoding.** Every emitted numeric span in scoped contexts re-derived by a cell before commit; disagreement forces re-sampling. Measured as hallucinated-arithmetic rate before/after.
- **G3 — Residual-stream injection (Lazarus pattern).** Routing head reads layer-L residual, cell computes, result injects back. On CUDA serving hardware this requires WS-E residency to avoid PCIe round trips (~10 µs each way); on Apple Silicon it does not. Gate is a measured behaviour delta on arithmetic tasks, not an architecture milestone.

### WS-H: Trained invocation (~3–4 weeks; the core of the programme)

- **H1 — Data factory.** Toolformer-style self-labelling with an *exact* oracle: sample contexts (chuk-math-gym, GSM8K-style, synthetic templates over library families), propose cell calls, execute (deterministic, µs), keep the call iff it strictly improves the continuation under the pre-registered criterion. Volume target: ≥10⁶ filtered examples; GPU batch scoring via E3. Degenerate-call defence: admission-style dedup on call sites + strict-improvement filter (R4).
- **H2 — SFT.** Small model (see Q4) with cell identities as stable vocabulary bound to content-addressed hashes. **Gate:** the trained model beats the prompted `cell_solve` baseline at equal parameter count on a held-out set, including held-out *families* (generalisation of the calling reflex, not memorised call sites). Shortcut rate (model computes trivial cases itself) is measured and given a pre-registered acceptable band — the adoption eval already showed trivial-case shortcutting is rational, not a failure.
- **H3 — RLVR with cell reward organs.** Rewards computed by cells on-GPU across full rollout batches; `trapped_ops` enters the reward as a penalty term so the policy cannot farm host traps. **Demonstration owed:** one bitwise-reproducible training run — same seed, same cells, same gradients, replayed on a second machine.

### WS-I: Circuit prosthetic (research, parallel; the headline)

- **I1 — Baseline.** Re-establish the localised arithmetic circuit in Gemma 3 4B; ablation accuracy floor recorded.
- **I2 — Transplant.** Ablate the circuit; escalate operands from the residual stream to a cell; inject the result. **Falsifiable claim, pre-registered:** post-surgery accuracy on the cell-covered operand range ≥ 99% (from the ablated floor), with regression ≤ ε on a held-out unrelated-capability battery. Either outcome is a finding; a partial outcome (accuracy recovers but operand extraction is the bottleneck) is the expected hard mode and gets filed, not hidden.

## 3. Sequencing

E1+F1 spike on Metal (a weekend) → E2/E3 → **F2 gate** → G1 → H1/H2 → **CUDA E1–E3** → H3, with E5, G3, WGSL, and I as parallel tracks. The F2 gate is deliberately load-bearing: it is the cheapest place to falsify the routing thesis before training spend. CUDA is sequenced immediately before H3 because that is where it is actually needed — batch reward scoring on training hardware — not earlier out of completeness.

## 4. Risks (pre-registered)

- **R1 — GPU integer semantics.** Shift-by-≥width and similar corners differ across shading languages; the IR defines the semantics and codegen masks to enforce them. The E1 battery exists to catch exactly this.
- **R2 — Routing becomes the bottleneck.** The paraphrase problem reappears one level down as head accuracy. Mitigation: F2 gate precedes H; fingerprint space, not text, is the key space.
- **R3 — Shortcutting.** Known from the adoption eval. Pre-register the band; train on contexts where verification is required, not where arithmetic is trivial.
- **R4 — Degenerate self-labelled data.** The model learns to emit no-op or self-confirming calls. Strict continuation-improvement filter + call-site dedup; audit a sample by hand each batch, first-workflow-batch style.
- **R5 — PCIe erosion (CUDA path).** G3 on CUDA without residency loses the latency argument. Sequence Apple-unified-memory first; E5 (persistent kernel + CUDA Graphs) is the retirement condition, and no NVIDIA latency claim is made before it lands.
- **R6 — f32 cross-vendor drift.** Bit-exactness is gated per vendor (E4) and *between* vendors (E6). If a vendor's toolchain cannot be pinned to unfused strict-IEEE (driver recompilation, WGSL implementation variance), that target's f32 support is declared unsupported in its descriptor rather than approximately supported — integer cells remain full-family.
- **R8 — CUDA toolchain nondeterminism.** NVRTC output can vary across CUDA versions; if kernel bytes enter any hash, pin the toolchain version in the descriptor or emit PTX directly (Q6). Behavioural equivalence (E6) is the invariant either way; byte-stable kernels are a stretch goal, not a claim.
- **R7 — Hash hallucination.** Constrained decoding over the known-hash vocabulary; unknown hash = counted trap. The failure is visible in `trapped_ops`, not silent.

## 5. Open questions

- **Q1 — Tool identity in the vocabulary.** Raw hash tokens vs learned name embeddings vs nearest-neighbour lookup in fingerprint space. Leaning: hash-bound special tokens for H2 (stable, auditable), fingerprint-space routing revisited at G3.
- **Q2 — When does IR-step cost enter the descriptor?** Proposal: now, as the canonical family cost, with existing T-state/cycle numbers re-labelled as refinements — one reviewed regeneration, WS-A-style.
- **Q3 — Call syntax at decode time.** Special tokens vs structured span grammar. G1 decides empirically; whichever loses is recorded with the measurement.
- **Q4 — Which model.** Gemma 3 4B (mech-interp continuity, required for WS-I anyway) vs Granite (composer continuity with cell_solve). Leaning: Gemma for I, Granite for H2's baseline comparison — both, with the battery shared.
- **Q5 — Does the behavioural router replace or gate the TF-IDF index?** Proposal: gate-then-rank hybrid; text search is never removed (pure-text queries have no probes), but it stops being the ranking authority where probes exist.
- **Q6 — CUDA emission: direct PTX or CUDA C++ via NVRTC?** Direct PTX is the rustrv32-encoder move — exact control, auditable bytes, no toolchain in the trust story — but costs an encoder. NVRTC is pragmatic and fast to land but imports the compiler into the picture (R8). Leaning: NVRTC for the E1 spike with the toolchain version pinned in the descriptor; revisit PTX-direct if kernel bytes ever need to enter a hash.

## 6. Ledger

### 2026-07-13 — CUDA E1–E3 + interp backend: built, golden-locked, gate PRE-REGISTERED (WS-E CUDA slice)

**Shipped the CUDA backend** ("second — critical path for WS-H3", §2 target
matrix), on the sequencing this doc set (§3: CUDA E1–E3 before H3), as one
walker with two dialects: `rustmsl::codegen` grew a `Dialect { Msl, Cuda }`
seam (11 dialect points — headers, address-space qualifiers, `CELLFN`/div-
helper noinline attributes, `sx16`/`sx32`, window-helper prefixes,
`__bits_*` intrinsics over `__popc`/`__clz`/`__ffs`, and the kernel
signature with a grid-tail guard) while every tick placement, mask, guard,
and the whole kernel body stay shared text — a semantics fix lands in both
dialects by construction. The refactor was locked byte-identical first
(M0/M1: MSL emission goldens over a 27-snippet corpus + the interp kernel,
then the seam, then the CUDA arms). The interp backend's `KERNEL` split the
same way: one shared body, per-dialect headers, decoder constants
*generated* from the Rust-side opcodes (the "kept in lockstep" comment
deleted as a failure mode). Q6 resolved as leaned: **NVRTC** (cudarc 0.19.8,
dynamic-loading, driver API pinned `cuda-12060`), `--fmad=false` pinned now
so E4 inherits it (R8), `--gpu-architecture` queried from the device — the
gate always compiles on the box it runs on. `MslModule` → `GpuModule`
(carries its `Dialect`; executors refuse the wrong runtime typed), alias
kept. Executors: `CudaBatch` mirrors `GpuBatch` exactly; `CudaInterpBatch`
mirrors `InterpBatch` over the same `bytecode::pack`. The noinline dodge is
kept on CUDA **for uniformity, not by assumption** — the Metal miscompile is
not presumed to transfer; whatever NVRTC-specific quirks exist, the battery
below is what finds them, and fixes land in the Cuda arms only.

**The battery is shared, so the gates cannot drift**: the msl_battery
harness extracted to `cell80/tests/battery_common/` (discovery, schedules,
fanned-out oracle, transcript book, the battery loops themselves) behind a
`Backend` vtable; `cuda_battery.rs` is the same sweeps on `CudaBatch`. The
oracle transcripts are backend-independent by construction (key, src hash,
seed, oracle digest — all interpreter-side), so the CUDA gate *reads* the
book the Metal gate blessed and never writes it. `corners.rs` and a new
`interp_parity.rs` (first direct `InterpBatch` coverage outside examples)
run on whichever backend the build has.

**PRE-REGISTERED GATE (before any CUDA silicon runs this code).** On a
pinned cloud box (docs/16 runbook: CUDA 12.6 image, Ampere+, every
toolchain version recorded): (i) `rustmsl --features cuda` suite green —
corners, interp parity, goldens; (ii) the library battery, value + state +
fused megakernel, **bit-exact values + trap status + IR-step counts (+
final state bytes)** against `cell80_core::Interp`, same floors as Metal
(≥ 230 value, ≥ 300 state compiled), same named exclusions
(`STATE_OOW_DEFECTS`), same transcripts and seeds; (iii) the 10⁶-input
value and state gates. Any mismatch is a filed defect fixed in the CUDA
dialect arms or excluded by name with a reason — **the gate is never
weakened**. Throughput is recorded as measured on the card, never
extrapolated. Status: **built and golden-locked on macOS; unverified on
silicon until the docs/16 session appends its results entry here.**

**Pre-silicon addendum (same day): the CUDA text's semantics validated
end-to-end by CPU emulation.** The emitted CUDA source is plain C++ once a
small shim supplies the vocabulary (`rustmsl::cpu_emu`: attribute defines,
`blockIdx`/`threadIdx` as serial loop variables, `__popc`/`__clz`/`__ffs`
over host builtins — legitimate because every UB corner in the emitted text
is explicitly guarded, so the host compiler is a fair executor of its
semantics). Results, host clang 17 on the M-series box: the corner battery
(runtime shifts, signed div/rem MIN/-1, bit intrinsics, trap folding incl.
the full 10⁸-tick fuel burn, the do-while continue wrapper, typed state
incl. trap-point partial state) — all green, values + status + steps + state
bytes (`rustmsl/tests/cuda_text_semantics.rs`); the interp kernel's CUDA
text vs `cpu_run` — green; and the **full library** through
`cell80/tests/cuda_cpu_emu_battery.rs`: 249/249 value cells clean (243
digest-identical to the blessed oracle transcripts — steps included),
539/539 state cells clean (505 via transcript), and the 249-cell fused
megakernel with 0 disagreements — the fused-scale shape where Metal's
compiler bug lived. The Linux build (`x86_64-unknown-linux-gnu`,
`--all-targets`, both feature states) cross-checks clean, so the box
session cannot stall on a compile. **What remains unvalidated — and is
exactly what the docs/16 session proves: NVRTC acceptance and NVIDIA
codegen.** No CPU-emulation result is cited as silicon verification.

Owed after the gate: `library_launch_cost` CUDA port (the E3 fixed-cost
figure), `Body::Msl`/`Body::Cuda` cartridge variants + GPU `Target`
descriptor entries, E4 f32, E5 residency (persistent megakernel / CUDA
Graphs), E6 cross-target battery automation.

### 2026-07-12 — The interpreter backend: the megakernel's wall priced, and the fix that scales (WS-E/F)

**Priced the megakernel launch — and found a wall.** `compile_library`'s
library×probe dispatch was assumed cheap; it isn't. A **kernel-size cliff** at
~64→128 fused cells (~44× jump in dispatch time), count-driven (reversed-order
confirms it isn't cell identity) and **not** sync overhead (single-command-buffer
tiling, since reverted, didn't help; same-kernel ×N is ~30× cheaper than N
distinct kernels). The mechanism is per-kernel code growing with the library
(PSO re-specialization / occupancy — unprofiled, but bounds-over-mechanism the
rule survives): **kernel size must be constant in library size.** The current
library is already past the cliff — a full-library launch is ~30 ms, ~2×10⁶
evals/s, three orders under the one-cell peak. This is exactly the "library-launch
fixed cost" the README owed. (`cell80/examples/library_launch_cost.rs`.)

**Shipped the fix: `rustmsl::interp`, a fixed-size bytecode interpreter.** One MSL
kernel reads each cell's IR from a data buffer (per-cell offset table +
concatenated bytecode; one threadgroup per cell, probes across lanes), so adding
cells grows a *buffer*, not the kernel. No cliff — flat/no-cliff to 500k distinct
entries (153 MiB), shared≈distinct (memory bandwidth isn't a wall either), where
the compiled path cannot even build. Bit-identical to the reference interpreter,
values **and** IR-step counts, at **93% of value cells** (232/249): the width/
control subset + short-circuit logic + `halt` + full call inlining + the u16/u32
width classes + the `__bits_*` intrinsics. Step parity carried by emitted markers
at the walker's exact charge points (per statement, per loop-attempt, per
expression node), coalesced within basic blocks; the trap battery (fuel Δ=0, halt
code, div0, signed MIN÷-1) all parity-verified CPU+GPU. Unit-tested; a CPU
reference VM (`cpu_run`) is the portable oracle. This makes the **two-body
architecture** literal: compiled `GpuBatch` for single cell × N inputs (WS-H
reward organs, fuzzing — fastest per eval, its 3.7×10⁸ peak untouched),
interpreted `InterpBatch` for library × probe-set (WS-F — the only one that
*scales* in cell count). Handoff around 10²–10³ cells.

**WS-F gate: cleared, wide — with an honest timing correction.** Behavioural
routing (execute candidates against a query's I/O examples, rank by match) hits
**100% precision@1** on the real retrieval dataset (463/463 expected cells in the
tied-top rank) — against the 0.389 text baseline and the ≥0.80 gate. The central
thesis holds: behavioural routing is exact where text is a coin-flip. **But** on
today's 232-cell library the GPU *loses* to the scalar Runner loop — per-query
dispatch is fixed-overhead-bound (~2 probes/query), and batched fingerprinting at
DEFAULT_PROBES is ~16× slower (build + tiny grid). The interpreter backend is a
**scale play**: its home is exhaustive index-build fingerprinting and
synthesis-scale evaluation, not interactive per-query routing on a small library.
The **F2 hybrid** (fingerprint/text gate → execute top-k) stays load-bearing for
interactive latency. WS-F correction, measured at representative composition:
exhaustive execution ≈ 20 ns/eval, so 10⁶ cells × 8 probes ≈ ~160 ms —
index-build/refresh, not per-query interactive.

**Synthesis by execution — the same primitive, at the scale where the GPU wins.**
Library × probes *queries* a library; a candidate population × a target *evolves*
one. Program synthesis: a 4096-candidate population scored against a target in ONE
`InterpBatch` dispatch per generation (1.3×10⁸ evals/s), full-domain-verified
solve. The **library-growth engine** (the inliner *is* the composition engine — a
candidate that Calls cells linearizes into one program) grew a real cell
(`bit_length → reverse_bits`, full-domain 0/65536, novel), then a **wave** (85→93
cells, *compounding* — grown cells become building blocks, dedup gate rejecting
near-dupes). Independent-target discovery (targets specified by behaviour, not as
compositions) + **CEGIS** (grow the probe set with counterexamples until "matches
the probes" == "is the function", so every solve is full-domain-correct by
construction). The load-bearing lesson, applied to the synthesizer itself: **the
GPU makes evaluation free; search and generalization are the real problems, and
full-domain verification + the dedup gate are what separate a discovery from a
lucky probe-fit.** (`cell80/examples/gpu_{route,fingerprint,synth,grow,grow_wave,discover}.rs`.)

### 2026-07-11 — The step-cost offenders fixed, GPU-audited (step-budget amendment §3a)

**Shipped.** Six of the seven cells carrying ~99.9% of the oracle gate's bill
rewrote value-identically (closed forms and absorbing-state early exits — the
`pow_mod` prelude kernel the amendment proposed is withdrawn; the exact-halt
constraint and the actual cost causes decided differently, see the amendment
v0.2). The bill: **3.94×10¹² → 5.74×10¹¹ ticks (~7×)**; the blessing run
drops from ~70 minutes to ~10, and what remains is the honestly-declared
O(n) tail. Every rewrite was audited **old-vs-new on the GPU** — 300 k inputs
per cell, values + trap status bit-compared, steps deliberately excluded —
and the audit caught a real bug in the first attempt (`n > 4` admitting the
prime 5 into a compositeness shortcut) in seconds, before any test suite
could. The GPU auditing its own oracle's cost reduction is the
retrieval-by-execution machinery pointed at the library's maintenance.

### 2026-07-11 — Typed-state readback: the state cells join the GPU (WS-E follow-up)

**Shipped.** The biggest coverage block lands: state cells
(`impl X { fn run(&mut self) }`, two-thirds of the library) now run on Metal.
Each thread carries a private state window at `STATE_BASE` (0xB000) — loaded
from an input buffer, byte-routed through the same `rd8`/`wr8` emulation, and
written back after the run, **even on a trap** (the interpreter's memory at
the trap point is observable, and identical tick placement makes the mutation
point identical). A state cell's param 0 is the `&mut self` pointer
(`STATE_BASE`, not an input word); extra scalar params ride the input triple.
The battery drives **adversarial random state bytes** (any bit pattern is a
valid scalar field — arrays included, which the named-field surface can't
even set yet) and asserts values, status, steps, **and final state bytes**
per input. Result: **496 state cells eligible, 496 compiled, 496 bit-exact**
(five corner tests pin the shape: field roundtrip, state-dependent control
flow, array-field loops, u32 fields with extra args, and a mid-mutation trap
leaving partial state identically). GPU library coverage goes from 245 value
cells to **741 of 746** — everything except f32 (E4) and two filed defects.

**Found.** The adversarial-state battery immediately caught a real defect
class: two day-old sliding-window cells (`simple_moving_average`,
`weighted_moving_average`) index an array field by an **unmasked state field**
(`self.window[self.head]`) — under fuzzed state the write lands far outside
the declared struct. The interpreter's open 64 KiB absorbs it silently; the
GPU's typed window traps it (`STATUS_OOW`) — the stricter reading, and
arguably the correct family semantics: a state-derived wild write should be a
refusal, not an absorption. Skip-listed with a filed reason; the fix (mask
the index on read — free on the operational envelope) belongs to the
sliding-window pack. This is the "state-derived unbounded write" sibling of
the step-budget finding: adversarial batteries keep converting latent cell
assumptions into typed refusals.

**Owed.** Unchanged otherwise: the library-launch fixed cost, `Body::Msl` +
E6 attestation, f32 (E4), CUDA before H3 — and the two OOW cells' bounds fix.

### 2026-07-11 — Oracle transcripts: the gate's interpreter cost paid once (WS-E follow-up)

**Shipped.** The bit-exactness gate's wall clock was never the GPU (its full
share is < 1 s at the measured rate) — it was the reference interpreter
independently deriving every expected sextet. That verdict is deterministic
per `(cell source, input schedule)`, so it now memoizes as a **content-
addressed transcript digest** (docs 12's fact-file idea applied to the GPU
gate): `cell80/tests/golden/msl_oracle_transcripts.json` holds one SHA-256
per cell × schedule, keyed by the combined source hash. A hit turns grading
into GPU-run + digest compare — the 245-cell CI battery drops from ~66 s to
~12 s (all residual is Metal pipeline compiles), and the 10⁶ gate from its
72-minute live bill to under a minute. The transcript is a **cache, never an
authority**: a
miss, a changed cell, or any disagreement falls back to the live interpreter
(which also localizes the disagreeing inputs); `UPDATE_GOLDEN=1` re-blesses.
A deliberate interpreter-semantics change regenerates the file; the
always-live R1/E2 corner battery guards that seam on every push. The oracle
side also got honest parallelism first (grading fans out per input chunk
across all cores — a step-heavy cell no longer pins one core), so the
one-time blessing run is ~16× the old serial pace.

### 2026-07-11 — E2 + E3 on Metal: loops, IR-step parity, the megakernel (WS-E slices 2–3)

**Shipped.** Loops and branches lower to MSL (`while`/`loop`/`for`,
`break`/`continue` — a `for` body rides a `do…while(false)` wrapper so
`continue` reaches the induction step; MSL has no `goto`), budget-bounded by a
per-thread fuel counter that mirrors the interpreter's `tick()` placement
**exactly**: one step per statement, per expression node, per loop iteration.
Every thread reports its step count and the batteries assert **IR-step
parity** alongside value parity — Q2 is now operational, not notational: the
canonical family cost is metered identically on CPU and GPU, and a runaway
loop is a counted trap (`STATUS_FUEL`, same 100M budget) on both substrates,
never a hung dispatch. E3's two layouts share one kernel shape (grid =
cells × inputs, cell-major): `compile` emits the one-cell × N-inputs module
(fuzzing/reward organs), `compile_library` fuses the whole eligible library
into one translation unit (library × probe-set — retrieval by execution's
substrate). Eligible coverage doubled: E1's 173 straight-line cells plus the
loop cells (and the library grew under us all day — 245 integer value cells
at the final run), every one bit-exact on values, status, **and steps**. The
pre-registered gate ran clean over the widened fragment: **245 cells × 10⁶
seeded-random inputs, values + status + IR steps bit-identical, zero
disagreements** (M3 Max, 72 min live — the one-time blessing price; see the
transcripts entry). The E3 megakernel battery runs the same cells × 16
probes in **one dispatch** with zero disagreements.

**Measured (M3 Max, end-to-end including buffer setup + readback, fuel
metering on).** One-cell × N: **3.7×10⁸ evals/s** at N = 2²⁰ (0.4 ms at 2¹⁶,
54 ms at 2²⁴) — the ≥10⁸ target clears by 3.7× *with* exact metering.
Library × probes: 137–178 ms/launch nearly flat from 8 to 512 probes
(→ 7×10⁵ evals/s at 512) — a fixed per-launch cost dominated by the fused
242-case kernel, not by the evals; fine for batch retrieval, not yet the
"~ms" decode-loop number, and the fix directions are recorded as owed.
Codegen itself is cheap: 2.2 ms to emit 514 KiB of MSL, 2.9 s one-time Metal
compile. Divergence probe (gcd, data-dependent loop counts): uniform-deep
lanes 2.9×10⁸ evals/s vs shuffled-random 3.4×10⁸ — wall time tracks the
**max lane** in a warp (12.3 ms at mean 112/max 240 steps vs 14.4 ms at
uniform 251), confirming the E2 hypothesis: WCET-friendly ≈ SIMT-friendly,
because a WCET bound is exactly a bound on the worst lane.

**Found (and this is why the battery exists).** The Apple Metal compiler
**miscompiles** an integer divide feeding a branch that guards stores through
a `thread`-reference parameter in a non-inlined function — the branch
polarity inverts. Invisible while the inliner swallowed everything; at
242 fused cells the heuristic stopped inlining `mul_sat` and the megakernel
battery caught it (5 disagreeing probes), bisected to a 10-line repro, and
pinned: `always_inline` is ignored at scale, MSL 3.1 doesn't help, the
member-array Ctx doesn't help alone. The dodge is structural and shipped:
div/rem ride opaque `noinline` value-taking helpers (the call boundary blocks
the faulty fusion; signed `MIN/-1` wrap lives inside them), and cell
functions are **pinned noinline** so the shipped configuration is exactly the
battery-validated one rather than whatever the inliner felt like. R1 said
"GPU integer semantics differ across shading languages"; the sharper lesson
is that the *toolchain itself* is part of the threat model, and bit-exact
batteries at library scale are the only reason this was caught.

**Owed.** The library-launch fixed cost (~140 ms): split pipelines /
Metal function tables / per-cell specialization — needed before F1-scale
"~ms" retrieval claims. State cells (455 now) still await typed-state
readback; `Body::Msl` + family-hash attestation (E6) with the host
integration; f32 (E4); CUDA before H3. And the cost-map surfaced a
**worst-case step discipline** gap — seven cells carry ~99.9% of the gate's
oracle bill (one is 1.9M steps/input) — proposed as a per-cell step budget +
`pow_mod` prelude kernel in
[step-budget-amendment.md](step-budget-amendment.md).

### 2026-07-11 — E1 on Metal: `rustmsl` + the library battery (WS-E slice 1)

**Shipped.** `rustmsl`, the MSL sibling of rustz80/rustrv32 over the same
cell80-core seam: IR→MSL codegen for straight-line integer cells (loop-free;
`if` allowed) with the interpreter's semantics reproduced arm by arm, a Metal
batch executor (`GpuBatch`: fast-math off, unified-memory buffers, one thread
per input triple — the layout E3's one-cell×N-inputs megakernel grows from),
and two batteries. The **R1 corner battery** (16 cases, ~70k evaluations) pins
interpreter ≡ GPU on exactly the pre-registered drift corners:
shift-by-≥-width (literal and runtime counts), i16 arithmetic-shift
saturation, signed div/rem `MIN/-1` wrapping (select-guarded in the emitted
MSL — C++ overflows where the IR defines the wrap), byte-width wrap,
short-circuit evaluation hiding a divide, the width bridges, the bit-method
kernels, and both trap statuses. The **E1 library battery** compiles every
straight-line integer value cell in the library — 173 of 657 — and the
pre-registered gate ran clean: **10⁶ seeded-random inputs per cell
(1.73×10⁸ evaluations), the full `[r0, r1, r2, status]` quad bit-exact
against the reference interpreter, zero disagreements** (M3 Max, 266 s
wall, interpreter-side dominated; CI keeps a 512-input version green per
push). Traps are per-thread statuses, never poisoned values: divide-by-zero
and `halt(code)` map to the interpreter's refusals exactly.

**Exceeded.** The E1 plan said "the robo family shapes"; the battery runs the
entire eligible library. Calls survive on the GPU (helper functions through
the flat slot file, the caller/callee aliasing order preserved), and the
window-emulation weakening pre-registered in E1 is concrete and typed: consts
+ slot file are mapped per thread, unmapped reads return the interpreter's
untouched-memory zero, unmapped writes are a counted trap (`STATUS_OOW`),
never a silent drop.

**Owed.** State cells (415 of 657) — typed-state readback on the GPU path
(the `state_addrs` window per thread), with E3's host integration. Loop
cells (69) are E2, next. Cartridge integration — `Body::Msl`, an MSL target
id, family-hash attestation of the GPU body (E6) — rides with E3's per-body
host dispatch. f32 is E4, untouched. No throughput claim yet: E3's megakernel
layouts own the ≥10⁸ evals/s benchmark, and this slice's executor is
correctness-shaped (one pipeline per cell, one buffer round-trip per batch).
