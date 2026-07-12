# 17 — Deterministic Ecology on State Cells: the experiment programme (draft v0.1)

**Status:** EX-0 through EX-5, the full programme, built and landed — receipts in
`deterministic-ecology-findings.md`. EX-3's coupled-arms-race claim did not hold up under a
permutation-null test (an honest kill condition, not smoothed over); every other gate
passed. Still speculative, off-roadmap (same footing as `cell80-life.md`).
**Depends on:** cell80-life (composition → selection, established), evolved-cells /
cell-synth-evolve (GA/MCTS/A* search reusable API), the WS-E GPU interpreter
(state cells, IR-step parity, one-cell×N at 3.7×10⁸ evals/s up to N=2²⁰, library×probe
megakernel flat ~140–180ms across probe counts), admission gate + behavioural
fingerprints, cell80-core reference interpreter (the oracle).
**Hardware:** M3 (Metal interpreter). CUDA only if a run needs >~500k organisms.

## 0. Thesis and what's already settled

`cell80-life` proved three things at CPU scale that this programme scales and stresses:
curated cells compose into sustained population dynamics; the genome file is causally
load-bearing, not cosmetic; mutation produces *selection*, not drift (the planted
`argmin3` purge, 8 seeds). What it could not do: run enough organisms to see rare
variants, pit two strategies against each other at scale with mutation, or make the
world's step the GPU's dispatch.

**The claim under test:** an ecology where every organism's genome is content-addressed
auditable bytecode, every organism is a state cell stepped by the reference-parity GPU
interpreter, and every run **replays bit-for-bit from (seed, genome-set)**, exhibits
open-ended selective dynamics at a scale where the *mechanism* of each evolutionary event
is inspectable — not inferred from population statistics but read off the genome that won.

**Non-goals.** No fitness function hand-designed toward a target behaviour (fitness is
survival/reproduction in the world, never a distance-to-goal). No nondeterminism anywhere
— RNG is counter-based (seed as input), so "same seed, same genomes → same history" is a
hard contract, not a hope. No organism escapes the sandbox: an organism *is* a cell, so
the trust surface is unchanged from every other cell.

## 1. The determinism contract (the spine — verified before any biology)

Every experiment inherits this and none proceeds if it fails.

- **World state = f(seed, genome-set, tick)**, exactly. Two runs, same inputs, byte-identical
  history — organism positions, energy, births, deaths, the lot.
- **Counter-based RNG** (Philox/threefry shape): randomness is a pure function of
  (seed, tick, organism-id, stream). No global RNG state, so parallel dispatch order
  cannot perturb outcomes.
- **GPU ≡ interpreter at every tick.** The world-step is state cells under the WS-E
  interpreter; a CPU reference run of the same ticks agrees byte-for-byte on all organism
  state. Divergence is a filed defect, never "GPU variance."
- **Genome = content-addressed bytecode.** Every organism's genome hashes; a lineage is a
  chain of hashes; "what won" is answerable by reading a cell, not a statistic.

**EX-0 — the replay gate.** Run any world twice on (seed, genomes); assert byte-identical
history. Run it on GPU and CPU-reference; assert byte-identical. **Kill:** if replay isn't
bit-exact, the whole premise ("auditable, reproducible evolution") is void — fix before biology.
Cost: days. This is the first thing built.

**Status: DONE.** Both gates passed cleanly on the first working implementation. Full
account in `deterministic-ecology-findings.md`'s `## EX-0` section.

## 2. The experiments

Ordered so each unlocks the next; every one has a kill criterion and a pre-registered
"what would count as real vs. artifact."

---

### EX-1 — Scale the known ecology (does the CPU result survive 10⁴–10⁵ organisms?)

**Question.** cell80-life's dynamics were seen at n≈10. Do the same genomes, stepped on
the GPU interpreter, sustain populations at 10⁴–10⁵ in a larger world — or does scale
reveal that the small-n stability was a boundary artifact?

**Method.** Port the grazer/rapid_reproducer genomes unchanged. 2-D toroidal world with
food fields. Sweep world size and food density; measure population trajectory, births,
deaths, extinction rate, vs. the CPU baseline's qualitative regimes (steady vs boom-bust).

