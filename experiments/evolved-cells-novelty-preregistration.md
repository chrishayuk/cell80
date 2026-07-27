# Pre-registration: does undirected novelty search discover cells an agent would actually want?

Status: **registered, nothing run.** This is EV-2 — the second pre-registered run of the
`evolved-cells` line, following `evolved-cells-preregistration.md` / `evolved-cells-findings.md`
(EV-1, 6/6 reachable targets, closed 2026-07-05). Same discipline as EV-1 and the doc-17
ecology programme: hypothesis, method, controls, and a numeric bar committed *before* anything
runs, so a good-looking result can be checked against what we said would count, and a bad one
can't quietly become "promising early signal."

## The claim being tested

EV-1 answered *"can search find a correct chain for a target we specify?"* — yes, 6/6, including
one target (`mystery_bits_2`) A\* structurally cannot reach while GA and MCTS both can. Every
target was written by hand first. That makes EV-1 a search benchmark, not a library-growth
instrument: it can only ever find behaviours someone already thought to name.

EV-2 removes the target. **The claim: undirected search over compositions of existing cells,
selected for behavioural novelty against the whole library under a cost constraint, produces
cells a retrieving agent would want — at a rate distinguishable from random chains.**

If that holds even weakly, there's a third authorship route with a demand-free discovery step:
the library can grow into behaviour-space nobody mapped. If it fails, the finding is equally
useful and considerably more likely (see the predictions): **behaviour-space novelty does not
track usefulness, and machine library growth needs a demand signal, not a novelty signal.**

### Why this isn't C1 again

`cell-superoptimization-findings.md` (C1, 2026-07-14) is a FAIL that could be mistaken for a
verdict on this: 1 stage-2-confirmed win against a bar of ≥5, zero genuine fan-out wins, kill
language reading *"the authored library is near-optimal under composition at the searched depth."*

That result is about search as an **optimizer** — finding cheaper implementations of behaviours
that already exist. EV-2 is search as a **discoverer** — finding behaviours that don't. C1's
conclusion is not evidence against EV-2 and must not be cited as such; if EV-2 fails it will
fail for its own reason (novelty ≠ demand), which is a different mechanism entirely. Recording
this now because in three weeks "we already showed search doesn't beat the authored library"
is exactly the sentence that would retire this thread for the wrong reason.

## What changes when the target goes away

Removing the oracle removes the correctness bar with it. A chain of existing cells is correct
by construction — it computes whatever it computes; there is nothing to be wrong about. Three
consequences, all of which shift work onto the acceptance side:

1. **Correctness is replaced by non-triviality** (G1 below). Constants, identity, and
   extensional duplicates of existing cells are the failure modes, not wrong answers.
2. **Novelty becomes the fitness, and novelty is trivially gameable.** Five random bit ops are
   maximally novel and completely useless. This is the central risk and the reason for the
   matched junk control.
3. **"Useful" has no mechanical definition**, so it has to be judged — blind, with controls
   that calibrate the judge (G3).

## Scope, fixed now

**Arity-1 `fn run(x: u16) -> u16` value cells only**, same as EV-1 and for the same two forced
reasons (`Op` threads exactly one free value; the real gate fingerprints arity ≤2 non-state
free functions). Measured against the current library: **79 of the 794 `.rs` files under
`cell80/cells/` are arity-1 `u16 -> u16` value cells.** Those 79 are both the op pool ceiling
and the novelty-comparison set.

Two scope notes that follow from that number:

- `MAX_POOL` is currently 34. EV-2 uses **all 79** — the pool roughly doubles, which on EV-1's
  own Follow-up 4 evidence (A\*'s failure boundary is depth-driven and non-monotonic) changes
  the search landscape and is not a free parameter to tune afterwards. Pinned at 79 now.
- Novelty is measured **over the full `u16` domain**, not over the 22 fingerprint probes. Both
  sides are affordable: `Op::from_cell`'s `Fast` precompute already builds full-domain tables,
  so all 79 library truth tables cost one precompute pass (~5.2M cell evals, CPU, minutes) and
  every candidate's table is free thereafter. No GPU needed at this arity.

## Deviation note carried forward from EV-1

`DEFAULT_PROBES` has grown from **12 probes at EV-1's run to 22 now** (verifier-shape separators,
the finance frequency gate `[2,0,1]`, and the 2026-07-14 overflow separators). EV-1's reported
agreements (0.167–0.833, "fraction-of-12-probes-matching") are therefore **not comparable** to
anything EV-2 reports. Any EV-2 number quoted against EV-1's table is a bug in the write-up.

## The design: two arms and two controls

All four groups are drawn from the same op pool, rendered identically, and pass the same G1
triviality filters. They differ only in how the chain was selected.

