# Pre-registration: C1 — superoptimization over the library, under a real cost model from the start

Status: **pre-registered before any C1 search runs.** Same discipline as
`cell-cost-discovery-preregistration.md` and `cell-fanout-gate-preregistration.md` — the
hypothesis, cost model, scope, and success/kill criteria are fixed here, before the
harness runs, because that is the only way a result gets judged against what was said
would count rather than reshaped around what came out.

This is C1 from `discovery-challenges.md` §2: *"For each library cell, find the cheapest
DAG that is full-domain identical to it... fitness = mean repriced cost, hard constraint
= full-domain equality... Gates: (i) beat the four hand-found pipeline wins... (ii)
per-body divergence."*

## 0. What C1 actually is, precisely, given what C0 already built

C0 (`cell-fanout-gate-preregistration.md`) already built exactly the search this
experiment needs — DAG-with-fan-out grammar, GA+CEGIS over the library's free-fn
vocabulary, GPU-batched full-domain verification (`InterpBatch`, now parity-fixed and
tested), each target being *some library cell's own behaviour*. C0 deliberately used
**IR steps** as its cost model, disclosed as "a cheaper proxy... not repriced Z80 cycles"
— appropriate for a grammar test, wrong for a claim about real algorithms. **C1 is that
same search, same infrastructure, with the cost model fixed to be honest** — nothing
else changes in kind, only the fitness function's units and what gets trusted as a win.

This distinction is not academic. C0's re-run found exactly one construction
(`next_pow2 ← snap_up(mask_xor(x, is_zero(x)), highest_set_bit(x))`) that looked like a
genuine 3.25× win under IR steps, and inverted to 2.57× *worse* once repriced by the
same discipline `cell-cost-discovery` established (`P`, the measured true cost of a
mul/div host trap the runner charges at a flat, unrealistic 4 T-states —
`cell-fanout-gate-findings.md` §5). An IR-step-only fitness function doesn't just risk
occasionally rewarding that move; it **defines** trap-hiding as the optimum for any
candidate with a trapped op available, so **this is the one design mistake C1 exists to
not repeat.**

## 1. The claim being tested