**Gate (real, not artifact).** The two genomes reproduce their *qualitative* CPU regimes
at scale (grazer steady, rapid_reproducer oscillatory), and the flat-in-library-size
interpreter result holds: per-tick cost scales with organisms, not with genome diversity.
**Kill.** Populations only survive in a narrow hand-tuned parameter slot → the ecology is
fragile, not a substrate; report the slot and stop.
**Cost.** Low. Mostly harnessing existing genomes to the GPU stepper.

**Status: DONE.** Population survival/steadiness scales robustly to 10⁵ organisms, but the
qualitative regime *distinction* between the two genomes (steady vs. boom-bust) does not
survive scaling — traced, not just observed, to a small-population finite-size effect via a
dimensionality-isolation experiment (a height=1 ring reproduces the split at n≈8–20, but it
collapses again by n≈1000). Grazer's half of the gate passes at every scale/topology
tested; rapid_reproducer's oscillatory half does not, at any scale ≥ ~100 — the literal
answer to this experiment's own question. Full account in
`deterministic-ecology-findings.md`'s `## EX-1` section.

---

### EX-2 — Open-ended genome mutation (beyond tunable numbers to *bytecode*)

**Question.** cell80-life mutated genome *numbers* and swapped among same-signature
sibling cells. Can mutation act on the **bytecode itself** — the interpreter's genome is
a buffer — producing organisms whose *behaviour*, not just parameters, is novel and still
sandbox-safe?

**Method.** Two mutation operators, pre-registered separately: (a) parametric (the known
one, as control); (b) **bytecode-level** — point/insert/delete on the genome's op stream,
every mutant passing the same structural validity + admission-safety check a synthesized
cell would (bounded, typed, no out-of-window write — an invalid mutant is a counted
stillbirth, never a crash). Measure: fraction of mutations viable, behavioural novelty of
survivors (fingerprint distance from parent), whether bytecode mutation reaches strategies
parametric mutation provably cannot.

**Gate.** Bytecode mutants that are (i) sandbox-safe by construction, (ii) sometimes
viable, (iii) occasionally fitter than parent — and the fingerprint shows they're doing
something the parameter-space couldn't express. **Kill / re-scope.** If viable bytecode
mutations are vanishingly rare (the usual GP brittleness), fall back to the
richer-but-bounded move: mutation over a *typed cell-assembly grammar* (swap/insert whole
cells from the library into genome roles), which evolved-cells already showed is
searchable. Either outcome is a result about the mutation substrate.
**Depends on.** EX-1. **Cost.** Medium — the safety check reuses admission machinery.

**Status: DONE — both operators.** Operator (a) (parametric + cell-swap): ported into the
GPU-batchable engine; diversity emerges and grows exactly as expected (95.7% of births
carry a mutated role over 2,000 ticks). Operator (b) (cell-assembly composition, arity-
preserving 2-cell wiring): the kill/re-scope condition did **not** fire — viable
compositions are far from vanishingly rare (57.7%/83.3% viable across the two role pools),
and the ecology substantially exploits them once available (29.6% of births in an
extended-pool run carry a composed gene). "Occasionally fitter than parent" was not shown
at the population-aggregate level (composed-gene carriers averaged fewer direct children
than disk-gene carriers in this run) — reported as-is, not smoothed over; an
individual-candidate fitness breakdown remains open. Full account in
`deterministic-ecology-findings.md`'s `## EX-2` section.

---

### EX-3 — Predator/prey: the second organism (co-evolution, the tournament dispatch)

**Question.** cell80-life selected *within* one strategy (purge the bad mutant). Does a
world with two interacting roles produce *co-evolution* — an arms race read off the
genome lineages, not inferred from oscillation?

**Prior art.** cell80-life already has a working predator/prey mechanic at n≈2–8
(`genomes/predator.json`, species-tagged `prey_at`, the sensing bug fixed): predation is
correct and species-honest, but every run so far boom-busts to extinction — the predator
exhausts the grazer population within a few hundred ticks, then starves once its food
source is gone — with the standing hypothesis (not yet tested) that a stable equilibrium
needs more grazer carrying capacity, slower predator reproduction, or a satiation
mechanic, not a sensing fix. EX-3 is not "invent predator/prey" — it's "scale the existing
mechanic to where mutation has room to run, test the standing carrying-capacity/satiation
hypothesis directly, and check whether an arms race emerges instead of the same collapse."

