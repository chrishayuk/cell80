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

## Amendment refinement (2026-07-13, same day): the capacity hypothesis is confounded on v11

Control 2 above ("top-1 rises with capacity at fixed mechanism") is **not cleanly falsifiable on
v11**, and the amendment as first written overstated it. v11 is pretrained on TinyStories — it has
never seen arithmetic, tool-call syntax, or any structured symbolic invocation. So seen-cell top-1
≈ 6% is primarily a **prior-mismatch** number, not a capacity number: the task is wholly
out-of-distribution for a model whose world is children's stories. If v11 is scaled (more steps,
more unfrozen layers, more data) and top-1 stays low, "rank-without-top-1 is a ceiling" is **not**
a licensed conclusion — the rival explanation "this base has no relevant prior" is fully alive and
untested. Two confounded variables (capacity, prior relevance) with one knob.

**The clean discriminator is a base swap, not a scale-up.** A small model pretrained on code/math
— SmolLM2-135M (size-matched to v11's 115M), or Qwen2.5-0.5B (prior + modest capacity) — is the
same order of magnitude but has actually seen structured symbolic text. If a code/math-pretrained
~135M base converts rank→top-1 where the 115M TinyStories base does not, the binding constraint is
**prior, not parameters** — a far more useful and de-risking finding than "more of the same helps
a bit," and it tells you whether the CUDA path is buying a prior or buying capacity *before* the
spend. Revised control-2 outcomes:
- code/math base moves fingerprint held-out top-1 off zero while v11 does not → constraint is
  **prior**; scale a code/math base, not v11.
- neither base converts rank→top-1 despite the fingerprint rank signal → rank-without-top-1 is a
  genuine ceiling across priors — the strong, still-important null.
- Scaling v11 harder is deprioritized to **last**: it is the one arm where a null is
  uninterpretable.

**Two riders on the swap.** (1) **Re-check weight tying on any candidate base** — the pilot's
precondition (`lm_head.weight == embed.weight`, or the fingerprint row is inert) holds *natively*
in v11 but is not universal; a base with an untied head would silently null the fingerprint arm.
(2) **A frozen-trunk variant** (train only `W_f` + the head, unfreeze zero transformer blocks):
the current run unfreezes top-16 of 20, a large capacity-to-data ratio; if the fingerprint
advantage survives with almost no model adaptation, the mechanism claim is cleaner and stronger
(behaviour-as-address without retraining the network) than after adapting most of it.

**Run order (revised):** shuffled-fingerprint control (running — validates the existing headline)
→ frozen-trunk variant (does the mechanism need trunk adaptation?) → base swap to a code/math
~135M model (the honest prior-vs-capacity test) → 3 seeds on whichever base carries the signal.
Scaling v11 harder is last. The existing v11 result stands exactly as written — "mechanism
confirmed, invocation not yet" is true on v11 and stays true regardless of the swap; the swap
tells you *why* invocation hasn't arrived, not whether the address exists.

## Amendment (2026-07-13): the seen-cell inversion, pre-registered as a prediction before seeds

Seed 80 (top-16, LR decay) produced a **double dissociation** the hypothesis predicts but nobody
wrote down first: the shuffled control BEAT fingerprint on seen cells (top-1 0.475 vs 0.27, median
rank 2 vs 72) while COLLAPSING on held-out cells (median rank 566, worse than chance, vs
fingerprint's 43). The mechanism: behavioural geometry constrains rows to be similar for similar
cells — which costs rank-1 precision on seen cells and buys an address for unseen ones. An
arbitrary structured projection is free to memorize seen cells better yet transfers nothing. This
kills the skeptic's default ("fingerprint just has a better-conditioned init / the shared
projection helps optimization") — that predicts fingerprint ≥ shuffled *everywhere*; the seen-cell
inversion is the opposite.

Because this is currently a one-seed post-hoc story — and this project distrusts those (the EX-2
adoption number, the arms-race timeline) — the prediction is **registered here before the 3-seed
run**, so replication is confirmatory not narrative:

> **Prediction (pre-registered):** in all three seeds, on the same base, at a fixed config,
> (a) fingerprint **underperforms** shuffled on **seen-cell top-1**, and
> (b) fingerprint **outperforms** shuffled on **held-out median rank** (novel_cell × seen_comp),
> with shuffled held-out rank at or worse than chance (≥ ~395).
>
> If (a) and (b) both replicate in 3/3 seeds → mechanism confirmed by a prediction that could have
> failed. If either fails to replicate → the seed-80 dissociation was an artifact, and the
> behavioural reading weakens accordingly.

**Also flagged for examination, not yet reported as a result:** the `novel_cell × novel_comp`
bucket (n=48) shows shuffled median rank 292 — *better* than chance — where on `novel_cell ×
seen_comp` shuffled is 566 (*worse* than chance). That sign flip between the two novel buckets
either means something or means n=48 is too small to report; it is under-powered and must be
resolved (larger bucket and/or the full rank distribution across seeds) before it enters the
findings as anything other than "underpowered, unresolved."

## Amendment (2026-07-13): the mandatory description baseline, and the literature gap

A literature pass relocates this experiment and imposes a required arm. **Corrected novelty
claim:** "cells as vocabulary / tools as tokens" is **not** novel — ToolkenGPT (learn an embedding
per "toolken") and ToolGen (each tool a unique token) established it; we must not claim it. Worse
for a naive reading, the field already documented our *random* arm: ToolkenGPT "cannot use unseen
tools without retraining and embedding updates" and "exhibited a strong bias toward a small subset
of tools it had memorized." Our random/shuffled held-out collapse (rank 519/566, worse than
chance) reproduces that documented failure — external corroboration — and locates it mechanistically.

**The gap the claim actually sits in.** Two literatures have never been connected:
- *Tool-learning* solves unseen-tool generalization exclusively through **language about the tool**
  — documentation/description comprehension (GenTool, TOOLVERIFIER, Re-Invoke, RaTA-Tool; SOTA
  **CoTools** selects from natural-language tool descriptions over a frozen LLM; Tool2Vec is
  usage/query-derived — still language, not behaviour).
- *Program-embedding* learns representations from **execution traces** (DYPRO, LiGer, Trex, sem2vec)
  on the premise that syntactically similar programs behave differently — but only for program
  analysis, **never as an LLM's token embedding.**
Our conjunction — *executed behaviour as the tool token's address, giving unseen tools zero-shot
addresses* — is the unoccupied intersection. The precondition for it (exhaustive, cheap execution
of the whole library to compute every address) is exactly what the GPU interpreter buys; it is not
a footnote to this result, it is what makes it possible.

**Required arm (d) — description embedding, a STRONG baseline (mandatory, not optional).** Because
CoTools/description-routing is the state of the art for *this exact claim*, the central question is
now **does behaviour beat language as a tool address?** Arm (d): each cell token's row =
`W_d(sentence_encoder(descriptor/doc))` — same three-way-tied machinery as the fingerprint arm, but
the address is derived from the cell's *description text* (encoded with a real sentence encoder,
`bge-small-en-v1.5`, cached) instead of its behavioural fingerprint. Registered outcomes, on
held-out cells:
- **fingerprint > description** → "for programs, *what it does* addresses better than *what it's
  called*" — novel, defensible, lands in the gap.
