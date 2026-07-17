# CN-8 / CN-8b findings — the tape experiment

Chris Hay | CN Programme | 2026-07-17 (living document; sections land as their numbers do)

Registrations: CN-8 frozen at 781ba85 (before any corpus row); CN-8b frozen at debd168
(format iteration per CN-8's spine). Corpora: 57a076f (CN-8, all audits PASS), d2458ee
(CN-8b, incl. grammar audit). Partial readout committed at a751f86.

---

## 1. The question

CN-7 §8.14 showed a 115M model taught answer-only arithmetic learns a noise-robust
in-range interpolator with zero algorithmic content — 0.00 exact one digit past its
training range. CN-8 asks the sharpest follow-up the tape framing makes: **is the failure
profile a property of where the algorithm lives?** Train the same base (raw v11, SP id
space) on the same addition problems under two supervision formats — answer-only (the
surface lives in the weights) vs scratchpad (the algorithm lives on the tape) — matched in
both directions (examples and tokens), and read the failure profiles at the training-range
boundary (train 1–4 digit operands; B1 = 5-digit; B2 = 6-digit; n=200 frozen seed-90 sets,
identical across every arm).

## 2. CN-8 headline table (index-hint grammar)

| Arm | B0 (in-range) | B1 (5-digit) | B2 (6-digit) | first-error at B1/B2 |
|---|---|---|---|---|
| B scratchpad s80 | **1.000** [0.981,1] | 0.000 | 0.000 | index: 200/200, both bands |
| B scratchpad s81 | **1.000** [0.981,1] | 0.000 | 0.000 | index: 200/200, both bands (oracle-trace NLL 0.95/2.31 vs s80's 0.97/2.45 — full replication) |
| A-tok s80 (6M tok, 347k problems) | 0.970 | 0.000 | 0.000 | — (well-formed wrong answers) |
| A-tok s81 | 0.965 | 0.000 | 0.000 | — |
| A-ex s80 (identical 51k problems, 0.88M tok) | **0.425 — SANITY GATE FAIL** | 0.000 | 0.000 | — |
| R0 raw v11, answer format | 0.000 | 0.000 | 0.000 | format/truncation (never saw the format; gold-answer NLL ~6 flat — no cliff, no competence) |
| R0 raw v11, trace format | 0.000 | 0.000 | 0.000 | index (oracle-trace NLL flat 9.5–10.0 all bands — the format is pure midtrain knowledge; the trained arms' 0.00 in-range NLL and 0.95–2.45 OOD sit far below this floor) |

Teacher-forced NLL (secondary, the P6 dissociation): the A arms show the C3 cliff
signature on gold answers — 0.011 → 6.06 → 9.91 nats (s80), 0.013 → 4.64 → 8.61 (s81),
0.252 → 5.64 → 8.03 (A-ex). The scratchpad arm's NLL on gold **oracle traces** at the same
bands: 0.000 → 0.970 → 2.453 (s81: 0.950 → 2.307). The answer-only model finds correct
out-of-range answers ~e⁻⁶–e⁻¹⁰ improbable; the scratchpad model finds correct
out-of-range *procedures* merely unfamiliar.

The raw floor sharpens this: untrained v11 assigns gold answers a flat ~6.0 nats at
every band. Answer-only training pulls in-range NLL to ~0.01 and pushes B2 NLL to
8.0–9.9 — **above the untrained floor**. The cliff is not a region training failed to
reach; training actively mis-calibrates beyond the boundary (the §8.9/CN-7 11-nat
squeeze family, reproduced here without any mask).

## 3. CN-8 graded verdicts (as frozen in §7 of the prereg)

- **Sanity gate: FAILED by A-ex (0.425 < 0.90) → CN-8 is VOID as registered.** The
  permitted escalation round was **declined**, with the reasoning recorded at readout: the
  escalation knobs (≤2× tokens, lr) cannot alter the P2 outcome because P2's failure is in
  the frozen grammar, and the spine already prescribes format iteration under a new
  registration for exactly this branch. All other verdicts below are reported as measured,
  flagged by the void.
- **P1 (cliff replicates under full answer supervision): HELD** on both A-tok seeds —
  B0 0.97/0.965, B1/B2 exactly 0.000, NLL signature present. §8.14's cliff is now
  demonstrated under dedicated-drill training, not just S2-story training.
- **P2 (the knockout): NOT ACHIEVED — A-rule artifact branch, mechanically exact.**
  100% of scratchpad OOD failures (400/400 traces across B1/B2) fail first at the index
  line — the production the prereg named in advance as the grammar's one length-scaling
  element (the novel `a4` label). Everything downstream extrapolated perfectly: **1,600/
  1,600 OOD columns correct** in fetch, table, carry-propagation, and acc-copy, graded
  against the model's own state. Under the frozen A-rule: instrument artifact, thesis
  untested at this length, no knockout language.
- **P3 (budget does not rescue): HELD in the token direction** (A-tok, 6.8× examples,
  same tokens: 0.000 at B1 on both seeds); the example direction is void with the gate
  (A-ex's B1 0.000 is uninterpretable under B0 0.425).
- **P4 (opposite shapes): NOT GRADEABLE** as registered — B's exact-match is a step
  function too (via the index artifact), so the shape contrast never got measured. The
  *production-level* shape is the finding instead (see §4).
- **P5 (tape price 0.2–5% per column): MISSED LOW, flattering direction** — B0 measured
  0/800 columns (Wilson upper ≈0.5%); the learned tape at trained lengths is cleaner than
  the CN-2 precedent pinned.
- **P6 (NLL dissociation): HELD** (see §2 numbers).

## 4. The specimen: right answer to the wrong question

The B1 trace dump (cn8_dump_b_s80_B1.txt) shows the mechanism verbatim: given
`32985 + 37421`, the model writes an index line for a **4-digit** problem — subsampling
the 5-digit operand, typically dropping a middle digit (`3,2,8,5`, discarding the 9) —
then executes its column algorithm *flawlessly on the tape it invented*: every digit
fact, carry, and copy correct for `3285 + 3741`. The arithmetic program generalizes;
what fails is exactly one production: **reading how long the input is.** The model
projects every input onto its trained frame and computes correctly inside it.

Two readings, both licensed by the data: (i) as the A-rule anticipated, absolute
position labels (`a4` never emitted in training) are scaffolding that pins the frame —
an instrument artifact; (ii) more generally, *length-reading is itself a piece of the
algorithm*, and it stayed in the weights (trained frame) rather than moving to the tape.
CN-8b discriminates: its grammar has no labels, no indices, and no length-reading —
termination is state-based — so if the frame-pinning survives the format fix, it is not
scaffolding.

## 5. Findings independent of the void

1. **Sample-efficiency dissociation by supervision format**: on the *identical* 51,031
   problems, one epoch of scratchpad supervision → 1.000 in-range; one epoch of
   answer-only supervision → 0.425. The trace's intermediate structure is worth roughly
   the difference between mastery and coin-flip at equal exposure. (Confound noted:
   scratchpad rows also carry ~6.8× more tokens per problem; CN-8b's A-ex′ closes this
   by repetition to token parity.)
2. **The algorithm/length split** (§4): length-independent productions extrapolate at
   100%; the single length-dependent production fails at 100%. Length generalization is
   not one capability but (at least) two.
3. **Fluency cost asymmetry** (recorded-not-gated): 6M answer-only tokens destroyed
   TinyStories NLL (1.69 → 11.5–12.8); 6M scratchpad tokens barely moved it (→ 1.82).
   Same budget, same base, ~7× more optimizer steps in the short-row regime.
4. **Two instrument catches before any GPU time was spent** (the audit culture paying
   rent): CN-8's independent-schoolbook two-route audit ran clean, and CN-8b's grammar
   audit caught a context-flipped SP segmentation (bare-`w` piece) that would have
   silently contaminated the TF-NLL secondary — the CN-7 §8.1 family, caught at a gate
   this time (fixed pre-training by the canonical per-word encoder, amendment A1).

## 6. CN-8b readout (peel grammar)

**The registered knockout FAILED, and the failure is the finding.** CN-8b removed every
length-scaling scaffold CN-8 could be blamed on (no labels, no indices, state-based
termination; the grammar audit proved every required production at 5–6 digits already
occurs in training) — and the peel arm *still* scores 0.000 one digit out of range.

| Arm | B0 | B1 (5-digit) | B2 (6-digit) | first-error at B1 | col-cond B0/B1/B2 | oracle NLL B0/B1/B2 |
|---|---|---|---|---|---|---|
| B′ peel s80 | 1.000 [.981,1] | **0.000** [0,.019] | **0.000** | peel-copy 200/200 | 1.00 / 0.69 / 0.36 | 0.00 / 0.75 / 1.96 |
| B′ peel s81 | 1.000 [.981,1] | **0.000** [0,.019] | **0.000** | peel-copy 200/200 | 1.00 / 0.74 / 0.43 | 0.00 / 0.80 / 2.14 |
| A-ex′ s80 (token-parity, repaired) | **0.935** [.892,.962] | 0.000 | 0.000 | — | — | 0.05 / 5.46 / 8.45 |
| A-tok s80 (inherited) | 0.970 | 0.000 | 0.000 | — | — | 0.01 / 6.06 / 9.91 |
| A-tok s81 (inherited) | 0.965 | 0.000 | 0.000 | — | — | 0.01 / 4.64 / 8.61 |

(oracle NLL = teacher-forced NLL on the *gold* trace for B′, gold *answer* for A arms.)

### 6.3 Final grades — all arms in (frozen thresholds, prereg §5)

| Pred | Threshold | Result | Verdict |
|---|---|---|---|
| Sanity | B0 ≥ 0.90 all arms | peel 1.00/1.00, A-ex′ 0.935 | **PASS** — incl. the repaired control (CN-8's A-ex was 0.425; token-parity fixed it) |
| P1′ | A-ex′ B1 ≤ 0.05 | 0.000 | **HELD** — answer-only cliffs even at parity |
| **P2′** | **B′ B1 ≥ 0.50, both seeds, CI-disjoint from A** | **0.000 / 0.000** | **FAILS — no knockout, both seeds** |
| P3′ | A-ex′ B1 ≤ 0.05 with B0 ≥ 0.90 | 0.000 with 0.935 | **HELD** — the clean version CN-8 couldn't deliver: budget rescues in-range, *not* OOD, in **both** directions (6× examples = A-tok; token-parity repetition = A-ex′) |
| P4′ | A step / B′ graceful in **exact** (−ln exact ~linear) | B′ exact 1.00→0.00→0.00 | **FAILS as stated** — B′ is *also* a step function in exact-match; graceful decline survives only in col-cond (1.0→0.7→0.4) and NLL, not exact. This is exactly what the withdrawn "content extrapolates" would have predicted, and its failure confirms the §6.1 correction. |
| P5′ | per-column error on B1∪B2 ∈ (0, 5%] | in-range 0%; OOD 26–64% | **FAILS / mis-specified** — the prediction assumed a working-but-noisy OOD tape. Reality: in-range the tape is *perfect* (0/800, below the range), OOD it *collapses* (26–64%, far above). There is no regime where the tape is the "1.6%-noisy" object P5′ imagined. |
| P6′ | A shows C3 cliff; B′ oracle NLL at B1 < 1.5 | A 4.6–6.1; B′ 0.75/0.80 | **HELD** — the calibration dissociation is real: the peel tape finds correct OOD *traces* ~6–8× less surprising than the answer-only arms find correct OOD *answers*, even though both free-run to 0.000. |

**Net: two clean holds (P3′, P6′), one clean fail that is the headline (P2′), two falsified
optimism-predictions (P4′, P5′) — both written under the "content extrapolates" assumption
this run forced me to withdraw.** The registration's value is exactly that it recorded those
two optimistic predictions in advance and let them be killed; a doc written after the fact
would have quietly omitted them. Verdict stands as §6.1/§6.2: the knockout failed, the tape
extrapolates nothing OOD by itself at 115M/RoPE, its surviving value is decomposition +
legibility + verification, and whether the ceiling is a PE artifact is CN-8c's job.

- **P2′ (the knockout): FAILS** — B1 0.000, not ≥ 0.50. Per prereg §7.2 (no artifact
  branch exists in this registration), this is reported as a genuine result, not excused.
  The naive claim "a scratchpad rescues length extrapolation" is **falsified at 115M under
  RoPE**. (Consistent with Zhou et al.'s "…But Not Robustly": scratchpad length-generalization
  is real but PE-fragile; this is the fragile side.)
- **But the mechanism refines the claim rather than sinking it.** Oracle-trace NLL at B1 is
  **0.751** (vs the answer-only arms' 4.6–9.9): the correct over-length trace is *unfamiliar,
  not impossible*. Free-running greedy, the model **collapses toward a trained-length trace**
  and the **first** thing that breaks is always rendering the over-length remaining prefix
  (`peel-copy`, column 1, the longest-prefix column — 200/200 first-errors, precisely the §3
  registered residual, pre-graded genuine). Caveat, from the per-column number and stated so
  it isn't over-read: the collapse is **not** a clean shorter-but-valid computation — local
  self-consistency (fetch+table+carry+acc all holding against the model's own prior state)
  is 1.00 in-range but **degrades to 0.69 at B1 and 0.36 at B2**. So addressing fails *first*
  and unambiguously; downstream, the trace also degrades progressively (cascade from the
  broken prefix, plus some genuine off-frame arithmetic breakage that the first-error metric
  alone can't separate). The clean claim is "addressing initiates the failure," not
  "arithmetic is untouched."
- **Same failure as CN-8, relocated — which is the real result.** CN-8's index grammar
  subsampled the 5-digit operand into a 4-slot template and computed correctly on the
  invented 4-digit problem (first-error: index). CN-8b's peel grammar collapses toward a
  4-column traversal, first-error at the mangled over-length prefix (peel-copy), with
  local consistency degrading downstream (col-cond 0.69/0.36). Two grammars sharing no
  positional machinery fail at the
  identical underlying operation: **traversing/holding an operand longer than the training
  maximum.** The length-addressing of the tape does not extrapolate in either grammar; the
  arithmetic atoms never had to (see §6.1 — a column has no length axis).

### 6.1 What this does to the thesis (corrected — the first draft of this section over-claimed)

An earlier draft of this section said the tape's arithmetic *content extrapolates* (citing
1,600/1,600 correct out-of-range columns). **That claim is withdrawn as near-vacuous, and
the correction is load-bearing.** A column operation is digit + digit + carry — an atom
with no length axis. It is drawn from the same finite table (10×10×2) whether the problem
is 4-digit or 40-digit, so it is *never* out of distribution at any operand length. "Correct
out-of-range columns" is therefore not evidence of extrapolation; the columns were
in-distribution by construction. By the same construction, **the entire OOD burden of
length generalization concentrates in the addressing/traversal productions** — index-labeling
(CN-8) and prefix-copy (CN-8b) — and those are exactly what failed, 400/400 then 200/200.

The honest statement of what a scratchpad does here is one-sided: **it reduces length
generalization to length-addressing, and length-addressing fails.** The tape extrapolates
*nothing* OOD by itself at this scale. Its genuine contributions are two, and neither is
capability:
1. **Decomposition** — it turns an opaque `0.000` into a *localized* production failure
   (the collapse happens at column 1's over-length prefix, not "somewhere in the arithmetic").
2. **Legibility** — the specimen dumps in §8 exist *because* the tape exists; the failure
   mechanism is readable off the trace.

This makes the cells argument **cleaner, not louder**: the cell is not supplementing a tape
that works OOD (it doesn't) — it **replaces the entire traversal**. And critically, the
load-bearing reason to prefer a cell is **verification, not the tape's positional failure**.
Verification never depended on the tape failing and survives either outcome of CN-8c
(§6.2/the CN-8c registration): a cell returns a *checked* answer at any length; a
self-generated tape returns a sampled one whether or not the addressing extrapolates. The
positional-ceiling observation is a **secondary, scoped, and falsifiable** claim — see §6.2.

### 6.2 Scope, the alternative reading, and the falsifier (registered before it hardens)

**PE scope, welded on.** "Length-addressing is a network capability that does not
extrapolate" is, on the present evidence, a fact about **RoPE (θ=10⁴) at 115M**, nothing
more. The literature already contains the counterexample: Zhou et al. and the abacus-embedding
line show trained scratchpads *do* length-generalize under position encodings designed for
it, sometimes dramatically. So the unconditional phrasing is one v12 embedding choice away
from falsification and must not be cited as established. The discriminator is registered as
**CN-8c** (`cell-native-architectures-cn8c-preregistration.md`): the identical peel grammar,
one PE intervention (randomized/abacus-style positions), both branches priced in advance —
if the peel arm then extrapolates, the ceiling was a PE artifact and the "cells supply what
the tape cannot lay out" prop falls (cells retreat to verification, their real ground); if
it still collapses, "positional ceiling at this scale" becomes a two-PE result and earns
the unconditional phrasing it currently only borrows.

**The reading this section is NOT allowed to bury.** The rival interpretation, stated
plainly: *scratchpads don't help with length generalization at all at this scale, and the
"content vs addressing" split is post-hoc comfort — a way to keep a tape thesis alive after
the tape failed the one test built to kill it.* §6.1's correction concedes most of this:
the tape achieved nothing OOD. What survives the rival reading is only decomposition +
legibility (diagnostic value, not capability) and verification (which was never a tape
claim). If CN-8c collapses too, that is the honest headline, and the programme keeps only
those two — which is enough for cells but is *not* "the tape is where math happens."

**Meta-note, recorded while it is still today.** This is the second scope-retreat of the
day (the diversity law → marshalling this afternoon; the tape claim → content-not-addressing
tonight). Each is individually honest and mechanistically supported, and each moves toward
the programme's prior commitment. The defense demanded of the diversity reformulation
applies identically here: **the sharpened tape claim is not cited as established until CN-8c
runs**, and the rival reading above is on the record, not managed.

Headline (P2′ pass/fail) held pending B′ s81 (seed rule §8.15) and the B2 band; A-ex′
closes P1′/P3′. The s80 mechanism is unambiguous, and two independent grammars agreeing on
the failure locus is itself a two-route result — but no grade is stamped on one seed.

## 8. The frame-projection phenomenon (the day's most-replicated finding)

Across three formats with **zero shared machinery**, one behaviour recurs: given an input
larger than the training frame, the model **projects the input onto the trained frame and
computes correctly inside it** — a confident, well-formed answer to a smaller problem than
the one it was asked.

| Specimen | Format | Trained frame | Over-frame input | What the model did |
|---|---|---|---|---|
| Broker prompt 4 (CN-6/CN-7) | tool-call marshalling | 3-digit args | `12345` | emitted `safe_div(123, 123)` — truncated the operand to frame width |
| CN-8 index grammar | answer-on-scratchpad | 4-digit operands | `32985` | wrote a 4-slot index line, dropping the `9`; computed the 4-digit sum correctly |
| CN-8b peel grammar | state-rewrite scratchpad | 4-digit operands | `32985 + 37421` | emitted a 4-column trace (locally-consistent arithmetic), collapsing the 5-digit problem |

Three independent surfaces, one mechanism. This is the most replicated single finding the
programme owns, and it is plausibly **the mechanistic face of L8** (the frame/skeleton
having finite trained cardinality). It is promoted here from a bullet buried in three
findings docs; it should graduate to its own artifact.

**The open mechanistic question, and why it needs the read-side.** Is the full over-frame
operand *represented upstream* and discarded at emission, or *never encoded* past frame
width? The CN-8b oracle-trace NLL of **0.751** (the correct 5-digit trace is unfamiliar,
not impossible) hints the information is present and the collapse is a decode/addressing-time
event, not an encoding-time truncation — but that is a hint, not a measurement. This is
exactly a CN-10 probe target (LARQL/read-side on v11): probe whether the 5th digit is
linearly recoverable from the residual stream at the emission position on a collapsed
generation. If it is, frame-projection is an addressing failure over intact information —
which tightens §6.1's "addressing, not content" split into a representational claim and is
the strongest argument yet that the build-side and read-side halves of the programme need
each other.

## 7. Chronology (protected, per the R1 writeup directive)

1. CN-8 prereg frozen and committed before any corpus row (781ba85).
2. Corpus + audits clean; five arms trained; eval chain interrupted twice mid-run
   (machine contention; partial results committed at a751f86 before resuming).
3. B s80: B0 1.000 → B1/B2 0.000, all-index first-errors; the A-rule branch fired as
   frozen. A-tok both seeds: textbook cliff. A-ex: sanity-gate fail 0.425.
4. Escalation declined with reasons; CN-8b registered (debd168) per the spine, its
   grammar audit failed once (SP segmentation), was fixed pre-training (A1), passed
   (d2458ee); arms launched.
5. A parallel-session numbering collision (a different draft also minted "CN-8") was
   resolved by renumbering the draft to CN-9 (0e2c7a0): frozen registrations keep their
   numbers; drafts renumber.