**Method.** Two organism classes sharing the world: prey (forage food) and predators
(gain energy by catching prey), ported from cell80-life's existing roles rather than
designed fresh. Interaction resolved by **tournament dispatch** — the GPU-scale version of
pairwise contests, batched. Both genomes mutable (EX-2 operators). Measure lineage traits
over time: prey evasion, predator pursuit; look for the signature of an arms race (coupled
trait escalation) vs. one class simply winning, and separately check whether the
world-size/satiation levers that cell80-life could not test at n≈2–8 are enough on their
own to produce a stable equilibrium with mutation off — the control that isolates "more
room" from "evolution" as the explanation for any stability seen.

**Gate (the anti-artifact bar).** A *coupled* dynamic — predator improvement followed by
prey counter-adaptation traceable in the genome hashes — not merely Lotka-Volterra
population cycles (which EX-1's dynamics could produce with no evolution at all). The
distinction is the whole point: population oscillation is ecology; **traceable coupled
trait change is co-evolution.** Pre-register both and require the latter.
**Kill.** Only cycles, no traceable coupled adaptation, across seeds → the world lacks the
degrees of freedom for an arms race; report and revisit interaction rules.
**Depends on.** EX-2. **Cost.** Medium-high — the flagship run.

**Status: DONE — the kill condition fired, honestly.** The two-species engine itself works
cleanly (bit-exact replay, GPU ≡ CPU-reference, including predation-kill tournament
dispatch), and the pre-registered mutation-off control landed a strong, well-powered result:
mutation is causally necessary for predator/prey coexistence at this scale (10/10 seeds
collapse to predator extinction without it, across two independently-robust world configs;
a satiation mechanic built specifically to rule out an overhunting confound doesn't rescue
it either, also 10/10). But the flagship claim — a traceable, coupled arms race — was not
found: across 6 long (10,000-tick) seeds, a rigorous permutation-null test showed the
observed cross-species event alternation is statistically indistinguishable from chance
(p = 0.13–0.99). Per the pre-registered gate, this is real ecology (population dynamics,
mutation-dependence) without demonstrated co-evolution — the kill condition this doc named
up front, not a negative result being smoothed over. A genuine structural limitation likely
contributes: grazers have no predator-sensing channel at all in this model, so any coupling
could only ever act through the far fainter differential-mortality route. Full account,
receipts, and what would raise confidence further in `deterministic-ecology-findings.md`'s
`## EX-3` section.

---

### EX-4 — The lineage record (what only this substrate can deliver)

**Question.** Given determinism + content-addressed genomes, can every evolutionary event
in a run be *explained* — the winning genome at each turning point read, diffed against
its parent, and the causal mutation named?

**Method.** No new world — instrumentation over EX-1/2/3 runs. Full lineage tree
(hash → parent hash), diffs at each fixation/purge event, the fingerprint delta that
mutation produced. Because the run replays bit-for-bit, any event can be re-entered and
inspected at full resolution *after* the fact — the counterfactual-sweep idea, applied to
evolution: fork at a turning point, vary the one mutation, replay, see if the outcome flips.

**Gate.** For a sample of fixation events, the responsible genome change is identified and
its behavioural effect confirmed by replay-with-that-change-reverted. **This is the
research artifact:** "evolution you can single-step and diff," which no stochastic ALife
system can offer because none of them replay exactly.
**Depends on.** EX-1 (works even without EX-2/3). **Cost.** Low-medium; pure payoff on the
determinism spine.

**Status: DONE.** Ran the full pipeline for real: a genuine sustained plurality-change
event, traced to one origin mutation (a full 6-field diff surfaced a second, co-occurring
numeric drift in the same birth), reverted, replayed — the event no longer occurred after
the revert, with every tick before the fork byte-identical to baseline. "Evolution you can
single-step and diff," demonstrated on real data, not just built and unit-tested. Full
account in `deterministic-ecology-findings.md`'s `## EX-4` section.

---

### EX-5 — SOMA hand-off (does the population substrate serve the creature-raiser?)

**Question.** Is a bit-reproducible, auditable-genome population the training substrate
SOMA's creature-raiser wants — organisms as candidate policies, the world as a
deterministic curriculum?

**Method.** Scoping/interface only, not a biology claim. Define the seam: an organism's
genome *is* a policy cell; a surviving lineage is a selected policy; the multi-target
contract means a winning genome deploys to the RP2350 with proved behavioural identity.
Prototype the export: winning genome → `.cell` → RV32 artifact, hash-attested end to end.
**Gate.** One organism evolved in EX-3 exported and shown behaviourally identical on the
robot's target ISA. **Depends on.** EX-3 + docs/13 RV32 path. **Cost.** Integration.

**Status: DONE — passed on the first real run, zero new `cell80`/`rustrv32` code needed.**
One real surviving predator from an EX-3 flagship run had its full resolved genome (6
gene-cell choices, including a `repro_promoter` that had genuinely evolved away from its
species' starting cell) hash-attested and proven behaviorally identical across the Z80
body, the RV32 body (the robot's target ISA), and the CPU-reference interpreter — extending
EX-0–EX-3's "GPU ≡ interpreter" discipline to "GPU ≡ interpreter ≡ RV32." Scoped
deliberately to per-cell attestation, not a single composed whole-organism RV32 artifact —
the tick engine's host-orchestrated control flow isn't folded into one cell (that's a
redesign, not a prototype); cycle counts aren't reported (the RV32 cycle table stays
provisional until B4). Full account in `deterministic-ecology-findings.md`'s `## EX-5`
section — this closes out the doc 17 programme.

---

## 3. Sequencing

```
EX-0 (replay gate, days) ── spine, blocks everything
   └─ EX-1 (scale known ecology) ─┬─ EX-4 (lineage record) ── low-cost payoff
                                   └─ EX-2 (bytecode mutation) ─ EX-3 (predator/prey)
                                                                     └─ EX-5 (SOMA seam)
```

Wave 1: **EX-0 then EX-1** — prove replay, prove the known ecology scales. If EX-1's gate
fails, the substrate claim is wrong and the cheap way to learn it is here. **Done.**
Wave 2: **EX-4** (nearly free, and it's the differentiating artifact) alongside **EX-2**.
**Done.**
Wave 3: **EX-3** (the flagship, the watchable one), then **EX-5** if the robot lane wants
it. **Both done** — EX-3's kill condition fired honestly (see its `Status` note above);
EX-5 passed on the first real run (see its `Status` note above). The doc 17 programme
(EX-0–EX-5) is complete.

## 4. The anti-artifact discipline (cross-cutting)

The failure mode of ALife demos is calling ordinary dynamics "evolution." Every gate above
pre-registers the mundane explanation and requires beating it:

- Population *oscillation* is not selection (EX-1 vs EX-3's coupled-trait bar).
- A genome file *correlating* with dynamics is not causation — swap-with-zero-code-change
  is the cell80-life control, kept (EX-1).
- A fitter survivor is not open-endedness — fingerprint-novelty vs. parent, and
  reachability beyond the parameter space, is the bar (EX-2).
- "It looks alive" is not a result — the lineage diff that *names the mutation* is (EX-4).

Determinism is what makes every one of these checkable rather than rhetorical: the claim
"this mutation caused this fixation" is tested by reverting exactly that mutation and
replaying, which only works because the run is bit-exact.

## 5. The channel note (why this doubles as content)

EX-3 rendered — tens of thousands of organisms stepping in lockstep, an arms race
unfolding — is the most watchable thing the substrate can produce, and unlike every other
ALife video, this one can pause on a single organism and show the audited bytecode of what
just out-competed its parent. "Deterministic evolution with inspectable genomes" is the
one-line hook; EX-4's single-steppable lineage is the payoff shot. It rides on the
research, costs nothing extra to capture, and needs video-1's cell vocabulary as its only
prerequisite.
