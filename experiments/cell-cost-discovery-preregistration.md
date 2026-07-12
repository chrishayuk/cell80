# Pre-registration: does cost pressure find cheaper, proved-identical implementations inside the existing library?

Status: **pre-registered before any search was run.** This document fixes the hypothesis, the
cost model, the scope, and the success/kill criteria first, in the same discipline as
`evolved-cells-preregistration.md` — so that whatever comes out is judged against what we said
would count, not against a write-up reshaped around the result.

## The claim being tested

A novel algorithm is not usually a new function — it is a **cheaper implementation of a
function you already have** (Strassen computes matrix multiply; FFT computes the DFT; the
primitives were ancient in both cases, the wiring was the discovery). cell80 is unusually
well-placed to test a narrow version of this claim mechanically, because it has the three
ingredients at once:

1. **Exact behavioural equality is checkable, not sampled**: a unary `u16 → u16` cell has
   65,536 inputs; comparing two implementations over the full domain is a proof, not a test.
2. **Cost is exact and deterministic**: `Report.cycles` is T-state-exact by construction, the
   same number on every run.
3. **A library of 790 verified primitives** exists to compose from.

The narrowest rung of the claim, tested here: **for existing unary library cells, does a
pipeline composition of other existing library cells reproduce the cell's behaviour exactly
(full u16 domain) at strictly lower cost?** A hit is a machine-found, machine-proved cheaper
algorithm for a function a human already authored. Zero hits is a certification that the
authored library is near-optimal under pipeline composition at the searched depth — a real
result either way, which is what makes this safe to pre-register.

Precedent that the loop pays when a human runs it by hand: the GPU-cells profiling session
found a handful of cells eating ~99.9% of an oracle budget and cut the bill ~7× with
value-identical rewrites. This experiment automates exactly that move and points it at the
whole library.

## Relation to prior experiments (what is genuinely new here)

`cell80::synth`, `cell-synth-evolve`, and `evolved-cells` all search compositions that
**reproduce examples** — the objective is coverage of a behaviour that doesn't exist in the
library yet. Here the target behaviour **already exists**; the objective is **strictly lower
cost at proved-identical behaviour**. Nobody asked for a new function. That inversion —
equivalence as the constraint, cost as the fitness — is the thing being tested, and it is the
signal that demand-driven and coverage-driven synthesis cannot provide (nothing "asks" for
FFT; it computes the same function as the naive DFT, just cheaper).

## Step 0, fixed before running: the cost model

**Primary metric**: mean over the full u16 domain of per-run cost on the Z80 body, where

```
cost(v) = cycles(v) + P × trapped_ops(v)
```

**Why repricing is mandatory, not optional**: the runner charges every mul/div host trap a
flat **4 T-states** ("a fast hardware op", `cell80/src/runner.rs:127`, with `trapped_ops`
reported alongside precisely because of this caveat). A cost-pressure search is an adversary
against its cost model: under near-free multiplication, the cheapest implementation of
anything multiplicative is "use the trap", and any rewrite that trades multiplies for
adds/shifts — the entire Karatsuba/Strassen family of moves — is priced out of existence
before the search starts. The trap must be priced at what the substrate would pay without it.

