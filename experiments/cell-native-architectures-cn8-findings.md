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
| R0 raw v11, trace format | TBD | TBD | TBD | TBD |

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

TBD — chain launched 2026-07-17 (B′ s80/s81 on the identical problem multiset, ~5.0M
tokens each; A-ex′ s80 at 6.0M-token parity via repetition; A-tok arms inherited as
measured per prereg §2). The registered stakes, restated so hindsight can't soften them:
CN-8b has **no artifact branch**. If B′ extrapolates (B1 ≥ 0.50 both seeds, CIs disjoint
from every A arm), the knockout stands. If B′ fails — including via the named peel-copy
residual — the tape claim takes a genuine hit: the state-carry of the written algorithm
failed to extrapolate one digit.

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