Does population search over typed DAG compositions (fan-out-capable, over the library's
existing verified cells) find, for existing library cells, cheaper full-domain-identical
implementations — under a cost model repriced for the real Z80 substrate from the
start — beating `cell-cost-discovery`'s four pure-pipeline wins (`isqrt`, `bit_length`,
`is_weekend`, `is_odd`)? A hit is a machine-found, machine-proved, **honestly costed**
cheaper algorithm for a function the library already ships. Zero hits (or hits that
don't survive the P=0 sensitivity lane) is a certification that the DAG grammar adds
nothing over pure pipelines at this depth, in this library — a real result either way,
which is what makes this safe to pre-register.

## 2. The cost model — the design this pre-registration exists to fix in advance

**Two stages, not one, forced by an engineering constraint that turns out to also be the
right discipline.** The GPU interpreter (`InterpBatch`) that makes GA-scale search
tractable reports **IR steps**, not real Z80 T-states — real T-states require the CPU
Z80 emulator (`cell80::Runner`), too slow to run per-candidate-per-generation at
population scale (this is exactly why C0's first, broken cut moved verification to the
GPU in the first place). So:

- **Stage 1, search fitness (GPU, IR-step space).** Two corrections made *while writing
  this pre-registration*, before any code, each caught by checking implementability
  rather than assuming it: (1) the first draft described repricing by a *dynamic
  per-input* "mean trapped_ops," mirroring `cell80::Runner`'s ED FE trap counter — but
  `InterpBatch`'s bytecode has no such concept at all. Every `BinOp` (`Add`, `Mul`,
  `Div`, whatever) is charged an identical one-`Step` cost
  (`rustmsl/src/interp/bytecode.rs`'s `Inst::Bin(BinOp, Width)` — one instruction
  variant, one charge, regardless of operator); the underpricing here is broader and
  structural (the IR-step model is blind to operator *and* shift-amount cost, not just
  mul/div — the same blindness that made `highest_set_bit`'s real ~676-T-state shift
  chain look free). (2) The next draft proposed fixing this by statically counting
  `Inst::Bin(Mul|Div, _)` occurrences in each vocabulary cell's own linearized
  bytecode — but `Inst` and `CellProgram::code` are both `pub(crate)` inside `rustmsl`
  (confirmed directly: `rustmsl/src/interp/bytecode.rs:15,83`), invisible to an external
  example crate; that approach cannot be built without changing `rustmsl` itself,
  which this pre-registration does not propose to do.

  **What's actually built:** a **static, per-vocabulary-cell signal derived from the
  real Z80 substrate**, not from IR bytecode at all. Each vocabulary cell (both the
  unary total-cell targets and every arity-≤2 pool cell used as a `Call` node) is
  compiled via `cell80::Cartridge::compile` (the real `rustz80`/Z80 path — the same
  compiler stage 2's confirmation already needs) and run once, up front, over a
  representative sample (the full 65,536-input domain for unary cells; the same
  u8×u8 = 65,536-pair grid `cost-discovery`'s own `P`-measurement uses, for binary
  cells — arity-2 full-domain-on-CPU is exactly what that programme itself deferred as
  intractable) via `cell80::Runner`, recording each cell's own **mean real
  `trapped_ops`** over that sample — the authoritative signal (dynamically observed
  from the actual compiled Z80 body), not a guess from source syntax or an unavailable
  bytecode. A composed candidate's stage-1 fitness is then `mean IR steps (over probes)
  + P_IR × (sum of each `Call` node's cell's precomputed mean trapped_ops, over the
  `Expr` tree, recursively)` — repriced *from generation zero*, using a real,
  dynamically-measured per-cell signal as a static per-candidate proxy (computed once
  per cell, reused for every generation, not re-observed per composed candidate).
  `P_IR` itself is measured fresh, in **IR-step units** (not cost-discovery's
  T-state-space `P = 5,412` — a different unit space entirely, since it multiplies a
  count against an IR-step total): the same differential method — a trap-free
  shift-and-add `mul16` vs. a plain `a*b` cell, over the full u8×u8 grid — tabulated
  via IR steps (`InterpBatch`'s own `steps` output) rather than `Runner`'s cycles, so
  the reprice constant and the fitness units stay self-consistent even though the
  *signal it's multiplying* (mean trapped_ops per cell) comes from the other engine
  entirely. This mixing is deliberate and sound: `mean trapped_ops` is a dimensionless
  count, observable by whichever engine actually models traps (only `Runner` does);
  `P_IR` is a separate, IR-step-denominated price for that count.
- **Stage 2, confirmation (CPU, T-state space, cost-discovery's `confirm.rs` pattern).**
  Every full-domain-verified, IR-repriced-cheaper candidate is hand-composed into one
  source (inlining every stage, matching `experiments/cell-cost-discovery/src/bin/
  confirm.rs` and this session's own `spotcheck_next_pow2_z80.rs`), recompiled, and
  re-costed under the **real** Z80 cost model: `Runner`'s cycles + `P_T × trapped_ops`,
  with both the candidate's `trapped_ops` (`cell80::Runner`'s real, dynamic, per-input
  ED FE trap counter — the actual mechanism, not a proxy) and `P_T` itself measured
  fresh in this same harness by the identical T-state-space method `cost-discovery`
  used (expected, not assumed, to reproduce ≈5,412 — a fresh measurement, checked, not
  hardcoded). **Only a candidate that is cheaper under stage 2 counts as a win.** A
  candidate that wins stage 1 but loses stage 2 is reported, named, and excluded —
  exactly `next_pow2`'s fate this session, now a designed-for outcome instead of a
  discovered one.

**Sensitivity, pre-committed (per `discovery-challenges.md`'s own C1 gate and
`cost-discovery`'s established practice):** every stage-2-confirmed win is additionally
reported at `P_T = 0` (raw T-states only). A win that only survives at `P_T = 0` (a
`mul`/`div`-trap-hiding artifact by the mirror argument) is labelled
`repricing-dependent` and does not count toward the success gate — the same rule that
would have caught `next_pow2` immediately had it been run through stage 2 before being
reported.

**What this does not fix:** stage 1's IR-step-space repricing is a *static structural*
proxy for real Z80 cost, shaping search direction only — it is not required to be a
perfect predictor of stage 2's verdict (stage 2 is the actual gate), and it is known in
advance not to capture per-input branch-dependent trap behavior (a cell that only
divides on some inputs gets the same static bump on every input) or any cost source
outside mul/div — the shift-amount blindness that inflated `highest_set_bit`'s apparent
cheapness (~676 real T-states with zero traps) is **not** addressed by this reprice at
all, a real disclosed gap, not an oversight. A candidate stage 1 mis-ranks for either
reason can still be searched, found, and correctly rejected at stage 2 — the design
only needs stage 1 to be *directionally* honest about mul/div-heavy candidates, not
perfectly predictive of every real cost source.

## 3. Scope, fixed now

- **Targets.** Every total (`Halt::Returned` on all 65,536 inputs) unary
  `u16 → u16` free-fn cell in the vocabulary pool, built the same way C0's re-run built
  its 69 targets (`gpu_fanout_gate.rs`'s existing loader: `discover_cell_files` →
  `rustmsl` lowering → `linearize` → `InterpBatch` tabulation). **Not** re-derived via
  `cell-cost-discovery`'s own separate compile path — the two harnesses' target sets
  already differ by construction (69 vs. 62, disclosed in C0's findings §0) and forcing
  them identical would cost more engineering than the comparison is worth; "beat 4" is
  a bar on the *count* of genuine wins across a similar-but-not-token-identical target
  universe, matching how C0 already handled this.
- **Vocabulary.** C0's existing pool: every unary/binary free-fn cell (arity ≤ 2,
  scalar-typed, single-function after inlining) that lowers and linearizes under 64
  locals — **now including `raw-arith`'s `add`/`sub`** (both lower cleanly; confirmed in
  C0's re-run). **`shl`/`shr` are excluded from the search vocabulary**, disclosed, not
  silently worked around: `rustmsl`'s linearizer bails on `Expr::ShiftVar` (a runtime,
  non-literal shift amount), a pre-existing limitation of the GPU-interpreter path only
  (the real Z80 compiler, `rustz80`, already supports them — `bit_is_set`/`set_bit`/
  `clear_bit`/`toggle_bit` ship using exactly this). Fixing `ShiftVar` support in
  `rustmsl`'s linearizer would be a genuine, separate piece of engineering (a new opcode
  in the bytecode + MSL kernel) — out of scope for this pre-registration; noted as a
  concrete, scoped follow-up if C1's results motivate it.
- **Grammar.** Unchanged from C0: `Expr` trees (`Var(0)`, `Lit(k)` from the same
  14-constant set, `Call(name, args)` over the vocabulary), max depth 4, fan-out via
  free reuse of `Var(0)`. A target's own cell is excluded from its own candidate trees
  (the `is_carmichael`-class self-composition Goodhart guard C0 already implemented).
- **Depth / budget.** Matching C0's proven constants: population 4,096, up to 400
  generations per target with the same tightened plateau early-stops (20 generations
  no-improvement pre-hit, 40 post-hit), the same capped per-generation full-domain
  verification (4 smallest-tree candidates/generation). Not widened for this first C1
  pass — comparability with C0's already-validated budget matters more here than
  squeezing out a marginal win-rate improvement; a budget study is explicitly deferred
  (`cell-fanout-gate-findings.md`'s own closing recommendation).
- **Equivalence.** Tier A only: exact table equality on all 65,536 inputs, verified via
  `InterpBatch` (stage 1, the search's own CEGIS growth) **and independently
  re-verified via `cpu_run`/`Runner` at stage 2** (the hand-composed single source) —
  belt-and-suspenders after this session's `InterpBatch` parity bug, even though that
  bug is now fixed and regression-tested.

## 4. Search algorithm

Reuse `gpu_fanout_gate.rs`'s GA+CEGIS shape unchanged in structure: population of
`Expr` trees, elitism + mutation, CEGIS probe growth (a probe-perfect candidate is
verified full-domain via `InterpBatch` before being trusted; a counterexample grows the
probe set). The only change is the fitness computation itself (§2's `P_IR`-repriced IR
steps, not raw IR steps) and that the search **does not stop at the first
full-domain-correct find** — exactly as C0 already does, it keeps searching for a
cheaper (now: cheaper *and honestly costed*) verified candidate until the budget or
plateau ends the run.

## 5. Success / kill, pre-registered

- **Primary gate (i):** ≥5 stage-2-confirmed wins (full-domain identical, strictly
  cheaper under `Runner` cycles + `P_T`), each robust under the `P_T = 0` sensitivity
  lane. This "materially exceeds cost-discovery's 4" in the same sense
  `cell-fanout-gate-preregistration.md` used for its own bar.
- **Kill:** fewer than 5 stage-2-confirmed wins. Per `discovery-challenges.md`'s own
  kill language for C1: *"the authored library is near-optimal under pipeline
  composition at the searched depth"* — except now the claim is stronger, since the
  grammar tested is DAG-with-fan-out, a strict superset of pipelines, under the real
  cost model. A kill here is evidence the DAG grammar's added expressiveness doesn't pay
  at this depth/budget on this body — not evidence against the search or the
  infrastructure, both already validated by C0.
- **Depth-1 hits are gate-escape audits, not discoveries** — the same rule
  `cell-cost-discovery-findings.md` established and C0 already applied to
  `wilson_theorem_check ← is_prime(x)`. Reported separately, never counted toward (i).
- **Gate (ii), per-body divergence (RV32) — explicitly deferred, not v1.** Doc 18's own
  prediction: *"run the same search against the RV32 cost model, where hardware
  multiply inverts the economics... it rejects the mul-avoiding rewrites `isqrt ←
  geomean2` depends on."* This needs an RV32 cost model + repricing pass this
  pre-registration does not build. If (i) passes, (ii) is the natural next
  pre-registered phase — upgrading the claim from "identical behaviour on four bodies"
  to "identical behaviour, per-body optimal implementations." Named here so it isn't
  silently dropped, not attempted here.

## 6. What this would NOT show

Nothing about: arity-2 targets (out of scope, same reason cost-discovery deferred them
— 2³² domain wants a different GPU dispatch shape), `shl`/`shr`-using candidates in the
search (excluded by the disclosed `ShiftVar` gap — a real Z80 confirm-stage hand
composition *could* in principle use them, but the search itself cannot find such a
candidate to confirm), recursion or divide-and-conquer (the grammar has none), RV32 or
any other body's economics (gate ii, deferred), or a search-power/win-rate claim beyond
this specific budget (a dedicated budget study is `cell-fanout-gate-findings.md`'s
separately-recommended, not-yet-run follow-up).

## 7. Method reuse

`cell80::{discover_cell_files, Cartridge::compile}` → `rustmsl` lowering + `linearize`
→ `InterpBatch` (stage 1, search) → hand-composed single source + `cell80::Runner`
(stage 2, confirm) — the same pipeline `gpu_fanout_gate.rs`, `cell-cost-discovery`'s
`confirm.rs`, and this session's `spotcheck_next_pow2_{z80,breakdown}.rs` already
established and proved correct. Harness: a new `cell80/examples/gpu_superopt.rs`
(macOS/Metal only, matching its C0 predecessor).
