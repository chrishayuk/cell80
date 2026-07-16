# CN-8: The Tape Experiment — scratchpad vs answer-only, same weights, same budget

**Pre-registration v1.0 — predictions pinned before any corpus generation or training**

Chris Hay | CN Programme | 2026-07-17

---

## 1. Purpose

CN-7 §8.14 established that a 115M model given every advantage — clean
curriculum, oracle-signed data, unlimited in-range exposure — learns
beyond-tier arithmetic as a noise-robust interpolator that scores exactly
0.00 one digit past its training range. The cliff was measured under one
supervision format: answer-only (the injected result is the only answer
token the model ever produces or predicts).

CN-8 tests the sharpest prediction that framing makes: **the failure
profile is a property of where the algorithm lives, not of the model.**
A model taught column addition *on a scratchpad* should extrapolate past
its training range (the algorithm lives on the tape — the serial loop runs
in tokens, not layers); the same model taught answer-only should cliff at
the range boundary (the surface lives in the weights). Same base
checkpoint, same parameter count, same data budget — opposite failure
profiles.

Theoretical frame, stated honestly:

- Fixed-precision constant-depth transformers sit in (roughly) TC⁰, and
  fixed-length multi-digit addition is *also* in TC⁰ (carry-lookahead) —
  so expressivity alone does not predict a cliff one digit past training.
  What predicts it is learnability: answer-only supervision gives SGD no
  gradient toward the algorithmic circuit when in-range interpolation
  achieves the same loss (the RASP-L line: answer-only addition is not
  expressible as a length-generalizing shortcut program; the scratchpad
  decomposition is).
- Chain-of-thought escapes the depth limit by using output as a serial
  tape (poly-step CoT lifts the class). CN-8's scratchpad arm is that
  theorem run as a training experiment.
- The tape is *unverified*: every carry is a sampled token. CN-8 measures
  the per-step error rate of the learned tape (P5) — the residue that is
  the whole argument for verified cells (CN-2 measured 1.6% wrong-number
  on a strong battery; cells are the same delegation done at microsecond
  cost with signatures).

**Prior art, so the novelty claim is calibrated:** scratchpad-vs-direct
contrasts exist in pieces — Nye et al. 2021 (scratchpad), Lee et al. 2023
(chain-of-thought data for small-transformer arithmetic), Zhou et al.
2023/2024 (RASP-L prediction of exactly this dichotomy; length
generalization exists but is fragile). What has not been done, and what
CN-8 adds: *matched budget in both directions* (token-matched AND
example-matched answer-only controls), an oracle-signed corpus with a
two-route audit, pre-registered thresholds frozen before training, a
mechanistic per-production grading of the generated traces, and the
failure-*profile* instrument from CN-7 §8.11/§8.14. The claim is
"audit-grade version of a suspected result," not "never tested."

---

## 2. Pinned baselines

| # | Quantity | Value | Provenance |
|---|---|---|---|
| C1 | Answer-only beyond-range exact (S2 story format, add/sub/round, one digit past) | 0.00 on every cell | CN-7 §8.14 off-dist probe |
| C2 | Same, in-range | add 0.85 / round 1.00 / sub 0.75 | CN-7 §8.14 |
| C3 | Cliff NLL signature | 0.1 → 3.7–5.4 at B1, worse at B2 | CN-7 §8.14 |
| C4 | Unverified-tape residue precedent | 1.6% wrong-number rate | CN-2 |
| C5 | v11 pretrain | 24M tokens TinyStories, NLL 0.66 (SP mapping) | CN-7 §8.1/§8.3 |
| C6 | Midtrain throughput | 15M tokens ≈ 12,450 s on MPS (full model, bs 16) | cn7_midtrain_nomask_run.log |

---

## 3. Definitions (all frozen)

### 3.1 Task and bands

Multi-digit integer addition, canonical surface `{A} + {B} =`. Operands
are drawn with no leading zeros; d-digit operand ∈ [10^(d-1), 10^d), with
d=1 meaning [1, 9].

- **Training range**: (d_A, d_B) uniform over {1..4}² — mixed lengths,
  both orders.
