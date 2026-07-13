# 16 — Cell-Native Model Architectures: the experiment programme (draft v0.1)

**Status:** draft for review · **Depends on:** docs/14 (WS-F passed 2026-07-11 checkpoint 21; WS-E interpreter substrate), cell80-py, H1 factory (spec'd), LARQL (117 tok/s Gemma 3 4B, L30 injection), Gemma circuit map (L13 classifier / L15 confidence / L21 Calculator), TinyModel + MLX training stack
**Hardware:** every experiment below runs on the M3 unless its row says CUDA.

## 0. Thesis

**The model provides judgment; cells provide guarantees; the boundary lives inside the
architecture.** The programme is committed to the thesis, not to a depth: the three
integration depths — (1) cells as vocabulary, (2) behavioural-spec emission with external
routing, (3) a routed cell organ on the residual stream — share their expensive components
(fingerprint embedding space, operand-readout probes, the H1 factory, admission/co-match
machinery), so the gates below choose the depth; conviction does not.

**Non-goals.** No claim that scale can't do arithmetic — the claim is that scale doesn't
provide *guarantees* (verified, auditable, exact off-distribution, governable). No
tolerance of unverified splices: every cell result entering a context or a residual stream
is bit-exact against the reference interpreter or it is a defect. No experiment proceeds
past a failed upstream gate on optimism.

## 1. The experiments

Ordered by information density per unit cost, not by ambition.

---

### CN-0 — Operand readout (the keystone probe)

**Question.** Can the operands of an in-flight arithmetic operation be decoded from the
residual stream reliably enough to hand to a cell?

**Method.** Gemma 3 4B under LARQL instrumentation. Prompt battery of two-operand
arithmetic in varied surface forms (digit, word, mixed, embedded-in-narrative). Probe
families, cheapest first: linear; Fourier/helix-basis (respecting the known periodic
number encodings); 2-layer MLP. Sweep the L13–L21 band (classifier → Calculator). Score
*exact* u16 recovery of both operands, per layer, per probe family, on held-out prompts.

**Gate.** Some (probe, layer) achieves ≥95% exact-pair recovery on held-out surface forms.
**Kill.** No family exceeds 80% anywhere in the band → depths 2–3 are scoped out for
Gemma-class models; the programme continues at depth 1 (tokens). A negative here is a
*scoping* result, not a programme kill — that property is why CN-0 runs first.
**Cost.** Days. Existing instrumentation.

**Status: DONE — gate not met, CN-3 scoped out for Gemma-class models.** Two waves
(hyperparameter sweep, operation breadth, a narrative contrastive probe, and a null test on
the raw embedding layer) killed three alternative explanations for the held-out-family
generalization gap — under-tuning, addition being a representative operation, and narrative
lacking the operand information outright — before the scope-out was drawn. The mechanism
that survives: operand encoding forms by L0–L1 and stays flat through L26, a fast-forming
*numeral* encoding, not evidence of in-flight arithmetic computation — so there is no
computed operand state for a prosthetic to intercept in the first place. The literal kill
trigger as worded above ("no family exceeds 80% anywhere") did not strictly fire (sub/mul
clear 80% on several families) — the **gate** (≥95%) is what was never met anywhere, and
the scope-out is a reasoned call on the accumulated evidence, not a mechanical trigger. Full
account: `cell-native-architectures-findings.md`'s `## CN-0, read against the gate, after
two waves` section.

---

### CN-1 — Cell tokens with fingerprint embeddings (depth 1, the ablation that matters)

**Question.** Does a model learn to invoke cells as vocabulary — and do
behaviour-derived embeddings beat learned ones where it counts (cells never seen called)?

**Method.** Small model (TinyModel-class). Tokenizer extended with ~800 cell-identity
tokens + call-grammar delimiters. Corpus from the H1 factory (chuk-math-gym +
GSM8K-style + library-family templates; strict-improvement filter; exact oracle;
admission-style dedup on call sites). Constrained decoding over the known-hash set;
result tokens spliced by the runtime at zero decode cost. **Three arms:** (a) no-cells
baseline, (b) random-init cell embeddings, (c) **fingerprint-init** — embedding rows
projected from each cell's behavioural fingerprint over the shared probe battery.

**Gates.** (i) H2 rule: arm (b) or (c) beats the prompted `cell_solve` baseline at equal
parameter count, held-out *families* included. (ii) The novelty gate: (c) − (b) is
positive specifically on held-out-family invocation — the only mechanism by which unseen
cells have meaningful addresses. **Pre-registered band:** trivial-case shortcut rate
(model computes easy cases itself) is measured and accepted within a declared band —
shortcutting is rational, not failure.
**Kill.** (i) fails → trained invocation doesn't beat prompting at this scale; re-scope
before spending on depth 2–3 training. (ii) fails with (i) passing → fingerprints don't
transfer to embedding space; depth-2 routing still stands on F2, but the "computed
vocabulary" claim dies.
**Cost.** The programme's first real training spend. M3/MLX.

**Status: slice-0 pilot done, three iterations, concluded (toy scale, not the pre-registered
build) — the headline is architectural, not a number.** A small MLX transformer trained
from scratch (not TinyModel v11 — its tokenizer's immutable `.vocab.bin` has no
`add_tokens` API, a real detour deferred past the pilot) first ran **untied** (output
projection separate from input embeddings) and both arms silently scored 0.000 everywhere —
read correctly as "the harness is broken," not "the hypothesis is false," before any number
was trusted. **Weight tying is therefore stated as a result, not a footnote: a
fingerprint-placed embedding can only influence a prediction in a model whose output
projection shares weights with its input embeddings** — a real precondition on the whole
"the embedding is the behaviour" hypothesis, and a related constraint worth checking before
CN-4's design hardens (its result-projection needs to land somewhere the model can actually
read). Given tying, gate (i)'s core claim is supported at toy scale (fingerprint-init
0.993–1.000 vs. random-init's more variable, sometimes much worse 0.640 mean) — held
loosely, as an init-quality/convergence-speed effect, not CN-1's novel claim. **Gate (ii) —
the one that matters — got a genuine, pre-registered test in iteration 3 and came back a
clean FAIL.** Iteration 1's held-out cell had a defining input token absent from training
entirely; iteration 2 recombined only already-trained tokens into a novel *combination* and
still scored 0.000. Iteration 3 built a genuine 3x2 compositional grid (heavy, multi-partner
exposure to every category/variant token — a real basis for learning composition as a rule)
and pre-registered the bar *before* running it: fingerprint-init > 0.5 and random-init <=
0.25 on the held-out combination. Both arms scored exactly 0.000 — a clean, pre-registered
FAIL, meaning no compositional generalization exists at toy scale for any embedding strategy
to modulate. **This is not evidence against the fingerprint hypothesis** — exactly-0.000 on
both arms is a floor, not a comparison: neither arm reached a hidden state the embedding
could influence, so there was nothing for either init strategy to be judged on. The verdict
is "gate (ii) untestable at this scale," not "fingerprint-init lost." Per the fork agreed in
advance, gate (ii) now moves to the real build (a real model, the real H1 factory's
compositional-coverage corpus, a real training spend), not a fourth toy iteration — this
pilot's job (harness validated, tying precondition found, corpus requirements mapped, gate
(ii)'s first real test run and closed) is done. Full account:
`cell-native-architectures-findings.md`'s `## CN-1 slice-0` section.

---

### CN-2 — Verified decoding in LARQL (G2, product-shaped)

**Question.** Does re-deriving every scoped numeric span with a cell before commit
measurably reduce wrong-number rate — at negligible latency?

**Method.** LARQL server: span grammar over scoped contexts; each numeric span
re-derived by the matching cell; disagreement forces resample. Measure hallucinated-
arithmetic rate before/after on the GSM8K campaign battery; measure added latency.

**Gates.** Wrong-number rate reduction is significant on the pre-registered battery;
overhead ≤1% of per-token budget (85 µs at 117 tok/s — expected slack ~100×).
**Kill.** None fatal — a null result is itself informative (models' wrong numbers are
mostly *unscoped*), and the machinery is CN-1's error-correction layer regardless.
**Cost.** Engineering only. No training. First shippable artifact and the natural video.

---

### CN-3 — The prosthetic (depth 2 landing; WS-I proper)

**Question.** Read operands at CN-0's winning layer, execute the exact cell, inject the
result via the Lazarus write path — does model behaviour on arithmetic change, and is it
exact where the native lookup is known to fail?

**Method.** Gemma 3 4B in LARQL. Readout = CN-0's winner. Compute = warm cell (µs).
Write = L30 1-D injection (demonstrated: 100% P(target)). Evaluation stratified by
**lookup coverage**: operand ranges inside vs. outside the regimes where the L21
Calculator is known reliable.

**Gate.** The off-distribution-exactness signature: accuracy on out-of-coverage operands
rises to ≈ readout fidelity (the conditional guarantee made measurable: exact given the
operands). In-coverage behaviour not degraded.
**Kill.** Injection lands but behaviour doesn't move → the result re-enters too late to
influence the continuation; revisit injection layer before abandoning (the zone map's
L21–L29 retrieval-reopening band is the alternative write site).
**Depends on.** CN-0 pass.
**Cost.** Days once CN-0 lands. This is the headline experiment.

**Status: SCOPED OUT for Gemma-class models — CN-0 did not pass.** No readout feature CN-3
could actually deploy at the decision point (a single last-token tap) reaches the operand
information reliably across surface forms; the features that *did* read cleanly
(`operand_positions`, whole-sequence pooling) require already knowing where the operands sit
in text or reading after the full sequence is in, neither available when CN-3 would need to
act. Not parked pending a better probe — see CN-0's status note for the mechanism. The
programme redirects to CN-1 next.

---

### CN-4 — The routed organ on TinyModel (depth 3, minimal form)

**Question.** Can a model *train into* a frozen cell bank — a routing head + learned
read/write projections, MoE-style straight-through — and use it where it counts?

**Method.** TinyModel + MLX. Frozen bank of arithmetic cells as discrete experts at the
model's commitment layer (TinyModel's own map — Gemma layer numbers do not transfer).
Learned: routing head (reading attention output, per the 96%-of-routing-signal finding),
operand projection in, result projection out. Auxiliary usage loss against
route-around. Curriculum from cells with the exact IR-step difficulty axis: train
steps<50, evaluate steps>200 and out-of-coverage operands.

