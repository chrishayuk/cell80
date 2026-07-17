# CN-8 / CN-8b — the tape experiment

**Does a scratchpad rescue length extrapolation, or does the arithmetic just live in the weights either way?**

Chris Hay | Cell-Native (CN) Programme | 2026-07-17

Registrations frozen before data: CN-8 `781ba85`, CN-8b `debd168`, CN-8c `19d8dfb`.
Corpora + audits: CN-8 `57a076f`, CN-8b `d2458ee`. Final grades: `ed60fc7`.

---

## 0. Result in one paragraph

Two supervision formats, the same 115M base (raw TinyModel v11), the same addition
problems, matched budget: **answer-only** (the answer is the only supervised token) and
**scratchpad** (a full column-addition trace). Both were trained to master 1–4 digit
addition and tested one and two digits past that range. The answer-only arm cliffs to
**0.000** exact one digit out of range — the CN-7 §8.14 result, now reproduced under
dedicated drill. The scratchpad arm, built across **two independent grammars** specifically
to remove every scaffold the failure could be blamed on, **also cliffs to 0.000 on both
seeds.** The registered knockout — "same weights, same data, opposite failure profiles" —
**did not land.** The mechanism is sharp and replicated: the model **projects the
over-length input onto its trained frame and computes correctly inside it**; what fails is
never the arithmetic, always the *length-addressing* of the tape. The corrected thesis:
a self-generated scratchpad extrapolates *nothing* out of range by itself at this scale and
positional encoding — it relocates all length-generalization into addressing, and addressing
fails. Its surviving value is **decomposition, legibility, and (for cells) verification** —
not capability. Whether the ceiling is an artifact of RoPE specifically is the pre-registered
job of CN-8c.

---

## 1. The question and why it is the right one

CN-7 §8.14 established that a 115M model taught answer-only arithmetic — clean curriculum,
oracle-signed data, unlimited in-range exposure — learns a **noise-robust in-range
interpolator with zero algorithmic content**: perfect near the training boundary, exactly
0.00 one digit past it. That was measured under one supervision format. The sharpest
follow-up the "tape" framing makes is a *contrast*: if arithmetic that lives on a
self-generated tape extrapolates while the same arithmetic compressed into weights cliffs,
then the failure profile is a property of **where the algorithm lives**, not of the model —
and the whole routing-plus-verified-tools architecture falls out as a measurement rather
than a preference.

CN-8 is that contrast, run as a controlled, audited, pre-registered training experiment with
matched budget in *both* directions and mechanistic grading of the generated traces.

## 2. Theoretical frame (the calibrated two-layer claim)

The naive version — "transformers can't do math, a tape fixes it" — is wrong in a way a
competent critic catches, and the corrected version makes the result mean *more*.

- **Expressivity does not predict the cliff.** A fixed-precision, constant-depth transformer
  sits in (roughly) **TC⁰**. But multi-digit addition *at any fixed length* is **also TC⁰**
  (carry-lookahead parallelizes it to constant depth), so the circuit for, say, 12-digit
  addition *exists* inside a network this size. Expressivity only forbids the truly
  *unbounded* case, where input length outruns any fixed depth. Taken alone it predicts
  failure *asymptotically*, not a cliff one digit past the training boundary.
- **Learnability does.** What predicts the cliff is inductive bias. Under answer-only
  supervision, in-range interpolation achieves the same loss as the algorithmic circuit, so
  SGD has no gradient toward the algorithm. The **RASP-L** line (Zhou et al. 2023/2024)
  makes this near-formal: answer-only addition is not expressible as a length-generalizing
  shortcut program, so no length generalization is the *expected* outcome — whereas the
  scratchpad decomposition *is* expressible and *should* generalize. **CN-8 tests exactly
  the "should".**
- **Chain-of-thought is the tape theorem, and it is already delegation.** Merrill &
  Sabharwal and Feng et al. show CoT steps buy serial time — polynomially many steps lift
  the class from TC⁰ toward P. "The loop happens in tokens, not layers" is literally the
  theorem. So a scratchpad *is* delegation to a tape; the only question cell80 cares about
  is whether that tape can be **wrong** (a sampled carry) — CN-2 measured 1.6% wrong-number
  on a strong battery; a cell is the same move with a verified tape at microsecond cost.