| group | n | selection |
|---|---|---|
| **Arm A** (control arm) | 100 | pure novelty — maximize full-domain distance from the nearest library cell |
| **Arm B** (registered primary) | 100 | novelty × parsimony — among non-trivial non-duplicates, minimize real compiled T-states, then depth |
| **Junk control** | 50 | uniformly random chains, length 2–5, no selection at all |
| **Authored control** | 50 | real, already-landed library cells |

Sizes are set by power, not convenience — see the success criterion. 300 items × 2 judges is
cheap (small local models, short behaviour-only prompts), and the reachable chain space is
enormous, so the binding cost is CPU-hours of search, not design compromise.

The junk control is the load-bearing one: **matched to the evolved candidates on representation,
rendering, and every mechanical filter, differing only in having been selected.** If the judge
can't separate Arm B from junk, the search contributed nothing, and no amount of "but look at
this one" in the write-up changes that.

Arm A exists so that a positive Arm B result is interpretable. If A ≈ B, the parsimony pressure
is inert and the honest reading is that the fitness function isn't doing the work the claim
needs it to do.

## The four gates

**G1 — non-triviality (mechanical, pre-judge).** A candidate is discarded if it is constant over
the full domain, is the identity, has an image of fewer than 8 distinct values, is
**extensionally identical over all 65,536 inputs to any of the 79 library cells**, or is
**extensionally identical to another candidate already selected in any group**. The
library-dedup filter is strictly stronger than the real gate's 22-probe check — deliberately,
see the free finding below. The pairwise-among-candidates filter exists because n=100 per arm
is otherwise paddable with near-identical chains, which would inflate an arm without the search
having found anything.

**G2 — the real novelty gate.** Codegen the surviving chain to a single `fn run` source
(`evolved_cells::codegen`, `syn`-based), compile it as a real `Cartridge`, compute
`Fingerprint::of` against `DEFAULT_PROBES`, and require agreement `< DUPLICATE_AGREEMENT` (1.0)
against every arity-1 library cell — the literal mechanism `admission::admit` uses, called on a
candidate outside the gate, never touching `admission.rs` (same fence as EV-1).

**G3 — blind usefulness (the actual experiment).** Protocol below.

**G4 — no-harm on retrieval.** Growing the library can degrade the router; this is the cost side
that a "we found 5 novel cells!" result would otherwise hide. Author retrieval rows for every
G3-accepted candidate, re-run `cell-eval gen-examples` and the retrieval eval, and require the
registered growth-gate floors in `cell-eval/tests/test_retrieval_examples.py` still hold:
**equipped paraphrase P@1 ≥ 0.82, adversarial ≥ 0.82, deployed overall ≥ 0.88, coverage ≥ 0.95.**
If a floor trips, the finding is *"machine-discovered cells cost retrieval accuracy"* — a real
and reportable result. **Existing retrieval rows are never edited to make a floor pass**; a trip
gets re-priced with analysis or stands as a fail.

## The blind judging protocol

The question G3 asks is *"would an agent retrieve this instead of writing code?"* — so the judge
is an agent. This also closes EV-1's one still-open item by promoting it from a nicety to the
gate: *"get retrieval rows and aliasing judgment from someone other than the experimenter."*

- **Presentation is behaviour-only.** Each item is shown as a sampled input→output table plus
  its domain characteristics — **no source, no summary, no tags, no name.** Evolved candidates
  are `codegen`'d let-chains and authored cells are hand-written; showing source would let a
  judge separate them on *style* rather than usefulness, which would silently answer a different
  question. Behaviour-only presentation is what makes the four groups genuinely blind.
- **Judges: two pinned local models** (`gemma4:e4b` and `granite4.1:3b`, the pair the cell-eval
  harness already uses), each judging every item independently, accept = both agree. Order
  randomized under a fixed seed; group membership never in the prompt.
- **Human confirmation, secondary.** The G3-accepted set gets a non-experimenter human pass
  before anything is proposed for real admission. This does not enter the numeric bar.

### Judging runs in two phases, and calibration comes first

**Phase 0 — calibration, controls only, no candidates scored.** 25 authored + 25 junk, drawn
disjoint from everything used in Phase 1. Gate: **≥ 18/25 authored accepted (0.72)** and
**≤ 3/25 junk accepted (0.12)**. Up to **three** judge configurations (prompt wording, model
pair) may be tried against this set. If none passes, the experiment is **VOID** and the reported
finding is *"no calibrated judge was available for this task"* — which is itself a real result
about LLM-judge feasibility on behaviour-only cell descriptions, not a null.

Then **the judge configuration is frozen**, in writing, before any candidate is scored.