**Gates.** (i) Off-distribution arithmetic beats a matched-parameter no-organ baseline.
(ii) Organ usage above a pre-registered floor on cases where delegation is optimal
(anti-route-around). **Named measurement, not a gate: atrophy.** Compare the organ
model's *native* arithmetic (organ masked at eval) against baseline — does delegation
free capacity or install brittleness? Either answer is a result.
**Kill.** Gradient starvation or route-around resists the auxiliary-loss toolkit at
TinyModel scale → depth 3 parks; depths 1–2 carry the programme.
**Depends on.** CN-1's factory + curriculum (not CN-0 — projections are learned here).
**Cost.** The second training spend. M3.

---

### CN-5 — Circuit formation against an exact oracle (the science rider)

**Question.** Under what data distribution does a small model learn the *algorithm*
instead of the lookup table — and does the presence of an organ (CN-4) change what the
weights learn?

**Method.** Piggybacks CN-4's runs plus matched no-organ trains. Track circuit formation
with existing interp tooling against the cell as ground-truth algorithm; measure the
model-vs-oracle gap over training; vary distribution breadth via the difficulty axis.

**Gate.** None — this is instrumentation-first science; the deliverable is the formation
curves and the organ/no-organ contrast (the atrophy question, mechanistically).
**Cost.** Marginal on CN-4. Feeds the interpretability channel directly.

