# CN-8b: The Tape Experiment, format iteration — peel grammar

**Pre-registration v1.0 — predictions pinned before any CN-8b corpus row exists**

Chris Hay | CN Programme | 2026-07-17

---

## 1. Purpose and lineage

CN-8 (frozen 781ba85; partial readout a751f86) resolved its knockout
prediction P2 into the pre-registered artifact branch: 100% of scratchpad
out-of-range failures occurred at the index line — the one production the
prereg named in advance as length-scaling (the novel `a4` label). The
mechanistic grading showed everything else extrapolated perfectly: 1,600
out-of-range columns with zero errors in fetch, table, carry propagation,
and accumulator copy, executed on a mislabeled tape (the model subsamples
a 5-digit operand into its 4-slot template — the dump shows it dropping a
middle digit and adding the number it invented, flawlessly).

CN-8's frozen spine prescribes: format iteration under a NEW
registration. This is it. Same thesis, same bands, same eval sets, same
thresholds; the grammar is replaced by one with **no labels, no column
indices, and no length-reading anywhere**, and the artifact class that
fired is retired by a mechanical audit rather than by hope.

CN-8's sanity gate also failed on the example-matched arm (A-ex B0
0.425): one epoch over 51k problems at 0.88M tokens under-trains
answer-only in-range competence (itself a sample-efficiency finding: the
identical problems under trace supervision reached 1.000). CN-8b repairs
the control's definition: example-matched **at token parity via
repetition**.

## 2. Pinned baselines (inherited — nothing about these arms changes)

| # | Quantity | Value | Provenance |
|---|---|---|---|
| D1 | A-tok s80 B0/B1/B2 exact | 0.970 / 0.000 / 0.000 | cn8_eval_atok_s80_answer.json |
| D2 | A-tok s81 B0/B1/B2 exact | 0.965 / 0.000 / 0.000 | cn8_eval_atok_s81_answer.json |
| D3 | A-tok answer-NLL signature | 0.01 → 4.6–6.1 → 8.0–9.9 | same |
| D4 | CN-8 B (index grammar) s80 | B0 1.000; B1/B2 0.000, first-error 100% index; col-conditional 800/800 at every band | cn8_eval_b_s80_trace.json |
| D5 | Training corpora | identical problem multiset (51,031) and identical 6M-token A-tok corpora | cn8_corpus_stats.json |
| D6 | Eval problem sets | seed-90 frozen B0/B1/B2, n=200 | cn8_eval_problems.json |

The A-tok arms are CN-8b's token-matched controls *as measured* — their
training and evals are untouched by the format change. The raw-v11 floors
and CN-8 B s81, in flight at freeze time, attach to both registrations
when they land (they are floor/replicate rows, not gates; their values
are unknown at this commit).

## 3. The peel grammar (frozen exactly)

