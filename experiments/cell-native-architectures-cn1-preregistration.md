# Pre-registration: CN-1 real build — does a trained invocation reflex beat prompting, generalize to cells never seen called, and add value beyond verified decoding?

Status: **pre-registered before any training run.** This document fixes the architecture
preconditions, the corpus design, the arms, the baselines, and the success/kill criteria
first, in the same discipline as `evolved-cells-preregistration.md` and
`cell-cost-discovery-preregistration.md` — so the result is judged against what we said
would count. It supersedes nothing in doc 16 (`cell-native-architectures.md` §CN-1); it
*instantiates* CN-1's real build with the constraints the slice-0 pilot earned and one gate
the pilot's era could not have named (gate (iii), below).

## The claim being tested

A model can learn to invoke cells as vocabulary — to specify a computation and receive a
verified, hash-attested, replayable answer — such that:

1. the trained reflex beats prompting at equal parameter count (**gate (i)**, the H2 rule);
2. behaviour-derived (fingerprint) embeddings give never-seen-called cells *meaningful
   addresses*, which learned-from-scratch embeddings cannot by construction (**gate (ii)**,
   the novelty gate — the claim that makes invocation library-size invariant); and
3. the trained reflex does something the already-shipped inference-time correction loop
   (CN-2 G2) cannot do without any training at all (**gate (iii)**, new — the beyond-G2
   gate).

Gate (iii) exists because CN-2 changed the baseline under this experiment's feet: verified
decoding already drives the scoped wrong-number rate to zero at ~4.8% wall-clock overhead
on the GSM8K battery, where the model's scoped arithmetic was already 98.4% right before
correction. A trained invocation reflex evaluated only on scoped arithmetic would be
competing against a deployed system that is exact there for free. Its value must therefore
be claimed — and measured — where G2 cannot reach, or the experiment can "pass" while
meaning less than it appears to. That failure mode is named here, before the spend, so the
write-up cannot be reshaped around it.

## What the slice-0 pilot fixed, now binding (not re-derived)

Three iterations of the toy pilot (findings, `## CN-1 slice-0`) bought three constraints
for the cost of a toy. They are **preconditions** of this build, not open questions:

1. **Weight tying is a precondition.** A fingerprint-placed embedding can only influence a
   prediction in a model whose output projection shares weights with its input embeddings.
   An untied build scores 0.000 on both arms and reads as a broken harness.
2. **Compositional coverage is a corpus requirement.** "Shares vocabulary" is not enough;
   held-out combinations are only a fair test when every individual factor has heavy,
   multi-partner exposure (iteration 2's lesson, confirmed necessary-but-not-sufficient at
   toy scale by iteration 3).
3. **Gate (ii) is untested, not failed.** The pilot's 0.000/0.000 was a capacity floor —
   neither arm reached a decision point the embedding could influence. It is not evidence
   against fingerprint embeddings and is not cited as such here.

## Architecture, fixed now

**Model.** TinyModel v11 (the pre-registered scale for CN-1), **PyTorch on M3/MPS** (the
model is PyTorch, not MLX — the pilot's toy was MLX, TinyModel v11 is not; corrected here
after the infra map). 115M params, dim 512, 20 layers, vocab 71261, and **weight tying is
native** (`model.py:136`, `lm_head.weight = embed.weight`) — the pilot's precondition holds
for free. The `v11.vocab.bin` tokenizer is extended by append-only re-serialization (its
`Vocab::save` writer is public; `u32` ids, no runtime ceiling), adding ~790 cell-identity
tokens + call-grammar delimiters at the tail so existing rows and trained embeddings are
untouched — this is in-scope engineering, the detour the pilot deferred. Result tokens
spliced by the runtime at zero decode cost. **Must be written** (none exist in-repo): an
embedding-resize utility (`load_state_dict` is `strict=True`), a checkpoint-resume path, and
the autoregressive generate loop that hosts constrained decoding.