---

### CN-6 — Behavioural-spec emission (depth 2 routing; scale-invariance test)

**Question.** Can the model learn to emit *I/O examples* instead of hashes — resolved by
the fused router (0.859) — so invocation generalizes to cells minted after training?

**Method.** CN-1's setup, spec-emission head-to-head with hash-emission: model emits a
small example set; `search_with_examples` resolves; result splices. Test set includes
cells **added to the library after the training corpus was frozen** (synthesis or
authored) — the case hash vocabulary cannot pass by construction.

**Gate.** Resolution P@1 on post-freeze cells within 10 points of the router's own
ceiling on equipped queries — invocation has become *library-size invariant*.
**Kill.** Models can't reliably emit discriminating examples → the two-tier design
(hash tokens for the hot set, text routing for the tail) stands as the depth-2 shape.
**Depends on.** CN-1 infrastructure; F2 (passed).

---

### CN-7 — Trained composition (the ceiling)

**Question.** Does trained invocation extend to *chains* — multi-cell plans with exact
intermediates — beating the prompted planfix lane (65–95% extraction yield, 16/16
cross-check precision) at equal params?

**Method.** H1 factory extended to multi-call continuations; same strict-improvement
criterion applied per-chain. Gate mirrors H2 at chain level; the cross-check gate rides
along as the precision floor.
**Depends on.** CN-1 pass. Deliberately last: single-call reflex first.