- **≈ equal** → "behaviour is a *description-free* address — needs no docs, no naming discipline,
  and is computable for machine-synthesized cells that arrive with behaviour and no prose" — still
  valuable, and exactly this library's case.
- **description > fingerprint** → honest, much weaker result, and **this is the outcome to
  pre-register against because it is live**: our cells have decent names and a sentence encoder is a
  very strong prior. Registering it now means we cannot quietly drop the description arm if it wins.

**The killer experiment (the synthesized-cell ace).** A cell minted by cost-discovery or evolution
has **no documentation** — so description-based routing (every literature method) is structurally
blind to it, while behaviour-based routing still computes an address. Pre-registered: a held-out
slice of **description-stripped cells** (name + docstring removed, only behaviour available); the
prediction is fingerprint addresses them at the same rank quality as documented cells while arm (d)
falls to chance. If it holds, it is the one experiment no description method in the literature can
match, by construction.

**Order:** the running swap answers top-1 (prior vs capacity); arm (d) and the synthesized-cell
slice are added to the v11 and swap comparisons (same `W_?` machinery, one more arm); 3 seeds
throughout. The paper's spine is **behaviour vs language as a tool address**, not "tools as tokens."

## Amendment (2026-07-13): retire novel_novel; provenance discipline; strong description arm