**P is measured, not invented**: two experiment-local cells are compiled and run on the Z80
body over the same fixed grid — a trap-free shift-and-add `mul16` (pure
`add`/`shl`/`and`/`while`, no `*`, fixed 16 iterations) and a plain `a * b` trap cell.
`P = mean_cycles(soft) − mean_cycles(trap)`: the differential isolates the trap's true
replacement cost, with shared call overhead cancelling exactly (and the trap's own 4 charged
cycles subtracted implicitly). Grid: the full u8×u8 cross (a, b ∈ 0..=255 — 65,536 pairs),
chosen so every product fits u16 and the trap cell cannot overflow-stop. The substrate prices
its own trap. P is reported in the findings. Div and fill traps carry the same P — a
single-price simplification (div's true software cost is higher), noted rather than modelled.

**Sensitivity, pre-committed**: every hit is also reported at **P = 0** (status-quo pricing).
A hit that survives P = 0 is unconditional; one that appears only under repricing is a
repricing-dependent discovery and is labelled as such. No other values of P will be tried
(no tuning P until something wins).

**Aggregation, pre-committed**: **mean** over the full domain, not worst-case. Cycles are
input-dependent (branches, data-dependent loops), so a candidate can win one aggregate and
lose the other; the findings may *report* worst-case numbers for hits, but the win condition
is the mean, decided now.

**Overhead bias, acknowledged**: a candidate chain of depth d is costed as the sum of d
separate `run_fast` executions — d call overheads against the target's one. The real composed
cell (one inlined body, the `compose.rs` link path) would be cheaper than the chain-sum. The
bias is therefore **against** discovery: a chain-sum win understates the true win. Hits are
additionally hand-composed into a single source and recompiled to report the inlined cost.

## Scope, fixed now

- **Targets**: free-fn (non-state) cells with signature `u16 → u16` that are **total** —
  `Halt::Returned` on all 65,536 inputs. Cells that halt/escalate/div-zero anywhere in the
  domain are excluded and counted in the findings.
- **Vocabulary (pipeline stages)**: (a) every total unary target cell itself; (b) every total
  free-fn `(u16, u16) → u16` cell with its **second** argument fixed at a constant from
  `C = {0, 1, 2, 3, 4, 5, 8, 10, 16, 255, 256, 0x00FF, 0xFF00, 0xFFFF}` (14 constants, fixed
  now; an op is admitted only if total over all v at that constant). Second-slot-only matches
  the existing `synth::Op` convention.
- **Operator**: pure pipeline — function composition only. **No fan-out**: `x*3 = (x<<1)+x`
  is *inexpressible* here because the running value cannot be used twice. No recursion, no
  iteration combinator. This is the thinnest slice of the composition-grammar programme, run
  first because it is buildable today; DAG wiring and the divide-and-conquer combinator are
  the next rungs and are explicitly not claimed by this experiment.
- **Depth**: ≤ 2 with the full vocabulary; depth 3 by extending the (deduped, cost-pruned)
  depth-2 frontier with **unary ops only** — full-vocabulary depth 3 is priced out on CPU
  (a GPU sweep is future work, noted not run).
- **Search**: breadth-first enumeration with exact dedup on the composed table, keeping the
  min-mean-cost representative per table — lossless for the mean objective, because the
  downstream cost of any extension depends only on the table, not on which chain produced it.
  Chains whose running mean cost already meets or exceeds the most expensive target are
  pruned (costs are additive and positive). Any frontier truncation beyond these two
  principled prunes is a scope cut and is logged with counts (no silent caps).
- **Equivalence**: exact table equality on all 65,536 inputs — Tier A (proved), the only tier
  in this experiment. No fingerprint-sampled equivalence is accepted for a hit.

## Success / kill, pre-registered

- **Success**: ≥ 1 target with a full-domain-identical chain at strictly lower mean cost
  (repriced). For each hit: the chain, both costs, the P = 0 status, and the recompiled
  single-source cost are reported.
- **Depth-1 hits are not discoveries**: a single existing op exactly matching another cell's
  table means the admission gate's *sampled* fingerprint admitted a behavioural duplicate.
  Those are reported separately as gate-escape audits.
- **Zero hits**: reported as the certification result — the authored library is near-optimal
  under depth-≤3 pipeline composition at this cost model. Not a failure; the claim's kill
  condition working as designed.
- **What would falsify the broader programme step**: zero hits *and* the hand-precedent
  (7×-style rewrites) being reproducible by a human inside this same operator class — that
  would mean the search, not the space, is the problem. (If the human rewrites all needed
  fan-out or wider arities, the operator class is the limit instead, which feeds the
  grammar-building rung.)

## What this would NOT show

Nothing about: arity-2 targets (2³² domain; the GPU full-domain path exists but is not wired
here), fan-out/DAG wirings, recursion or divide-and-conquer (the Karatsuba calibration rung is
**deferred** until the combinator grammar exists *and* a hand-computed oracle confirms
Karatsuba is strictly cheaper than schoolbook at the chosen width under this exact cost model
— pre-registering that target without the oracle would make a search failure uninterpretable),
other bodies' cost regimes (RV32 hardware mul inverts the trap economics), state cells, or
anything the 14-constant set excludes.

## Method reuse

`discover_cell_files` → `Cartridge::compile` (sandboxed) → signature filter →
`Runner::run_fast` to build per-op tables `(result, cycles, trapped_ops)` over the domain —
the verifier is the engine, as in `synth::Op::from_cell`, extended to carry per-input cost.
Harness: `experiments/cell-cost-discovery/`.
