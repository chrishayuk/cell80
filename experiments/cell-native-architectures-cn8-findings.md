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

| Arm | B0 | B1 (5-digit) | B2 | first-error at B1 |
|---|---|---|---|---|
| B′ peel s80 | 1.000 [0.981,1] | **0.000** [0,0.019] | TBD | peel-copy: 200/200 |
| B′ peel s81 | TBD | TBD | TBD | TBD |
| A-ex′ s80 (token-parity) | TBD | TBD | TBD | — |

- **P2′ (the knockout): FAILS** — B1 0.000, not ≥ 0.50. Per prereg §7.2 (no artifact
  branch exists in this registration), this is reported as a genuine result, not excused.
  The naive claim "a scratchpad rescues length extrapolation" is **falsified at 115M under
  RoPE**. (Consistent with Zhou et al.'s "…But Not Robustly": scratchpad length-generalization
  is real but PE-fragile; this is the fragile side.)
- **But the mechanism refines the claim rather than sinking it.** Oracle-trace NLL at B1 is
  **0.751** (vs the answer-only arms' 4.6–9.9): the correct over-length trace is *unfamiliar,
  not impossible*. Free-running greedy, the model **collapses to a trained-length trace** —
  it emits exactly 4 column lines for a 5-digit problem, with locally-consistent arithmetic
  (each w = x+y+cin, carries propagate), and the first thing that breaks is rendering the
  over-length remaining prefix (`peel-copy`, column 1, the longest-prefix column — precisely
  the §3 registered residual, pre-graded genuine).
- **Same failure as CN-8, relocated — which is the real result.** CN-8's index grammar
  subsampled the 5-digit operand into a 4-slot template and computed correctly on the
  invented 4-digit problem (first-error: index). CN-8b's peel grammar collapses to a
  4-column traversal and computes locally-consistent arithmetic on a mangled prefix
  (first-error: peel-copy). Two grammars sharing no positional machinery fail at the
  identical underlying operation: **traversing/holding an operand longer than the training
  maximum.** The arithmetic *content* of the tape extrapolates in both; the *length
  addressing* of the tape does not, in either.

### 6.1 What this does to the thesis (sharpens, does not rescue)

The pre-experiment framing was "the tape is where math happens." CN-8 forces a more
precise statement: a self-generated tape carries the algorithm's **content** (single-digit
facts, carry logic — and that content demonstrably extrapolates: CN-8 logged 1,600/1,600
correct out-of-range columns; CN-8b keeps per-column arithmetic locally consistent even on
mis-lengthed traces), **but addressing the tape — knowing it has five positions, not four,
and stepping through them — is a network capability that itself does not extrapolate at
this scale and PE.** A model's own scratchpad therefore inherits the network's positional
ceiling. A *cell* does not: it reads the actual n-digit operand and returns a verified
answer at any length. So the delegation argument comes out **stronger and more specific**:
the weakness of the unverified tape is not only that a sampled carry can be wrong
(CN-2's 1.6%) — it is that the tape cannot even be *laid out* past the length the network
was trained to traverse. The verified external tape has no such limit. "Where math happens"
splits cleanly: content can live on a self-tape; **length/position cannot, and that is
exactly what cells supply.**

Headline held pending B′ s81 (seed rule §8.15) and the B2 band; A-ex′ closes P1′/P3′.
Nothing above is graded on one seed — but the s80 mechanism is unambiguous and the two
independent grammars agreeing on the failure locus is itself a two-route result.

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