- **B0 (in-range eval)**: both operands 4-digit, fresh problems deduped
  against every arm's training set. 4×4 is the boundary-adjacent in-range
  point, and the only in-range cell where dedup is guaranteed possible.
- **B1 (one past)**: both operands 5-digit.
- **B2 (two past)**: both operands 6-digit.

No operand of more than 4 digits appears anywhere in any training corpus
(audited, §5). Bands stop at 6 digits because the model's hard max_seq is
256 and the worst-case 6×6 trace is 236 tokens (7×7 is 281 — impossible);
the format cannot be blamed for a truncation the context window forces.

### 3.2 The trace grammar (arm B), frozen exactly

Content-addressed index-hint format: digit labels are written once, then
each column line fetches digits by label (induction-head machinery, not
positional offset arithmetic), the answer accumulates by prepending (no
terminal reversal operation anywhere). Zero-padding of the shorter
operand happens in the label line. All spans carry loss in both arms.

```
{A} + {B} = | i a{L-1}#{digit} … a0#{digit} b{L-1}#{digit} … b0#{digit} |
c0 {x}+{y}+0={s} w{w} c{cout} a#{acc} |
c1 {x}+{y}+{cin}={s} w{w} c{cout} a#{acc} |
…
o{carry_out} [a#{acc} if carry_out=1] |
ans {R} .
```

Worked example, `3847 + 5296 =`:

```
3847 + 5296 = | i a3#3 a2#8 a1#4 a0#7 b3#5 b2#2 b1#9 b0#6 | c0 7+6+0=13 w3 c1 a#3 | c1 4+9+1=14 w4 c1 a#43 | c2 8+2+1=11 w1 c1 a#143 | c3 3+5+1=9 w9 c0 a#9143 | o0 | ans 9143 .
```

Properties: `acc` grows by prepending the written digit (`a#43` after
`a#3`); the overflow line is always present (`o0 |` or `o1 a#1… |`); the
final answer is a copy of the last `acc` and can never carry a leading
zero (top digit of the longer operand is nonzero, so a top-column write
of 0 forces a carry-out). The SP tokenizer splits all numbers to single
digits and renders every marker (`#`, `|`, `▁i`, `▁a`, `▁c`, `▁w`, `▁o`,
`▁ans`) as clean pieces (verified before freezing).

**The length-scaling production, named in advance:** the first label of
the index line is `a{L-1}`. Training operands reach 4 digits, so labels
`a0…a3`/`b0…b3` are seen; at B1 (5-digit) the model must emit the novel
label `a4`, at B2 `a5`. Emitting the successor of the largest seen index
is the one production that scales with length; it is graded separately
(§6) and the artifact rule (§7, A-rule) turns on it.

### 3.3 Answer-only format (arms A-tok, A-ex), frozen

```
{A} + {B} = {R} .
```

Identical problem prefix to arm B. The terminal ` .` supervises
termination in both arms (the CN-7 §8.9 P-a1 run-on confound,
pre-empted).

### 3.4 Arms

| Arm | Supervision | Corpus | Seeds |
|---|---|---|---|
| **B** (scratchpad) | full trace | N_B problems, ≈6M tokens | 80, 81 |
| **A-ex** (example-matched) | answer-only | the *identical* N_B problems | 80 |
| **A-tok** (token-matched) | answer-only | A-ex's problems plus fresh ones to ≈6M tokens (~6× the examples) | 80, 81 |

A-ex and B share the exact problem list: the only difference is what the
supervision writes between `=` and `.`. A-tok controls the other
direction (more problems, same tokens). Together they close both matching
objections: P3 predicts *neither* rescues the cliff.

### 3.5 Substrate and training recipe

Raw TinyModel v11 in the original SP id space (vocab 71,261 — no new
tokens; the grammar uses only existing pieces; no resize). Full-model
update, no loss mask (every token carries loss — there is no tier
boundary in this experiment), no replay. lr 1e-4, bs 16, AdamW wd 0.01,
warmup 200, linear decay floor 0.05, grad-clip 1.0 — cn7_train.py's
recipe with the species machinery removed. Token budget 6M per arm
(A-ex: its natural ≈1/6 of that, by construction).

