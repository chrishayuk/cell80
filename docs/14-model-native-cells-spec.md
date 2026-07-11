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