**Three-way tying (the pilot's precondition, sharpened by one step).** The pilot proved
input/output tying is necessary. Library-size invariance requires more: the *output head*
rows for cell tokens must also be fingerprint-derived, or a held-out cell is unspeakable no
matter how good its input embedding is — and gate (ii) would fail for a boring mechanical
reason indistinguishable, in the score, from the hypothesis failing. Registered design:

- One shared projection `W_f : fingerprint → d_model`, learned end-to-end **on seen cells
  only**, produces each cell token's embedding row. That same row serves as the token's
  input embedding *and* its output-head row (tied). At eval, a held-out cell's row is
  computed by applying the trained `W_f` to its fingerprint, frozen — the cell is thereby
  both readable and emittable.
- The random arm keeps identical tying but replaces `W_f(fingerprint)` with free per-cell
  learned rows. Held-out cells in this arm get rows that were never trained — which is
  exactly the ablation: doc 16's "the only mechanism by which unseen cells have meaningful
  addresses."
- Fingerprints are the behavioural fingerprints over the shared probe battery
  (`dump_fingerprints`), computed identically for seen and held-out cells.

**Constrained decoding admits held-out cells.** The decode-time grammar masks to the known
hash set *including* every axis-A held-out cell. Emission of a held-out cell is therefore
possible by construction; gate (ii) measures *selection*, not vocabulary membership. (This
also keeps gate (ii) distinct from CN-6, which owns the case the grammar cannot admit:
cells minted after training, resolved by spec emission + text routing.)

**Inline verification (doc 16 §3 control).** Every spliced result is verified against the
reference interpreter in-line; a mismatch anywhere is a filed defect and stops the run.

## Corpus, fixed now

**Two sources, mix ratio reported (procedure registered, ratio measured not invented):**

1. **H1 factory synthetics** — chuk-math-gym + GSM8K-style + library-family templates,
   strict-improvement filter against the exact oracle, admission-style dedup on call sites,
   `trapped_ops` in every filter term.
2. **CN-2 harvest** — verified-decoding traces are already (context, refuted claim,
   verified call, exact result) tuples produced as a side effect of G2. Every harvested
   example is a place a real model *actually* got the number wrong and a cell fixed it —
   training signal concentrated on the residual G2's economics care about, not where a
   template generator guesses failure might live. The G2 correction loop is thereby the
   experiment's Toolformer filter, already built and already exact. The harvest requires
   running the CN-2 harness over a larger battery than the committed 60 problems (2
   corrections total is a proof of mechanism, not a corpus); the harvest battery is drawn
   from the same generators as source 1's problems but disjoint from every eval battery
   below, and its size is reported.

**Two held-out axes, factorized, never conflated:**

- **Axis A — held-out cells (gate (ii)'s axis).** A stratified random ~10% of the cell
  vocabulary (stratified by pack/family so no family is entirely held out), drawn once
  before corpus generation, hashes recorded in the findings *before* training. These cells
  never appear as call targets anywhere in training (either source). Their fingerprints
  exist; their rows are derivable through `W_f`; the decode grammar admits them.
- **Axis B — held-out compositions (gate (i)'s generalization axis).** Held-out
  combinations of surface/context template × cell family where both factors appear in
  training with multiple partners but never together — the pilot's compositional-coverage
  requirement, applied for real. Held-out *families* per doc 16 §3 live on this axis.

The axes cross into four eval buckets — seen-cell × seen-composition, seen-cell ×
novel-composition, novel-cell × seen-composition, novel-cell × novel-composition — and all
four are reported. A pass or fail that cannot be attributed to one axis is not a result;
the factorization is what makes attribution possible, and it is cheap now and impossible
after the corpus is generated.

**Eval battery sizes, fixed:** ≥200 items per bucket, generated before training, disjoint
from both corpus sources and from the CN-2 harvest battery.

## Arms, fixed now

| Arm | Description |
|-----|-------------|
| (a) | no-cells baseline, matched parameters, same corpus with calls stripped |
| (b) | random-init tied cell rows (free learned embeddings) |
| (c) | fingerprint-init: three-way-tied `W_f` rows |
| (P) | prompted `cell_solve` baseline on the same base model — gate (i)'s comparator |
| (G) | **G2-at-inference**: the base model with *no invocation training*, verified decoding running downstream — the deployed alternative, gate (iii)'s comparator |

Arms (b) and (c) train **3 seeds** each; gates are evaluated per-seed and pass on a
majority (≥2/3). The pilot was one seed; the real build does not inherit that limitation.

## Gates, pre-registered

**Gate (i) — the H2 rule.** Arm (b) or (c) beats arm (P) on final-answer accuracy over the
full battery *including* the novel-composition buckets: absolute margin ≥5 points on the
novel-composition slice and non-overlapping 95% CIs by paired bootstrap over items.

**Gate (ii) — the novelty gate.** On the novel-cell buckets, scoring correct-hash-emitted
at the call site with exact arguments: arm (c) ≥ 0.5 **and** (c) − (b) ≥ 0.25 — the pilot
iteration 3 bar carried forward to the build it was always waiting for. Arm (b) is expected
at floor by construction; if arm (b) scores *high* here, that is a contamination alarm
(axis-A leakage into training), not a result, and triggers a corpus audit before anything
is read.

**Gate (iii) — the beyond-G2 gate.** Operational split, decided by machine not judgment:
an eval item is **G2-reachable** iff running the CN-2 span grammar + verifier over arm
(P)'s and arm (G)'s transcripts for that item finds a verifiable claim span covering the
target computation; otherwise **G2-unreachable** (answer-only formats, cells outside the
arithmetic claim grammar — bit ops, envelope/finance cells, string-adjacent cells — and
computations the model never writes as a scoped equation). Both slice sizes are reported;
if the unreachable slice is <100 items the battery is extended before evaluation, not
after. The gate: on the G2-unreachable slice, arm (c) beats arm (G) by ≥10 points absolute.
On the G2-reachable slice arm (c) must merely not lose to arm (G) by more than 2 points —
no value is claimed there, because arm (G) is already exact on scoped arithmetic for free
and pretending otherwise is the failure mode this gate exists to prevent.

**H2 shortcut band — split by G2-reachability, not pooled.** Delegation rate is measured
separately on the two slices. On G2-reachable-easy cases, shortcutting is *rational* (a
guess gets corrected downstream at ~5% overhead): the accepted band is wide, 0–90%
delegation, and no gate hangs on it. On the G2-unreachable slice, where a wrong guess stays
wrong, the pre-registered floor is ≥50% delegation on cases where the no-cells baseline
(arm (a)) is wrong — delegation precisely where it is strictly improving. A single pooled
delegation number averages a rational shortcut with the meaningful preference and is
uninterpretable; it is not reported without its split.

**Named measurement, not a gate: latency.** Tokens-to-answer for a delegated call vs. the
model generating the equivalent chain, and wall-clock vs. arm (G)'s correction overhead.
Either direction is a result; delegation-is-cheaper is expected but not required.

## Kill conditions, pre-registered

- **Gate (i) fails** → trained invocation doesn't beat prompting at this scale; re-scope
  before any depth-2/3 training spend (doc 16's kill, unchanged).
- **Gate (ii) fails with (i) passing** → fingerprints don't transfer to embedding space at
  a scale with real capacity; the "computed vocabulary" claim dies (this time for real —
  unlike the pilot, this build has no capacity-floor excuse: gate (i) passing *is* the
  proof that the model reached decision points embeddings can influence). Depth-2 routing
  on F2 still stands; CN-6's text-routing lane becomes the only novelty mechanism.
- **Gate (iii) fails with (i) and (ii) passing** → the reflex is real and generalizes, but
  adds nothing the inference-time loop doesn't already provide — the training spend buys
  guarantee-parity, not capability. CN-7 (chains) does not proceed on the strength of this
  result alone; the value claim moves entirely to domains G2's grammar will never cover,
  and that re-scope happens before, not after, the CN-7 spend.
- **Both (b) and (c) at floor on all buckets** → the pilot's capacity story reproduced at
  real scale; that would be genuinely surprising (this model demonstrably learns harder
  things) and would point at the harness first, per the pilot's own lesson: a 0.000/0.000
  is read as "broken harness" until proven otherwise.

## What this would NOT show

Nothing about: cells minted after the training corpus freezes (CN-6's territory, by
design — axis-A cells are in the grammar and the fingerprint distribution; post-freeze
cells are not even in the grammar); multi-cell chains with exact intermediates (CN-7,
deliberately gated behind this experiment); depth-2/3 integration (CN-3 scoped out for
Gemma-class, CN-4 unaffected either way except that the three-way-tying finding constrains
its result-projection design); Gemma-class models (layer maps and tying conclusions do not
transfer, doc 16 §3); scale beyond TinyModel-class (gate (i)'s comparison is
matched-parameter, always).

## Method reuse

H1 factory (corpus + strict-improvement filter, exact oracle) · CN-2 harness
(`cn2_g2_resample.py` lineage) as the harvest engine and the gate (iii) reachability
classifier · `dump_library` (all 790 cells' identity + probe-battery fingerprints in one
JSONL, superseding the per-name `dump_fingerprints`) for `W_f`'s inputs · LARQL's
`OpNameMask` closure seam (`FnMut(ids, &mut logits)`, applied after dense LM-head scoring,
before sampling) ported into the TinyModel PyTorch generate loop for constrained decoding
(real work, in scope, the pilot's named deferral) · reference interpreter for in-line splice
verification. Harness: `experiments/cell-native-architectures/`.

## Order of operations (so no number is seen before its bar is fixed)

1. Freeze this document. 2. Rebuild the tokenizer; port constrained decoding to MLX;
verify the harness end-to-end on a smoke slice with gates *not* evaluated. 3. Draw and
record axis-A cells. 4. Run the CN-2 harvest battery; generate the corpus; record the mix
ratio and coverage stats. 5. Generate eval batteries; classify G2-reachability from arms
(P)/(G) transcripts. 6. Train arms; evaluate; write findings against this document.

## Amendment (2026-07-13): two controls pre-registered before the stronger run

The first full run produced a positive gate-(ii) *mechanism* signal (fingerprint held-out
median rank 56/790, 65% in the top 10%; random 619, 0%; seen-cell control comparable, ruling
out a general arm difference). Before spending on a stronger run, two controls are pre-registered
here — as hypotheses with their falsifying outcomes stated first — because they are exactly where
a skeptic goes.

**Control 1 — the shuffled-fingerprint arm (is the signal behaviour, or just the projection?).**
The fingerprint arm's held-out advantage is controlled for *contexts* (both arms share the
descriptor corpus) but not for *information*: descriptors come from cell names, fingerprints from
behaviour, and behaviourally-similar cells often have similar names, so name-similarity could leak
through. New arm **(s) shuffled**: identical `W_f` and geometry, but each cell is assigned a
*different* cell's fingerprint (a fixed seeded derangement, `SHUFFLE_SEED=1234`, no cell keeps its
own). Trained and evaluated exactly like arm (c).
- **Pre-registered reading:** the signal is genuinely behavioural **iff** shuffled held-out
  ranking collapses toward random — concretely, shuffled held-out top-10% fraction ≤ 0.15 (vs
  fingerprint's 0.65) and median rank closer to random's than to fingerprint's. If instead
  shuffled stays high (≈ fingerprint), the effect is the shared projection layer / name geometry,
  **not** behaviour, and the headline must be restated as "a structured shared projection gives
  unseen cells addresses" — a weaker, still-real claim. This is the one control that would let a
  reviewer dismiss the behavioural reading, so it runs alongside (c) and (b) at one fixed config.

**Control 2 — the capacity hypothesis (why is top-1 zero?), stated as a hypothesis not an
assumption.** The mechanism is confirmed as a *ranking* signal; held-out top-1 is 0.000 because
the model is under-powered (seen top-1 ≈ 0.06–0.22). The plan "convert rank→top-1 with more
capacity" is a **hypothesis**, pre-registered as: *top-1 rises with training/model capacity at
fixed mechanism.*
- **Pre-registered outcomes:** (a) if a stronger run raises seen top-1 **and** moves fingerprint
  held-out top-1 from 0 to non-zero (with shuffled/random still ~0), the hypothesis holds and gate
  (ii) approaches its pre-registered bar. (b) If seen top-1 rises but fingerprint held-out top-1
  stays 0, then **rank-without-top-1 is a ceiling, not an artifact** — a distinct and important
  finding (the address is findable but not winnable), which no amount of the same scaling fixes.
- **Honest scope of this run:** it varies *optimization* capacity on the fixed 115M base
  (unfreeze depth, steps, LR decay for the late-run instability the top-12 run showed). True
  *model*-capacity scaling (a larger base) is future work (CUDA), and outcome (b) would motivate
  it specifically.

Both controls run at one fixed config across arms (c)/(s)/(b) so the comparison is clean; then 3
seeds per surviving arm for the gate proper.