Note on FFN policy: CN-7 §8.14 adopts FFN-frozen as *ladder* policy for
broker builds. CN-8 is a measurement, not a broker rung, and its
deliverable is in-weights vs on-tape arithmetic — which §8.10 showed
requires FFN plasticity. Both arms train full-model; the comparison is
internal and symmetric.

Fluency is out of scope: no replay, so TinyStories NLL will degrade in
both arms. It is *recorded* pre/post on a 500-row held-out slice
(recorded-not-gated) solely to pre-empt the "you destroyed the model"
reading; no claim rides on it.

---

## 4. What is NOT claimed

- Not that the scratchpad equals cells: the tape is unverified, and P5
  exists to measure exactly the error rate that makes it untrustworthy.
- Not "models can learn math with CoT": one task family (addition), one
  tokenizer (single-digit), one PE (RoPE, θ=10⁴), ≤6-digit operands, a
  115M substrate. The claim is about *where the algorithm lives*, not
  about capability in general.
- Not novelty of the bare contrast (see §1 prior art); the contribution
  is the controlled, audited, pre-registered version with mechanistic
  grading.

---

## 5. Corpus audits (gate — no training until clean)

1. **Two-route audit**: an independent string-based schoolbook adder
   (separate code path from the Python bignum `+` the generator uses)
   re-derives every column line, every acc state, the overflow line, and
   the final answer of every scratchpad row, and the answer of every
   answer-only row. Target: 100%. Any mismatch is a pipeline bug.
2. **Cell signature**: every instance with result ≤ 65535 is re-executed
   through the `add_sat` cell oracle (u16 ceiling); 100% of in-u16
   instances must sign. Keeps the corpus in the cell-signed lineage where
   the library's range allows.