**Both CN-1 bugs were found by a consistency check, not a failing test** — the untied head (pilot)
and the dropped `norm` (this build) each surfaced because *two routes to the same number
disagreed* (arm-vs-arm signature; reload-vs-training-time rank). Standing method for this lane:
compute load-bearing numbers by two paths and treat any disagreement as a defect.

**Provenance discipline.** The faithful metric is the live-model training-time eval (median rank).
The top-10% (rank<79) fractions were computed by a checkpoint-reload path that (until the norm fix)
ran a slightly-unfaithful model; those fractions are **parked, not reported as headline figures**,
until the 3-seed runs supply rank *and* fraction from one faithful path. Quote faithful ranks now.

**Retire `novel_cell × novel_comp` (n=48) as a reported bucket.** It is not merely small — it is a
**deterministically-selected, non-random subset** (axis-A cells that happen to intersect the fixed
held-out (template, pack) pairs). That is a biased draw, so more seeds make the biased estimate
*more precise*, not more correct. A valid novel_novel test would require **randomizing the axis-B
(template, pack) held-out selection across seeds** (i.e. a different data split per seed), which is
a different experiment. Absent that, novel_novel is uninterpretable by construction and is retired.
The three well-powered buckets carry the result: seen×seen, seen×novel_comp (n=624, the composition
axis for gate (i)), and **novel_cell×seen_comp (n=200) — the gate-(ii) signal**: fingerprint 43,
shuffled 566, random 519, chance 395. The two null arms bracket chance from the wrong side;
fingerprint is an order of magnitude better. That is the whole result and it does not need
novel_novel to stand.

**The description arm must be the STRONG version of the competing idea.** CoTools/description-routing
is literally the state of the art for this claim, so arm (d) must not be a bag-of-name-words strawman:
the address is a real sentence encoder (`bge-small`) over the **richest available** description of the
cell (expanded operation words + typed signature + family), projected through the same `W_f`-shaped
machinery. A weak description arm that loses proves nothing. Framing when it lands:
fingerprint > description is the exciting outcome; **fingerprint ≈ description is still a strong
result for us**, because the synthesized-cell ace (undocumented cells) is one no description method
can address by construction — that arm must exist in the final table.

## Amendment (2026-07-13): how to read the swap, and both paper framings — BEFORE the number

Registered before the SmolLM2 swap lands, so the reading is not written under the influence of
the result.

**Two numbers from the swap, in order.**
1. **Held-out top-1 ≠ 0** — the prior-vs-capacity answer. Does the rank signal (fingerprint held-out
   rank 43 on v11) convert to landing rank-1 on a base that has a code/math prior?
2. **Does the seen-cell inversion replicate at the new base?** — the base-independence test, and the
   one nobody thinks to check. On v11 the mechanism revealed itself as a *dissociation*: fingerprint
   was WORSE than shuffled/random on seen top-1 (0.27 < 0.475 < 0.785) yet far better on held-out.
   The pre-registered prediction applies **within the SmolLM2 base too**: fingerprint should
   underperform shuffled on seen top-1 and outperform on held-out rank.