---

## 2. Dependency graph and sequencing

```
CN-0 (probe, days) ──────────► CN-3 (prosthetic)
CN-2 (G2, engineering) ─ independent, ship first
CN-1 (tokens+fingerprint SFT) ─► CN-6 (spec emission) ─► CN-7 (composition)
                    └──────────► CN-4 (organ) ─► CN-5 (formation science)
```

Wave 1 (now, parallel): **CN-0 + CN-2** — the keystone probe and the shippable product,
neither blocks the other, both are days-scale. **CN-0 done — gate not met; CN-3 scoped out
for Gemma-class models** (see CN-0/CN-3's own status notes above). CN-2: one wave done
(60-problem verified-decoding battery), G2 build (injection/resampling) in progress.
Wave 2: **CN-1** (first training spend) — now the programme's next experiment, not
contingent on CN-3. **Slice-0 toy pilot done** (see CN-1's own status note above); the real
build (TinyModel v11 + a rebuilt tokenizer, the full H1 factory, ~800 cells, constrained
decoding) is still ahead.
Wave 3: **CN-4/5** and **CN-6**, informed by wave 2's gates. **CN-7** last.
CUDA enters only at H3-scale RL and organ training beyond TinyModel — on the spine, not
on wave 1's critical path.

## 3. Cross-cutting controls (apply to every training experiment)

- Matched-parameter baselines, always; no comparison across scales.
- Held-out **families**, not just held-out cases — the reflex must generalize, not memorize call sites.
- Shortcut-rate measured against its pre-registered band everywhere delegation is optional.
- `trapped_ops` in every reward/filter term; degenerate-call dedup at admission strictness.
- Every spliced result verified against the reference interpreter in-line; a mismatch anywhere is a filed defect and stops the run.
- Layer numbers never transfer across models (Gemma ≠ TinyModel); each model gets its own zone map before any layer-indexed claim.

## 4. Standing risks, named

**Interface fidelity** (CN-0 is the measurement; the guarantee is conditional on
readout and must always be stated that way). **Route-around/atrophy** (CN-4 measures
both; RLHF-hardens-gates predicts the risk is real). **Number serialization** at depth 1
(digit-first, arg-transcription error tracked, G2 as the net). **The bitter-lesson
objection** (answered in §0: the claim is guarantees, not capability). **Softmax routing
ceiling** at O(10³) candidates (measured; the two-tier design is the response, CN-6 is
its test).