State-rewrite scratchpad: each column line carries the *remaining* operand
prefixes, so every step is a local operation on the previous line —
shorten-by-one copy, single-digit table lookup, carry, prepend. Termination
is state-based (both prefixes exhausted → `- -`), never counted. All
tokens carry loss; operands 1–4 digits in training, same distribution as
CN-8 (D5's exact problem list).

```
{A} + {B} = | {Pa'} {Pb'} {x}+{y}+{cin}={s} w{w} c{cout} a#{acc} | … | o{c} [a#{acc}] | ans {R} .
```

where Pa′/Pb′ are the prefixes *after* peeling (rendered `-` when empty),
x/y are the peeled digits (0 when the operand is exhausted), and acc grows
by prepending. Worked examples:

```
384 + 529 = | 38 52 4+9+0=13 w3 c1 a#3 | 3 5 8+2+1=11 w1 c1 a#13 | - - 3+5+1=9 w9 c0 a#913 | o0 | ans 913 .
997 + 8 = | 99 - 7+8+0=15 w5 c1 a#5 | 9 - 9+0+1=10 w0 c1 a#05 | - - 9+0+1=10 w0 c1 a#005 | o1 a#1005 | ans 1005 .
```

Properties: the `- -` state occurs in the final column of every training
row (fully trained termination cue); `-` also appears mid-trace for every
unequal-length pair; the final answer cannot carry a leading zero (top
digit of the longer operand is nonzero, so a top-column write of 0 forces
o1). Worst-case token costs: 4×4 = 136, 5×5 = 172, 6×6 = 212 — B2 fits
max_seq 256 with headroom. Bands identical to CN-8: train (d_A,d_B) ∈
{1..4}², B0 = 4×4 deduped, B1 = 5×5, B2 = 6×6, same seed-90 sets (D6).

**The named residual, and its honest status.** Nothing at eval length
requires a novel token or a novel local production — the grammar audit
(§4) verifies this mechanically. What IS new at B1/B2 is *composition
length*: column lines render prefixes of 4–5 characters where training
rendered at most 3, and the loop runs 5–6 iterations where training ran
at most 4. Unlike CN-8's index line, the peel-copy is **load-bearing** —
it is the algorithm's state-carry, not scaffolding. Therefore, frozen
now: if B′ fails P2 with first-error mass concentrated in
{peel-copy at the longest-prefix columns, truncation}, that is reported
as the residual firing — but it is graded as a **genuine (narrow)
failure of the tape program's state-carry to extrapolate**, not as an
instrument artifact. CN-8b deliberately has no artifact escape hatch:
the grammar was chosen so that every remaining failure mode is
informative about the thesis.

## 4. Corpus, arms, audits (gate before training)

| Arm | Supervision | Corpus | Seeds |
|---|---|---|---|
| **B′** (peel) | full trace | CN-8's identical 51,031 problems, 1 epoch (≈4.5–5M tokens, measured at build) | 80, 81 |
| **A-ex′** (example-matched at token parity) | answer-only | the identical 51,031 problems repeated/reshuffled to 6.0M tokens (≈6.8 epochs) | 80 |
| A-tok (inherited) | answer-only | as measured (D1/D2) | 80, 81 |

B′ trains on ≤6M tokens while A-tok trained on 6M — the asymmetry favors
the *control*, disclosed as conservative. Training recipe identical to
CN-8 (§3.5 there): raw v11 SP space, full-model, full loss, no replay,
lr 1e-4, bs 16. TinyStories NLL recorded-not-gated.

Audits (all must pass or nothing trains):
1. **Two-route**: independent string/table schoolbook re-renders every
   trace and answer row (no code shared with the generator); exact text
   match.
2. **Cell signature**: every instance ≤65535 re-executed through add_sat.
3. **Range**: no operand >4 digits in any training text (independent scan).
4. **Problem identity**: B′ and A-ex′ (deduplicated) carry CN-8's exact
   problem multiset.
5. **Grammar audit (new)**: over the oracle traces of all 600 eval
   problems: (a) every SP piece required appears in the B′ training
   corpus; (b) every digit-triple (x,y,cin), every w/c/o form, and every
   `-` boundary configuration appears in training; (c) the only items on
   the novelty list are prefix-render lengths 4–5 and loop iterations
   5–6 — the §3 residual — and nothing else. The audit OUTPUT includes
   the residual list; an empty "nothing else novel" line is the gate.

## 5. Evaluation and grading (frozen)

Protocol identical to CN-8 §6 (greedy, n=200, same sets, parse after
`ans`; truncation = no ` .` by position 256). Trace grading against the
model's own prior state, classes: {peel-copy, fetch, table, carry-prop,
acc-copy, overflow, readout, loop-count, format, truncation}, where:
- peel-copy: each rendered prefix equals the model's own previous prefix
  minus its final character (`-` for empty);
- fetch: x/y equal the final characters of the model's own previous
  prefixes (0 against `-`);
- loop-count: number of column lines equals max(len A, len B).

| # | Prediction | Threshold |
|---|---|---|
| P1′ | A-ex′ cliffs like A-tok | B1 exact ≤ 0.05 (A-tok already graded ✓✓ at D1/D2) |
| P2′ | **The knockout**: B′ extrapolates | B1 exact ≥ 0.50, both seeds, Wilson CIs disjoint from every A arm (strong: ≥ 0.80) |
| P3′ | Token parity does not rescue the surface | A-ex′ B1 ≤ 0.05 with B0 ≥ 0.90 |
| P4′ | Opposite failure shapes | A arms step functions; B′ graceful, −ln(exact) roughly linear in column count B0→B2 |
| P5′ | The tape has a price | per-column conditional error on B1∪B2 columns ∈ (0, 5%]; B0 rate reported alongside |
| P6′ | NLL dissociation | A arms show D3's signature; B′ oracle-trace NLL at B1 stays below 1.5 (CN-8 B measured 0.97 with a *worse* grammar) |

Sanity gate: B′ (both seeds) and A-ex′ at B0 ≥ 0.90 — same escalation
clause as CN-8 (one round, ≤2×/lr, identical across arms, disclosed).
Partial rule: B′ B1 ∈ (0.05, 0.50) → "tape advantage, not knockout".
Seed rule: both B′ seeds must clear every graded threshold independently.

## 6. Decision spine

```
audits (incl. grammar audit) ── fail ──► fix pipeline, do not train
        │ clean
train B′ s80/s81, A-ex′ s80 ──► sanity gate ── fail ──► one escalation ── fail ──► VOID
        │ pass
band evals ──► P1′–P6′ graded as frozen
        │
P2′ pass ──► the knockout stands: same weights, same data, opposite failure profiles
P2′ partial ──► directional support only
P2′ fail ──► genuine hit to the tape claim (incl. the §3 residual reading) — ships as such;
             NO artifact branch exists in this registration
```

## 7. Commitments

1. Freeze-before-data: this document commits before any CN-8b row exists.
2. The §3 residual's grading (genuine-not-artifact) cannot be revisited
   after results exist.
3. Inherited baselines are cited as measured; no re-running of A-tok.
4. Negative results ship with the same care; two routes to every number.
5. Anything beyond addition (multiplication, subtraction) remains
   deferred to future registrations.