- **The honest carve-out.** Grokking shows small transformers *can* learn genuine algorithms
  — but only for **finite, closed** domains (modular arithmetic mod p, where the whole input
  space is structurally coverable). Open-ended integer arithmetic is not that. So the
  calibrated statement going in: transformers can learn finite-domain algorithms and can
  memorize in-range surfaces of infinite-domain ones; the tape is *supposed* to convert the
  second into the first by making the algorithm expressible. CN-8 asks whether SGD at 115M
  actually takes that path.

## 3. Method

**Task and bands (frozen).** Multi-digit addition, surface `{A} + {B} =`. Training operands
1–4 digits, mixed lengths and orders. Evaluation on **n=200 frozen seed-90 sets, identical
across every arm**: **B0** = 4×4 in-range (deduped against all training problems), **B1** =
5×5 (one digit past), **B2** = 6×6 (two past). Bands stop at 6 digits because v11's hard
`max_seq` is 256 and a 7-digit trace overruns it — the context window can never be blamed
for the cliff.

**Two grammars, run as two experiments.**
- **CN-8, index-hint grammar**: digit labels written once (`a3#3 a2#8 …`), then each column
  fetches digits by label and the answer accumulates by prepending. One production scales
  with length — the first index label (`a4` at 5 digits is novel) — and the pre-registration
  **named it in advance** as the sole length-scaling element and its potential artifact.
- **CN-8b, peel grammar**: no labels, no indices, no length-reading anywhere. Each column
  line carries the *remaining* operand prefixes (`3298 3742 5+1+0=6 w6 c0 a#6 |`), so every
  step is a local rewrite of the previous line; termination is **state-based** (`- -` when
  both prefixes exhaust). A **grammar audit** (a registration gate) mechanically verified
  that every production required at 5–6 digits already occurs in training — the CN-8 artifact
  class is retired by proof, not hope.

**Arms (matched both ways).**

| Arm | Supervision | Corpus | Seeds |
|---|---|---|---|
| B / B′ (scratchpad) | full trace | 51,031 problems | 80, 81 |
| A-ex (example-matched) | answer-only | the *identical* 51,031 problems | 80 |
| A-tok (token-matched) | answer-only | 6M tokens (≈6.8× the problems) | 80, 81 |

A-ex and A-tok close both matching objections: A-ex holds the *problems* fixed, A-tok holds
the *tokens* fixed. CN-8b additionally repaired A-ex to **token parity** (the same problems
repeated to 6M tokens, `A-ex′`), because CN-8's single-epoch A-ex under-trained.

