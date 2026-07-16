# CN-10: The Bridge Experiment — LARQL instrumentation on v11 (does the external canonical form match the internal one?)

**DRAFT v0.0 — NOT PINNED. Thresholds and predictions marked TO-PIN await Chris's
review AND the CN-8 readout; nothing here is registered until this header changes
and the pin is committed. Numbering checked against the tree 2026-07-17 (CN-8 =
frozen tape experiment 781ba85, CN-9 = abstraction-threshold DRAFT); no collision.**

Chris Hay | CN Programme | July 2026

---

## 1. Purpose

Two research tracks have run in parallel for eighteen months. The read-side (LARQL,
the GPT-OSS series) claims the transformer is a query system: attention as a query
compiler normalising surface forms into canonical structural templates, refined
layer by layer — syntactic early, semantic mid-depth, retrieval late. The
write-side (cell80, the broker, CN-7/CN-8) builds that machine on purpose:
normaliser, canonical intermediate, stored-procedure cells, content-addressed
index, planner. The frame already has one frozen falsifiable bet in flight —
CN-8's P1/P2 are the relations-vs-functions test (answer-only arms can only
materialise rows; only the rewriting arm can extrapolate).

CN-10 is where the tracks meet on one machine. It puts the read-side's
instrumentation on v11 — the only model in the programme where we own the
pretraining corpus, the tokenizer, the midtrain arms, and the probes end-to-end —
and asks three questions:

- **Q1 (existence)**: do parse/retrieve layer phases exist at all at 115M / 20
  layers, and where is the boundary?
- **Q2 (intervention)**: does canonical-intermediate training (CN-8 arm B) *move
  or sharpen* that boundary relative to budget-matched answer-only training
  (arm A-tok) — i.e. does supervising the query language reorganise the compiler?
- **Q3 (identity)**: does the externally trained canonical form *align* with the
  internal mid-depth representation — is the scratchpad the model's own internal
  intermediate surfaced as supervisable text, or a separate object bolted on?

A yes on Q3 with all selectivity controls passing is the fusing result: the
canonical intermediate is not a training trick but the internal query language
made external. A clean no is equally reportable: the write-side stands as
engineering, and the identity claim dies.

## 2. What already exists (no new training arms)

CN-10 consumes CN-8's checkpoints; it trains nothing of its own unless the §4
control gap is adopted.

| object | role in CN-10 | provenance |
|---|---|---|
| raw v11 (115M, 20L, SP vocab 71,261) | base — Q1 substrate | 15_v11_model |
| CN-8 arm B (scratchpad), seeds 80/81 | canonical-intermediate arm — Q2/Q3 | CN-8 §3.4 |
| CN-8 arm A-tok, seeds 80/81 | budget-matched no-canonical control | CN-8 §3.4 |
| CN-8 arm A-ex, seed 80 | problem-matched control (secondary) | CN-8 §3.4 |
| CN-8 eval bands B0/B1/B2 (seed-90 frozen) | probe battery input | CN-8 §3.1 |

The conversationally-named "DIV-3" arm is retired: arm B *is* the
canonical-intermediate training, and A-tok *is* the token-matched control, both
already frozen and audited.

**Control gap, decided before pinning (TO-PIN)**: A-tok controls token budget but
differs from B in supervised *content*, not just structure. The strict
structure-null is a **shuffled-trace arm** (arm B's corpora with trace lines
permuted within each example — same tokens, same vocabulary, destroyed canonical
order; CN-1's shuffled-control precedent). Decide at pin time whether A-tok
suffices for Q2 or the shuffled-trace arm must be trained (cheap: cn8 recipe
verbatim, one seed). Default if undecided: train it — the drift lesson (EX-2)
says never read a movement without the null that moves everything.

## 3. Instrumentation feasibility gate (blocks the pin, like CN-8 §5 blocked training)

LARQL the *server* loads Gemma-class GGUF; v11 is a TinyModel in the SP id space.
The *method* is what transfers. Gate: before predictions are pinned, ONE of

- (a) v11 exported to a LARQL-loadable format, tokenizer round-trip audited
  (the CN-1 lesson: v11 decodes only under the 15_v11_model SP tokenizer —
  any export carries a tokenizer-identity audit row, id→piece bijection on the
  full eval battery), or
- (b) LARQL's per-layer readout (logit-lens projection, span probes,
  activation extraction at every block) reimplemented in the v11 PyTorch stack,
  validated by reproducing one known LARQL reading on a model both stacks load.

is demonstrated on a 10-prompt smoke slice. If (a): `larql-server` invoked
directly (never `larql serve` — drops `--infer-timeout-secs`), `LARQL_SPIN_POOL=0`.
Route (b) is the expected winner (v11's stack already exists; no quantisation
confound); (a) is kept open because the server's constrained decoding is wanted
for CN-2-style follow-ups.

## 4. Measurement

### 4.1 Q1 — phase existence (base v11)