3. **Range audit**: no operand > 4 digits anywhere in any corpus, by an
   independent scan of the emitted text (not the generator's bookkeeping).
4. **A-ex/B problem-identity audit**: the (A, B) multisets of arm B and
   arm A-ex corpora are identical.

---

## 6. Evaluation protocol (frozen)

Free-running greedy decode (temperature 0), prompt `{A} + {B} =`, stop at
the first ` .` or at position 256. n=200 per band. The three band problem
sets are drawn once with seed 90 and are *identical across all arms* and
the R0 floor (raw v11, both formats — expected ≈0, format never seen).

- **Headline metric**: exact match of the parsed final answer (arm B:
  digits after `ans`; arms A: digits after `=`). A truncated generation
  (no ` .` reached) grades as wrong and is also counted separately as
  truncation.
- **Secondary (teacher-forced)**: answer-span NLL per band for A arms
  (cn7_offdist-style); oracle-trace NLL per band for arm B (does the
  model assign low NLL to correct out-of-range traces even where it
  cannot free-run them — reported as its own line).
- **Carry-depth strata**: exact by carry count within each band,
  per-stratum counts alongside rates.
- **Mechanistic trace grading (arm B)**, per generated trace, each
  production graded against the model's *own* prior state so credit
  assignment is per-step, not end-to-end:
  - index line: labels and digits exactly correct for the prompt;
  - fetch: each column's x, y match the model's own index line;
  - table: s = x+y+cin (single-digit addition fact);
  - carry propagation: cin matches the model's own previous cout;
  - acc copy: acc = w prepended to the model's own previous acc;
  - overflow line and readout: ans equals final acc;
  - loop count: exactly L column lines are produced;
  - first-error class per failed trace ∈ {index, fetch, table,
    carry-prop, acc-copy, overflow, readout, truncation, loop-count,
    format} (format = structurally unparseable segment).
- **P5 statistic**: per-column conditional correctness on B0 (a column is
  correct if fetch+table+carry+acc all hold given the model's own prior
  state); the per-step error rate of the learned tape.

---

## 7. Pre-registered predictions and thresholds

**Sanity gate (all arms, both seeds): B0 exact ≥ 0.90.** If any arm
fails, the experiment is VOID as registered — permitted remedy: one
escalation round (≤2× token budget and/or lr retune), applied to every
arm identically, thresholds untouched, escalation disclosed. A second
failure ships as a void report.

| # | Prediction | Threshold | Grades against |
|---|---|---|---|
| P1 | The cliff replicates under full answer supervision: A-tok and A-ex score ≈0 one digit past range | B1 exact ≤ 0.05, every A arm, every seed | C1 (0.00), C3 |
| P2 | **The knockout**: the scratchpad arm extrapolates | B B1 exact ≥ 0.50, both seeds, Wilson CIs disjoint from every A arm's B1 (strong form: ≥ 0.80) | — |
| P3 | Budget does not rescue the surface: neither 6× examples (A-tok) nor matched examples (A-ex) moves B1 | both A arms within each other's B1 CIs and ≤ 0.05 | C1 |
| P4 | Opposite failure *shapes*: A arms are step functions (B0 high → B1/B2 ≈ 0); B declines gracefully, consistent with per-column error compounding (−ln(exact) roughly linear in column count from B0→B2) | directional, graded on the plotted profile | C1/C2 shape |
| P5 | The tape has a nonzero price: per-column conditional error rate on B0 ∈ [0.2%, 5%] | measured on B0 traces | C4 (1.6%) |
| P6 | NLL mirrors exact (secondary): A arms show the C3 signature at B1; B's oracle-trace NLL stays near its B0 level at B1 | directional | C3 |

**Partial-outcome rule**: B B1 ∈ (0.05, 0.50) → "tape advantage, not
knockout"; report as directional support, no knockout language.

**A-rule (the artifact interpretation, frozen now because it will be
tempting to invent later)**: if arm B fails P2 and the first-error mass
is concentrated (> 50% of failed traces) in the length-scaling
productions {index, truncation}, the reading is *instrument artifact*
(format/PE — the known index-successor and context-length risks), the
thesis is UNTESTED at that length, and the result is explicitly not
evidence against the tape claim. The `loop-count` and `format` classes
do NOT count toward the artifact numerator even though they may be
length-driven — the conservative direction, biased against excusing a
failure.
If the first-error mass is in the length-independent productions {table,
carry-prop, fetch, acc-copy}, the failure is genuine and the tape claim
takes the hit. Symmetrically: if B *passes* P2, the same grading is
reported so the pass can be attributed to productions, not vibes.

**Seed rule (§8.15 institutionalised)**: no headline claim on one seed.
B and A-tok run seeds 80 and 81; every graded threshold must hold on both
seeds independently. A-ex (single seed) supports only P3, jointly with
A-tok.

---

## 8. Decision spine

```
corpus + audits ── any audit fails ──► fix pipeline, do not train
        │ clean
train 5 arms (B×2, A-tok×2, A-ex×1, sequential on MPS)
        │
B0 sanity gate ── fail ──► one escalation round ── fail ──► VOID report
        │ pass
band evals + trace grading ──► P1–P6 graded as frozen
        │
P2 pass ──► knockout: opposite failure profiles, same weights, same data
P2 partial ──► directional support, no knockout language
P2 fail + A-rule artifact ──► inconclusive at this length; format iteration is a NEW registration
P2 fail + genuine ──► the tape claim takes the hit; ships anyway
```

---

## 9. Budget

| Item | Cost |
|---|---|
| Corpus + audits | CPU, minutes |
| B s80/s81, A-tok s80/s81 | 4 × 6M tokens ≈ 4 × ~85 min on MPS (C6 rate) |
| A-ex s80 | ~1.2M tokens ≈ ~20 min |
| Band evals + trace grading | ~200 × 3 bands × 6 checkpoints, batched greedy ≤ 256 tokens; ~2–3 h |
| R0 floor | one eval pass |

Checkpoints are not committed (repo convention); scripts, corpora stats,
audit outputs, eval JSONs, and the findings are.

---

## 10. Commitments

1. This document commits before any corpus row exists; thresholds do not
   move after the §5 audits pass.
2. Negative and void results ship with the same care as the knockout.
3. Two routes to every number: the §5 audits, per-stratum counts beside
   rates, per-seed numbers beside aggregates.
4. The A-rule interpretation of a scratchpad failure is fixed *now*; no
   post-hoc artifact story beyond it will be claimed.
5. Multiplication (the flashier demo) is explicitly deferred to a future
   registration; nothing in this document licenses running it under these
   thresholds.