**Recipe.** Raw v11 in the original SP id space (no new tokens), full-model update, full loss,
no replay, lr 1e-4, bs 16, ~5–6M tokens/arm. TinyStories NLL recorded-not-gated. Audits
(two-route independent-schoolbook re-derivation of every trace, add_sat cell-signing of every
in-u16 instance, range scan, problem-identity check, and CN-8b's grammar audit) all passed
before any GPU time.

## 4. CN-8 results — index-hint grammar

| Arm | B0 | B1 | B2 | first-error (B1/B2) |
|---|---|---|---|---|
| B scratchpad s80 | **1.000** [.981,1] | 0.000 | 0.000 | index 200/200 both bands |
| B scratchpad s81 | **1.000** [.981,1] | 0.000 | 0.000 | index 200/200 both bands |
| A-tok s80 | 0.970 | 0.000 | 0.000 | — (well-formed wrong answers) |
| A-tok s81 | 0.965 | 0.000 | 0.000 | — |
| A-ex s80 (0.88M tok) | **0.425** — gate fail | 0.000 | 0.000 | — |
| R0 raw v11 (answer) | 0.000 | 0.000 | 0.000 | never saw format |
| R0 raw v11 (trace) | 0.000 | 0.000 | 0.000 | never saw format |

**Verdict: VOID as registered** — A-ex failed the 0.90 sanity gate (0.425); the permitted
escalation was **declined** (its knobs — tokens, lr — cannot touch a frozen-grammar failure,
and the spine prescribes format iteration under a new registration). Graded-as-measured:
**P1 held** (answer-only cliffs, both seeds); **P2 not achieved via the pre-named artifact**
(100% of scratchpad OOD failures fire first at the `index` line — the novel `a4` label);
**P6 held** (NLL dissociation, below).

**Two calibration facts that survived the void.**
1. *The cliff is trained-in miscalibration, not unreached territory.* Untrained v11 assigns
   gold answers a flat ~6.0 nats at every band. Answer-only training pulls in-range NLL to
   ~0.01 and pushes B2 NLL to **8–10 nats — above the untrained floor.** Training doesn't
   fail to reach past the boundary; it actively makes the correct answer *more* surprising
   there. (The maskless cousin of CN-7's 11-nat squeeze.)
2. *Sample-efficiency dissociation by format.* On the identical 51,031 problems, one epoch of
   scratchpad → 1.000 in-range; one epoch of answer-only → 0.425. (Confound: scratchpad rows
   carry ~6.8× more tokens; CN-8b's A-ex′ closes it.)

## 5. CN-8b results — peel grammar, and the final grades

The peel grammar removed every scaffold CN-8's artifact could hide behind. **The knockout
still failed.**

| Arm | B0 | B1 | B2 | first-error B1 | col-cond B0/B1/B2 | oracle-NLL B0/B1/B2 |
|---|---|---|---|---|---|---|
| B′ peel s80 | 1.000 | **0.000** | **0.000** | peel-copy 200/200 | 1.00 / 0.69 / 0.36 | 0.00 / 0.75 / 1.96 |
| B′ peel s81 | 1.000 | **0.000** | **0.000** | peel-copy 200/200 | 1.00 / 0.74 / 0.43 | 0.00 / 0.80 / 2.14 |
| A-ex′ s80 (token-parity) | **0.935** | 0.000 | 0.000 | — | — | 0.05 / 5.46 / 8.45 |
| A-tok s80 (inherited) | 0.970 | 0.000 | 0.000 | — | — | 0.01 / 6.06 / 9.91 |
| A-tok s81 (inherited) | 0.965 | 0.000 | 0.000 | — | — | 0.01 / 4.64 / 8.61 |

col-cond = per-column conditional correctness (fetch+table+carry+acc all holding against the
model's *own* prior state). oracle-NLL = teacher-forced NLL on the gold trace (B′) or gold
answer (A arms).

**Final grades against the frozen thresholds (prereg §5):**

| Pred | Threshold | Result | Verdict |
|---|---|---|---|
| Sanity | B0 ≥ 0.90 all arms | 1.00 / 1.00 / 0.935 | **PASS** — incl. the repaired control (token parity fixed CN-8's 0.425) |
| P1′ | A-ex′ B1 ≤ 0.05 | 0.000 | **HELD** — answer-only cliffs even at parity |
| **P2′** | **B′ B1 ≥ 0.50, both seeds** | **0.000 / 0.000** | **FAILS — no knockout** |
| P3′ | A-ex′ B1 ≤ 0.05 with B0 ≥ 0.90 | 0.000 with 0.935 | **HELD** — budget rescues in-range, not OOD, in **both** directions |
| P4′ | B′ graceful in **exact** (−ln exact ~linear) | 1.00→0.00→0.00 | **FALSIFIED** — B′ is *also* a step function in exact; graceful decline survives only in col-cond and NLL |
| P5′ | per-column error on B1∪B2 ∈ (0, 5%] | in-range 0%, OOD 26–64% | **FALSIFIED / mis-specified** — no regime where the tape is the "1.6%-noisy" object predicted |
| P6′ | A shows cliff-NLL; B′ oracle-NLL at B1 < 1.5 | A 4.6–6.1; B′ 0.75/0.80 | **HELD** — calibration dissociation, ~6–8× |

**Net:** two clean holds (P3′, P6′), one clean fail that is the headline (P2′), two falsified
optimism-predictions (P4′, P5′). P4′ and P5′ were both written under the "content
extrapolates" assumption that this run forced me to withdraw (§6); their failure is
*internally consistent* with the withdrawal, and the registration's value is precisely that
it recorded that optimism in advance and let it be killed rather than quietly omitted.

## 6. What this does to the thesis — corrected under pushback

**The first draft of this section over-claimed, and the correction is load-bearing.** It said
the tape's arithmetic *content extrapolates*, citing 1,600/1,600 correct out-of-range columns
(CN-8) and locally-consistent arithmetic on collapsed traces (CN-8b). **Withdrawn.** A column
is digit + digit + carry — an atom drawn from the same finite 10×10×2 table whether the
problem is 4-digit or 40-digit. It has *no length axis* and is therefore **never** out of
distribution at any operand length. "Correct out-of-range columns" is not evidence of
extrapolation; the columns were in-distribution by construction. By that same construction,
**the entire OOD burden of length generalization concentrates in the addressing/traversal
productions** — index-labeling (CN-8) and prefix-copy (CN-8b) — and those are exactly what
failed, 400/400 then 200/200.

The honest, one-sided statement: **a scratchpad reduces length generalization to
length-addressing, and length-addressing fails.** The tape extrapolates *nothing* OOD by
itself at this scale. Its genuine contributions are two, and neither is capability:
1. **Decomposition** — it turns an opaque `0.000` into a *localized* production failure
   (the collapse is at column 1's over-length prefix, not "somewhere in the arithmetic").
2. **Legibility** — the specimen dumps (§8) exist *because* the tape exists; the mechanism is
   readable straight off the trace.

A caveat the per-column number forces, stated so it isn't over-read: the collapse is **not** a
clean shorter-but-valid computation. Local self-consistency is 1.00 in-range but degrades to
0.69–0.74 at B1 and 0.36–0.43 at B2. So addressing fails *first* and unambiguously (peel-copy
200/200); downstream the trace degrades progressively (cascade from the broken prefix, plus
some genuine off-frame arithmetic breakage the first-error metric can't isolate). The clean
claim is **"addressing initiates the failure," not "arithmetic is untouched."**

This makes the cells argument **cleaner, not louder.** The cell is not supplementing a tape
that works OOD (it doesn't) — it **replaces the entire traversal.** And the load-bearing
reason to prefer a cell is **verification, not the tape's positional failure**: a cell returns
a *checked* answer at any length; a self-generated tape returns a *sampled* one whether or not
its addressing extrapolates. Verification never depended on the tape failing and survives
either outcome of CN-8c.

### 6.1 Scope, the rival reading, and the falsifier

**PE scope, welded on.** "Length-addressing does not extrapolate" is, on present evidence, a
fact about **RoPE (θ=10⁴) at 115M**, nothing more. The literature contains the counterexample:
Zhou et al. ("…But Not Robustly") and the abacus-embedding line (McLeish et al. 2024) show
trained scratchpads *do* length-generalize under position encodings built for it. The
unconditional phrasing is one embedding choice from falsification and is **not** cited as
established. The discriminator is registered as **CN-8c**
(`cell-native-architectures-cn8c-preregistration.md`): identical peel grammar, one PE
intervention (randomized offsets, else abacus), both branches priced in advance — Branch A
(it extrapolates → the ceiling was a PE artifact, the "cells supply what the tape can't lay
out" prop falls, cells retreat to verification) listed **first**, because it costs the
programme a talking point.

**The reading this section is not allowed to bury.** Rival interpretation, plainly: *scratchpads
don't help length generalization at all at this scale, and the "content vs addressing" split
is post-hoc comfort — a way to keep a tape thesis alive after the tape failed the one test
built to kill it.* §6 concedes most of it: the tape achieved nothing OOD. What survives is
decomposition + legibility (diagnostic, not capability) and verification (never a tape claim).
If CN-8c collapses too, that is the honest headline and the programme keeps only those — which
is enough for cells but is *not* "the tape is where math happens."

**Meta-note, recorded same-day.** This is the second scope-retreat of 2026-07-17 (the diversity
law → marshalling; the tape claim → content-not-addressing). Each is individually honest and
mechanistically supported, and each moves toward the programme's prior commitment. The defense
demanded of the diversity reformulation applies identically: the sharpened tape claim is not
cited as established until CN-8c runs, and the rival reading is on the record, not managed.

## 7. Two honesty corrections this run made me apply

Both were caught by instruments/pushback registered before the belief they killed — the CN-7
method, held here:
1. **"Content extrapolates" (§6)** — withdrawn as near-vacuous once it was pointed out that a
   column has no length axis. The corrected framing is stronger for cells.
2. **"Locally-consistent arithmetic" (§6 caveat)** — refuted by the per-column number
   (1.00 → 0.69 → 0.36); softened to "addressing initiates the failure."

The two falsified predictions (P4′, P5′) were downstream of correction #1 and died with it.
This is what the pre-registration is *for*.

## 8. The frame-projection phenomenon (the day's most-replicated finding)

Across three formats with **zero shared machinery**, one behaviour: given an input larger than
the training frame, the model **projects it onto the trained frame and computes correctly
inside it** — a confident, well-formed answer to a *smaller* problem than asked.

| Specimen | Format | Frame | Over-frame input | Behaviour |
|---|---|---|---|---|
| Broker prompt 4 (CN-6/7) | tool-call marshalling | 3-digit args | `12345` | emitted `safe_div(123, 123)` — truncated to frame width |
| CN-8 index grammar | scratchpad | 4-digit | `32985` | wrote a 4-slot index line dropping the `9`; computed the 4-digit sum correctly |
| CN-8b peel grammar | scratchpad | 4-digit | `32985 + 37421` | emitted a 4-column trace, collapsing the 5-digit problem |

Three independent surfaces, one mechanism — the most replicated single finding the programme
owns, and plausibly the **mechanistic face of L8** (frame/skeleton finite trained cardinality).
**Open question for the read-side (CN-10):** is the over-frame operand *represented upstream*
and discarded at emission, or *never encoded* past frame width? The oracle-NLL of 0.75 (the
correct long trace is unfamiliar, not impossible) hints the information is present and the
collapse is decode/addressing-time — but that is a hint, not a measurement. The clean probe:
is the 5th digit linearly recoverable from the residual stream at the emission position on a
collapsed generation? If yes, frame-projection is an addressing failure over intact
information — which turns §6's "addressing, not content" split into a representational claim,
and is the strongest argument yet that the build-side and read-side halves of the programme
need each other.

## 9. Chronology (protected — the sequence is part of the finding)

1. CN-8 prereg frozen and committed before any corpus row (`781ba85`).
2. Corpus + audits clean; five arms trained; eval chain interrupted repeatedly by machine
   contention; partial results committed (`a751f86`) before each resume; ultimately run
   detached to survive the interruptions.
3. B s80: 1.000 → 0.000/0.000, all-`index` first-errors — the pre-named artifact fired. A-tok
   both seeds cliffed textbook; A-ex failed the gate at 0.425. **CN-8 VOID**; escalation
   declined with reasons.
4. CN-8b registered (`debd168`) per the spine; its grammar audit **failed once** on a
   context-flipped SP segmentation (bare-`w` piece — the CN-7 §8.1 family), fixed pre-training
   by a canonical per-word encoder (amendment A1), then passed (`d2458ee`). Arms trained.
5. B′ both seeds: 1.000 → 0.000/0.000, all-`peel-copy` — the knockout failed with no artifact
   escape hatch. A-ex′ passed the gate at token parity (0.935) and still cliffed OOD.
6. Under pushback, "content extrapolates" and "locally-consistent" both withdrawn (§7); CN-8c
   falsifier registered with both branches priced (`19d8dfb`); final grades committed
   (`ed60fc7`).
7. Numbering: a parallel session minted a different "CN-8"; resolved by renumbering *its draft*
   to CN-9 (`0e2c7a0`) — frozen registrations keep their number, drafts renumber.

## 10. Status and next

- **CN-8 / CN-8b: complete and graded.** Knockout failed on both seeds and both grammars; the
  corrected thesis and its rival reading are on the record.
- **CN-8c: registered, not yet run.** The one open follow-up — does a length-generalizing PE
  rescue the peel arm (Branch A: ceiling was an artifact) or not (Branch B: two-PE result)?
  Verification stands either way. Needs its §6 pre-run engineering gate and a free machine.
- **CN-10 (read-side):** the frame-projection upstream-representation probe is the highest-value
  cross-programme experiment this run surfaced.

### Artifacts

Prereg: `cell-native-architectures-cn8-preregistration.md`, `…-cn8b-…`, `…-cn8c-…`. Code:
`cn8_corpus.py`, `cn8_train.py`, `cn8_eval.py`, `cn8_dump.py`, `cn8b_corpus.py`,
`cn8b_eval.py`, `cn8_master.sh`. Data/results: `cn8_corpus_stats.json`,
`cn8b_corpus_stats.json`, `cn8_eval_*_{trace,answer}.json`, `cn8b_eval_*_{trace,answer}.json`,
`cn8_eval_problems.json`, `cn8_dump_b_s80_B1.txt`. Checkpoints are not committed (repo
convention); every number above has a committed JSON and a two-route audit behind it.