**The trap (registered so it can't be explained away post-hoc).** SmolLM2 fingerprint showing a
strong *seen* number (0.39 by step 3200 vs v11's 0.22 final) is **not by itself evidence for the
mechanism** — the comparison is within-base, across-arms. If fingerprint instead *wins on both
axes* at SmolLM2 (beats shuffled on seen AND held-out), that is **informative in the wrong
direction**: it would suggest the base's prior is doing the addressing and the fingerprint is
merely a decent initialization — which *weakens* the mechanism claim, not strengthens it. The
dissociation, not the level, is the evidence.

**Both paper framings, written before the result (so the title isn't chosen by the number):**

- **Framing A — "behaviour beats language as a tool address."** Fires if fingerprint > description
  on held-out (documented cells). Title ≈ *Behaviour as Address: executed-behaviour embeddings
  outperform documentation for zero-shot tool invocation*. First figure: the held-out
  rank/top-1 bar, fingerprint vs description vs the null arms, across bases. The synthesized-cell
  arm is a strong supporting table (behaviour also works where description is blind).

- **Framing B — "behaviour is a *language-free* address."** Fires if description ≥ fingerprint on
  the documented library (live risk: clean names + structured signatures + strong encoder). Title ≈
  *A Description-Free Address for Synthesized Tools: executed behaviour invokes cells that have no
  documentation to route on*. First figure: the **synthesized-cell** result — behaviour addresses
  undocumented cells at documented-cell rank quality while every description method sits at chance,
  by construction. Here the synthesized-cell arm is not a supporting table, it is the whole paper,
  and it is the case this library is *heading toward* as cost-discovery/synthesis mint cells with
  behaviour and no prose. Still strong; a different paper, different first figure.

Both framings keep the honest scope: mechanism (rank) is established; invocation (top-1) is what the
swap tests; the synthesized-cell ace is the arm no description method can match either way.

## Amendment (2026-07-14): top-1 is the wrong bar — behaviour-as-address resolves a NEIGHBOURHOOD

The top-k confusion analysis (`cn1_confusion_analysis.py`) settles what the held-out plateau means.
On held-out cells the plateau is median rank ~21 / 88% top-10% / top-5 0.18 / **top-1 0.000**, and
the cells ranked ABOVE the true cell are its **behavioural siblings**: mean fingerprint agreement
0.436 vs 0.065 random (6.7×), same-family rate 0.112 vs 0.027 (4.1×). This is structural, not
capacity: fingerprints place behaviourally-similar cells adjacent, so the geometry that lands the
neighbourhood (rank 21) is the same geometry that fills ranks 1–20 with near-identical cells. The
mechanism's strength IS its top-1 ceiling.

**Superseding interpretation (registered before the faithful re-run lands).** The pre-registered
gate-(ii) bar `(c) ≥ 0.5` top-1 is **retired as the success criterion for this mechanism** — it
mis-specifies what behaviour-as-address delivers. The correct criterion is a **two-tier** one, and
it is the substrate's own F2 design:
1. **Neighbourhood localization (the model's job):** the trained fingerprint arm ranks the true
   held-out cell within a small behavioural neighbourhood — operationalized as **top-k for
   k≈10–20** (held-out top-10% ≈ 0.88 already), and the confusions being genuine behavioural
   siblings (agreement ≫ chance), not junk.
2. **Execution disambiguation (the runtime's job):** the shipped fused behavioural router (0.859,
   `search_with_examples`) resolves the true cell *within* the model's top-k by executing
   candidates — no top-1 from the model required.
**New pre-registered gate (ii′):** end-to-end, model-top-k → router-disambiguates recovers the
true held-out cell at rank-1 at a rate the router's own ceiling allows (target: within ~10 points
of the router's equipped-query accuracy), while the shuffled/random arms — whose top-k is NOT a
behavioural neighbourhood (confusions at chance agreement) — cannot, because the router has no
correct-neighbourhood to resolve within. This is falsifiable both ways and it is the claim the
substrate is built to deliver.

**To build (next analysis, cheap, on existing checkpoints):** the two-tier eval — take the model's
top-k for each held-out case, run `cell80_py.search_with_examples` over those k candidates with the
cell's I/O examples, measure rank-1 recovery; contrast fingerprint vs shuffled/random top-k. Also
report the residual (swap_bytes-type cells whose top-k is junk — genuine capacity misses) so the
neighbourhood claim is scoped, not oversold. The faithful re-run supplies the arms; the confusion
+ two-tier analyses run on its checkpoints from one faithful path.

## Note on gate (ii′) being a correction, not a moved goalpost (2026-07-14)

The bar moved from top-1 ≥ 0.5 to a neighbourhood bar (top-k recall + the confusion-similarity
statistic + two-tier execution resolution) **because the old bar was shown to be
mechanism-forbidden, not because it was missed.** The evidence chain is explicit and pre-committed:
one mechanism (fingerprints place behaviourally-similar cells adjacent) makes two predictions —
(a) the seen-cell inversion (fingerprint loses at rank-1 to shuffled/random, 0.22 < 0.42 < 0.74)
and (b) the held-out plateau (rank ~21, 88% top-10%, top-1 0.000 because ranks 1–20 are siblings) —
and **both are confirmed, (a) predicted before the data.** The confusion analysis then measured the
cause directly: cells beating the true cell are 6.7× more behaviourally similar and 4.1× more
same-family than chance. So rank-1 within the behavioural neighbourhood is the *one thing
behavioural geometry cannot do by construction*; a top-1 bar tests against the mechanism instead of
for it. This correction follows the same discipline this programme used for the ±20%-flatness
repricing gate and the pilot's "0.000 is a floor, not a comparison" note: the criterion is fixed to
what the evidence shows the mechanism delivers, and the reason it changed is recorded so it reads as
a correction on evidence. The claim reframes *upward*: not "the model picks the right unseen cell"
(mechanism-forbidden) but "the model locates an unseen cell's behavioural neighbourhood; execution
resolves the rest" (the substrate's F2 two-tier design, arriving as empirical necessity).

## Amendment (2026-07-14): scale-invariance is the real claim; execution verifies; CN-6 is the critical link

The two-tier pipeline demo (`cn1_two_tier.py`) confirmed tier-2 works: whenever the true held-out
cell is in the model's top-k, execution recovers it exactly (resolved == recall, no false
resolutions). But an honesty correction: **per-cell recall is lower than the per-item median-21
implied** — over 24 distinct held-out value cells, top-50 recall was 0.25 (the per-item median-21
over-weighted cells with many well-ranked items). The rank level is softer than the headline; the
confusion *mechanism* (siblings beat the true cell, 6.7×) is robust regardless.

**The real value of the address is scale-invariance, not correctness — and it is untested.** At 790
cells, executing the whole library against a probe set is ~1 ms, so the fingerprint buys nothing a
brute-force scan doesn't. Its value only appears at library sizes not yet reached. The
**load-bearing question**: does held-out rank hold **absolutely** or **fractionally** as the library
grows? Median rank 21 of 790 = 2.7%; at 10⁶ cells "top 2.7%" = 27,000 candidates (un-executable
per token) whereas "rank 21 absolute" is trivially executable — two completely different
architectures. **The decisive experiment: the retrained library-scale curve** (the 114→788,
21-checkpoint growth already run for text retrieval), measuring fingerprint held-out rank at each
size. **Post-hoc random subsampling cannot substitute** — random removal makes rank trivially
fractional. Model-free proxy run today: tight behavioural siblings (probe-agreement ≥ 0.8) per
held-out cell are few — median 0, mean ~3, max 25 — so *near-duplicates* are bounded and rare; but
whether the model's broader ~0.44-agreement neighbourhood saturates or grows with library size is
exactly what the retrained curve must show. **If absolute → behaviour-as-address scales and the
pipeline is real; if fractional → a second-stage prune is required.** This is the experiment that
decides whether the mechanism delivers at the size the pitch claims.

**Execution buys two things beyond picking (name them):** (1) it **verifies, not just selects** —
running the top-k against the query's examples confirms the winner reproduces the required
behaviour, so a wrong pick is *detected*, not silently returned (CN-2's guarantee, arriving in the
retrieval layer); (2) a **total miss is detectable** — if none of the k candidates reproduce the
examples, that is a **work order**, and demand-driven synthesis triggers exactly there. The loop
closes: address → execute → resolve, **or → mint**.

**The critical dependency — CN-6 is now the most important unrun experiment.** Execution can only
resolve if the query carries something *executable to check against*. The fused router hits 0.859
on **equipped** queries (I/O examples) but only 0.387 on text-only paraphrase — so the entire
pipeline rests on the model emitting **examples, not intent** ("the thing where (498,500,10)→500",
not "a discount calculator"). That is CN-6 (behavioural-spec emission), untested. It is the one
thing standing between a confirmed *mechanism* and a working *system*, and it moves to the top of
the queue after the faithful arms + the library-scale curve.

## RETRACTION (2026-07-14): the neighbourhood-bar reframe is withdrawn — the sibling data doesn't support it

The gate-(ii′) / "top-1 is mechanism-forbidden" reframe registered above is **retracted**. It is
left in place as a record of the reasoning trail (and of a goalpost-move caught and reversed on
evidence), but it does **not** stand as the criterion. The correction:

- The structural-ceiling argument claimed ranks 1–20 are near-identical siblings the true cell
  "hides behind." The data says otherwise. Model-free proxy: at probe-agreement ≥ 0.8 a held-out
  cell has **median 0, mean ~3** near-duplicates; exact duplicates essentially don't exist. The
  model's *confusions* sit at **~0.44** agreement — "loosely related," not sibling. 6.7× chance is
  real enrichment, but chance was 0.065, so 6.7× is still a small absolute similarity. **Nothing
  structurally forbids rank 1** — the true cell typically has zero genuine near-duplicates.
- The per-item median-21 measured the wrong unit and flattered. The honest usable-level number is
  **per-cell top-50 recall 0.25** over 24 distinct held-out value cells — three quarters of new
  cells are not found even in the top 50.

**Honest registration (this supersedes gate (ii′)):**
- **Mechanism: confirmed.** Fingerprint ≫ shuffled ≈ random on held-out; seen-cell control clean;
  double dissociation intact; 6.7× behavioural-sibling enrichment intact. Behaviour-derived
  geometry does something random geometry cannot.
- **Usable level: insufficient.** Per-cell top-50 recall 0.25. The address places cells in roughly
  the right region but does not resolve.
- **Top-1: not structurally excluded.** The original bar is back in play; it was retired on a bad
  argument.
- **Cause of the insufficiency: undetermined**, among (a) model capacity, (b) **fingerprint
  resolution** — the 20-probe battery may be too coarse to separate cells at ~0.44 agreement, and
  (c) corpus. (b) is the interesting, cheap, newly-testable one: if near-duplicates are genuinely
  rare in *true* behavioural space, a richer probe battery should push the ~0.44 confusions apart.
  This is a *fingerprint-quality* experiment, not a model experiment — and we own the probe set.

**Kept (independent of the retraction):** the scale-invariance point (absolute-vs-fractional
held-out rank as the library grows — the retrained library-scale curve) and **CN-6 (emit examples,
not intent) at the top of the queue.** Tier-2 execution also stands: when the true cell is in
top-k, execution resolves it exactly, no false picks.

**Next, in order:** (1) the **probe-richness sweep** — model-free: does a richer fingerprint
separate the ~0.44 confusions? If it materially moves per-cell recall, the address can sharpen and
top-1 returns as a live target. (2) The faithful arms (running). Then the library-scale curve and
CN-6.

## Amendment (2026-07-14): probe-bias control — over-merge is real but modest (retrain deprioritized)

The winner's-curse control ran before any probe-richness retrain (the pre-planned "is this win an
artifact of how I measured?" instrument, same shape as the P=0 lane and the permutation null). On
3000 *random* same-arity cell pairs (no selection): 20-probe agreement 0.1625 vs independent rich
(1980 probes) 0.1293 — **mean over-merge +0.033, median +0.018.** So the 20-probe battery genuinely
runs high, but modestly; the sweep's larger 0.344→0.245 drop was mostly selection regression. The
earlier "~28% over-merge" is retracted (~3× inflated). Consequence: the probe-richness retrain is
**defensible but low priority** — a ~3-point address bias is unlikely to move per-cell recall
much, and the plateau result (confusions genuinely ~0.245-similar) already says the information is
there and the *model* isn't using it. Order held: **faithful arms (in flight) → library-scale
curve → CN-6**; probe-richness retrain only if a cheaper cause is ruled out first.

## Amendment (2026-07-14): first-N eval-sampling bug — median rank and enrichment corrected

Prompted by the 0.065-vs-0.1625 null discrepancy: two routes to the same quantity disagreed, and
chasing it exposed that the eval scored the **first 200 items** of a **cell-grouped** eval file —
over-weighting whichever held-out cells come first. Corrections (robust random sample):
- **held-out median rank ~21 → ~114 of 790** (n=150 random: median 114, mean 209);
- **confusion enrichment 6.73× → ~2.7× median / ~4.2× mean** vs the all-790 null (agreement function
  verified identical across routes, diff 0.0000; both nulls correct for their population; confusions
  are 37% same-arity, 18% state, so all-790 is a reasonable comparator).

**Survives:** the arm contrast (fingerprint ≪ shuffled ≈ random on held-out) used identical items
across arms, so the double dissociation stands; only the *absolute* levels were inflated. **Net:
mechanism real, usable level weaker (median rank ~114, per-cell top-50 recall 0.25).** Fix:
`cn1_eval_ckpt.py` now shuffles (fixed seed) before capping; the in-flight training evals are left
untouched so the running re-run's arms stay mutually consistent (contrast-valid), and authoritative
*absolute* numbers come from a random-sampled re-eval of the faithful checkpoints post-run. This is
the third ratio-vs-mismatched-population catch in the programme (after EX-2's drift baseline) and
the fourth "is this number an artifact of how I measured it?" catch overall.