**Phase 1 — scoring, single shot.** All 300 items (100 Arm A, 100 Arm B, 50 junk, 50 authored),
disjoint from Phase 0. The scoring set's own controls are re-reported as a drift check against
the same rates (**≥ 0.72 authored, ≤ 0.14 junk**, i.e. ≥ 36/50 and ≤ 7/50). Drift outside those
bounds is **reported, not silently absorbed** — but it does not void Phase 1, because the primary
test uses the scoring junk group as its own comparator and a uniformly permissive judge inflates
both sides of that comparison.

Three things this buys, which a single-phase design does not:

- **A void costs a prompt iteration, not a re-run.** Discovering an uncalibrated judge after
  scoring 300 items means throwing away the run; discovering it in Phase 0 means fixing the
  prompt for the price of 50 judged items.
- **It removes the temptation that matters.** Judge configuration is frozen before anyone can
  see how the arms are doing, so "the judge was too harsh" stops being available as a
  post-hoc move.
- **The one predictable calibration failure gets caught cheaply.** A chunk of the 79 arity-1
  cells are things like `is_even` / `is_zero` — cells whose value is convenience and
  composability, not complexity. A judge asked "is this worth a library slot?" may well reject
  them, which would void the run on a misframed question rather than a real one. The correct
  framing is "would an agent *retrieve* this rather than write it," and Phase 0 is where that
  gets discovered and fixed, inside the three-configuration budget.

**Why the junk ceiling sits at 0.12–0.14 rather than 0.20.** The absolute floor for Arm B is
0.20; a validation ceiling at the same value would let a judge that accepts one noise item in
five satisfy condition 2 by permissiveness alone. Keeping the ceiling strictly below the floor
closes that corner. It is also not a harsh demand given accept requires *both* judges to agree —
the AND-gate puts the effective junk rate well below either model's solo rate.

**Registered double-use of the Phase-1 junk group.** Those 50 items serve as both the drift
check and the null comparator in the primary test. Splitting them further would be cleaner but
halves the comparator and gives back the power the design was just resized to get (25/100 vs
3/25 lands at p ≈ 0.13). Recorded as a known limitation rather than silently reused.

**Authored-control budget.** Phase 0 (25) plus Phase 1 (50) consumes **75 of the 79** available
arity-1 cells. If any prove unusable as controls, the Phase-1 scoring set shrinks first and the
Phase-0 calibration set is held at 25 — calibrating on fewer than 25 is worse than comparing
against fewer than 50.

**Registered circularity note.** Nobody can write "the task this cell solves" without having
seen the cell, so a fully objective demand test isn't available here. Blind-with-matched-controls
is the compromise: it can't prove usefulness in the abstract, but it can prove *discrimination*
— whether selection produced something judges treat differently from noise.

## Success criterion — pinned

**Primary bar, both conditions required:**

1. **Arm B acceptance exceeds junk acceptance at Fisher's exact test, one-sided, p < 0.05.**
2. **Arm B acceptance ≥ 0.20 absolute** (≥ 20/100).

Condition 1 is the real test; condition 2 stops "significant but negligible" from counting
(8/100 vs 0/50 reaches p ≈ 0.03 and is still a library nobody wants). The test, its sidedness,
and the alpha are fixed here; no post-hoc switch to a different statistic.

**An earlier draft of this document set the bar at "≥ 5/20 AND ≥ 2× junk" and that was wrong
in two ways, recorded here rather than quietly replaced.** The two conditions contradicted each
other at plausible junk rates (junk 2/15 → the ratio demands 0.267, which 5/20 = 0.25 fails),
and the ratio's denominator was noisy enough to swing the effective bar between 5/20 and 8/20
on control variance alone. Underneath that, the sizes had no power: 5/20 vs 2/15 is Fisher
one-sided p ≈ 0.4. The groups were resized to 100/100/50/50 to make the predicted effect
(0.25 vs 0.10) detectable at p ≈ 0.03.

- **Fails either condition: FAIL.** Registered conclusion — behaviour-space novelty does not
  track agent usefulness at arity 1; machine library growth needs a demand signal, not a
  novelty signal.
- **Arm A not significantly different from Arm B** (same test, p ≥ 0.05): the parsimony term is
  inert. Report as such rather than as a win for "novelty search," whichever way the bar went.
- Any G3-accepted candidate that trips a G4 floor is reported as accepted-but-costly, not
  quietly dropped from the tally.

## Pre-registered predictions

Written before running, with honest confidence, so the write-up can't retrofit expectations:

- **P1 — Arm A lands at or near the junk rate (≤ 0.20).** High confidence. Maximizing distance
  from every known behaviour is close to a definition of noise.