Per-layer battery over the frozen CN-8 eval prompts plus a syntactic-contrast
set (TO-PIN: construction; candidate = CN-9's frame-type bank):

- **Logit-lens curve**: KL(layer-ℓ projection ‖ final) and answer-token rank
  per layer, per band.
- **Probe ladder**: linear probes per layer for (i) surface/syntactic features
  (frame type, operand position), (ii) semantic/template features (operation
  identity, operand *values* binned), (iii) answer features (result digits).
  Probes trained on band-B0 activations, tested held-out.

**Boundary criterion — pinned before anyone looks (TO-PIN)**: candidate — the
layer at which probe family (ii) first exceeds family (i) held-out accuracy,
and the layer at which answer-token logit-lens rank first enters the top-K.
"No boundary" verdict criteria pinned with equal precision (flat curves, no
crossover): **an absent boundary at 115M/24M-pretrain-tokens is a scale-and-data
null, not evidence against the read-side theory** — this is written down now so
it cannot be argued either direction later.

### 4.2 Q2 — boundary under intervention

The §4.1 battery, identical prompts and probe recipes, run on: base, A-tok
(s80/s81), B (s80/s81), [shuffled-trace if adopted]. Deliverables: boundary
location and sharpness (probe-accuracy crossover width, TO-PIN definition) per
checkpoint per seed. The claim "canonical training reorganised the compiler"
requires the movement in B to exceed the movement in A-tok (and the shuffled
control if trained) beyond seed spread — two seeds is a spread indicator, not a
CI; wording of the claim graded accordingly (multi-seed gate discipline, CN-7
§8.16 precedent).

### 4.3 Q3 — identity of external and internal intermediates

On arm B checkpoints, prompts formatted **answer-only** (no trace in the input):
train a decoder probe from layer-ℓ activations to the *canonical trace tokens*
the grammar would emit for that problem (the trace is computable from the
problem — supervision needs no generation). Alignment = held-out trace-token
decodability from mid-depth.

**Selectivity controls, all three mandatory — a probe that decodes anything
decodes this**:

1. **Depth selectivity**: mid-depth decodability exceeds first-2 and last-2
   layers (margin TO-PIN).
2. **Model selectivity**: the same probe recipe on A-tok activations fails
   (threshold TO-PIN) — if answer-only training yields the trace equally well,
   the trace is task-inferable, not trained-in, and Q3 is unanswerable by this
   instrument (verdict class, not failure).
3. **Pairing null**: probes trained on shuffled (activation, trace) pairings
   score at chance.

Q3 verdicts: PASS all three → the identity claim, stated at 115M scope.
FAIL (2) in the direction "A-tok decodes it too" → instrument-limited,
redesign registered before rerun. FAIL (1) or (3) → no alignment; the canonical
intermediate is external-only; reported as the headline null.

## 5. Pre-registered forks

- Q1 no-boundary → Q2 reframes as "does canonical training *create* one" —
  arguably the stronger result; criterion for "created" = the same TO-PIN
  boundary test passing on B but not base/A-tok.
- Q2 moves-under-everything (A-tok moves it as much as B) → drift, not
  reorganisation; reported as such, Q3 still runs (Q3 does not presuppose Q2).
- CN-8 P2 *fails* (arm B does not extrapolate) → CN-10 still runs but the
  purpose line is rewritten before the pin: Q3 then asks why a trained
  intermediate that doesn't help is or isn't internally represented — the
  predictions TO-PIN below are contingent on CN-8's verdict, which is why this
  draft cannot pin them yet.

## 6. Predictions — TO-PIN by Chris after the CN-8 readout, before any probe is trained

(Explicitly blank. Candidate registrable quantities: boundary layer range at
115M; sign and magnitude of the B-vs-A-tok boundary shift; Q3 pass/fail as the
headline bet; whether Q1 phase count is 2 or 3 at this depth.)

## 7. Sequencing and hygiene

CN-8 audit gate is OPEN as of 57a076f; CN-10 does nothing until CN-8's arms are
trained and its verdict is graded (its checkpoints are CN-10's substrate, and §6
predictions are contingent on its outcome). Tokenizer identity audited on every
activation-extraction path (the CN-1 wrong-mapping incident is the standing
reason). No new CN numbers minted without a tree grep (CN-8/CN-9 collision
precedent). All probe code and extracted-activation manifests committed before
any accuracy is computed.

## 8. Provenance

Read-side: LARQL (~/chris-source/larql), Gemma circuit map (L13/L15/L21),
GPT-OSS series. Write-side: CN-7 R1 graded record (§8.13–8.17), CN-8
registration (781ba85 lineage, arms §3.4, bands §3.1). Frame stakes already
in flight: CN-8 P1/P2 as the relations-vs-functions bet. Instruments inherited:
multi-seed gate discipline, manifest + overwrite guards, disclosed-amendment
protocol.