- **P2 — Arm B beats junk but misses the bar, landing 0.10–0.20.** Medium confidence, and note
  this is a prediction that my own primary bar *fails* — the top of my range sits exactly on
  the absolute floor, so a knife-edge outcome is a live possibility and gets reported as the
  knife edge it is, not rounded either way. Cost pressure selects for short cheap chains, which
  correlates with describability, but I don't think it correlates enough.
- **P3 — the free finding fires: the sweep surfaces ≥1 probe-collision pair** (two chains,
  full-domain distinct, agreement 1.0 on all 22 probes). Medium-high confidence.

## Why this experiment has a floor

P3 is worth stating separately because it makes EV-2 worth running **even if the primary bar
fails outright.**

Full-domain equality implies probe equality, so a genuine extensional duplicate can never slip
past the gate — the gate has no false-*admit* direction here. The reachable failure is the
opposite one: **two genuinely different behaviours colliding on all 22 probes, so the gate
refuses a real cell.** That has happened twice in production and both times by accident during
authoring — `excel_oddlprice` vs `excel_oddlyield` (price refused as a duplicate of yield,
2026-07-11) and `add`/`add_sat` (2026-07-14). Each accident cost a probe-list amendment.

EV-2 sweeps thousands of full-domain-distinct arity-1 behaviours against the library with exact
tables in hand, which is precisely the machine that finds those collisions **on purpose** instead
of waiting for the next authoring session to trip over one. Every collision found is a proposed
new separator probe, and adding probes is monotonically safe (agreement only decreases; a true
duplicate stays 1.0). A quantified false-refusal rate for the admission gate, plus a batch of
new separators, is an on-roadmap deliverable that does not depend on the novelty claim being
true at all.

## What this would not show, even on a clean pass

- **Nothing about arity ≥2**, the more common library shape — a different search representation,
  not a bigger budget.
- **Nothing about whether the accepted cells are the ones worth having.** The judge answers
  "would an agent want this?", not "is this the best use of a library slot?"
- **An LLM judge is not a user.** Two pinned small models agreeing is a proxy for agent demand,
  validated only against controls whose membership the judge might partially recognize (the
  authored controls are public code). The human secondary pass exists because of this.
- **Acceptance is not admission.** The real gate additionally needs retrieval rows and human
  aliasing judgment; G4 measures the cost of adding them but a pass is still "would clear the
  automated bars," never "is merged."

## Build list — what doesn't exist yet

1. **Novelty fitness over full-domain tables** — new; `evolve_from` currently takes
   `examples: &[(u16,u16)]` and scores with `matched`. Needs a fitness entry point taking a
   scoring closure, or a parallel `evolve_novel`. This is the only real change to
   `cell-synth-evolve`'s API.
2. **The 79-cell truth-table precompute + nearest-neighbour distance** — new, straightforward,
   reuses `Fast`.
3. **Real T-state cost per candidate** — EV-1's cost proxy was summed per-op cycles; EV-2 needs
   the compiled candidate's own `Report.cycles`, since C1 proved the difference matters (its
   stage-2 composer bug was ~68 T-states of `CALL`/`RET` overhead, enough to flip a verdict).
4. **The G1 triviality filters** — new, cheap.
5. **The blind judging harness** — new; renders behaviour-only items, shuffles under seed, runs
   both judges, tallies with group membership held out until scoring, and computes the
   registered Fisher one-sided test. Must support the two-phase split (Phase 0 controls-only
   calibration, frozen config, then Phase 1's 300 items × 2 judges) with the phase sets drawn
   disjoint.
6. **G4 wiring** — existing `cell-eval gen-examples` + retrieval commands, no new code.

Reused as-is: `codegen`, `compile`, `build_ops`, `Fingerprint`/`DEFAULT_PROBES`, `evolve`/`mcts`/
`portfolio`. Cost estimate: CPU-only, hours not days; no GPU, no new workspace member — the code
lands in `experiments/evolved-cells/` alongside EV-1.

## Sign-off status

All of it is settled as of 2026-07-27:

- **Primary bar** — Fisher one-sided p < 0.05 (Arm B vs junk) AND Arm B ≥ 0.20 absolute. Set
  after the power problem in the first draft was found.
- **Group sizes** — 100 / 100 / 50 / 50.
- **Judge calibration** — Phase 0 gate ≥ 18/25 authored, ≤ 3/25 junk; three configurations
  allowed, then frozen. Phase 1 drift bounds ≥ 0.72 / ≤ 0.14.

None of these move after the run. A validation ceiling relaxed to rescue a void run is the same
failure as a bar relaxed to rescue a fail, and the three-configuration budget is the whole
allowance for making the judge work.

The pool size (79, all arity-1 cells) and the behaviour-only presentation are load-bearing for
the design rather than tunable; changing either means rewriting this document rather than
amending it.
