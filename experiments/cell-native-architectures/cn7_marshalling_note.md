# CN-7 post-closure note — marshalling, surface entropy, and the verification boundary

2026-07-17, from interactive broker sessions after R1 closed (§8.17). Feeds the R1.1
spec (Chris's doc, amendments folded there as v0.2) and the R2 prereg.

## The specimen (first live instance of the architecture's one unverifiable job)

Broker run (Chris's session): prompt asked for 243 shared among **7**; model emitted
`<call> ⟨safe_div⟩ 243 6 </call>`; cell returned 40 — **verifiably correct for the call
that was made**. Cells verify computation, not marshalling. Parse is the model's job and
the one thing the environment cannot check. Recorded as the boundary of what "verified"
means in any broker claim.

## Discriminating tests run (predictions graded)

- **Corpus grep**: divisor 7 in the sharing frame = 279/4,194 — ABOVE uniform mean.
  "7 is thin in training" graded WRONG.
- **Divisor sweep 2–9, two phrasings** (greedy): trained surface 8/8 marshalled OK
  (including out-of-range n=2); marbles/friends variant 7/8 — single miss at n=2
  (out of the trained 3–19 divisor range): `safe_div(243, 24)` — dividend-prefix
  contamination. Copy-where-trained holds in-range even under lexical shift;
  out-of-range, copying degrades into blending. The original 7→6 specimen did not
  reproduce under either reconstruction; verbatim-prompt dependence noted.
- **Interrogative register** (Chris's run): "What is 25 multiplied by 32?" →
  TinyStories reflex answers the question mark, then falls into the S3 degenerate
  attractor, harvesting prompt numbers into the collapsed mode. **Off-register math
  queries do not fail gracefully — they route to the worst available behaviour.**
  Quiet positive: the collapse output IS the S3 grammar — even fully off-distribution
  the model reaches for the call-adjacent format, not free prose. The attractor is made
  of the right material, frozen at the marginal mode; the S3 remix attacks collapse and
  rigidity with the same tokens.

## The number (the confession, quantified)

Template cardinality, digits+names+objects normalized:
**S2: 12 distinct surfaces / 25,000 rows** — 2 per frame, and the 2 is only the optional
warm-up prefix; each frame proper is ONE sentence (~2,000 verbatim repetitions).
S1: 2 surfaces per op (canonical + narrative); parity 4. Total S2 surface entropy
≈ log2(6) bits plus number slots. Every marshalling success in the battery was achieved
FROM single-sentence frames; every off-surface failure is downstream of this scalar.

**The L2 correlation plot is degenerate on this corpus** — per-frame cardinality is 1
everywhere, so accuracy-vs-entropy across frames has no x-axis variance. The honest
version is a DESIGNED experiment for R1.1: seed frames at cardinality 1 / 4 / 16 / 64,
measure marshalling per band. Cardinality becomes a manipulated variable, not a found
correlate; the figure that justifies the diversity requirements comes from the design.

## Spec amendments (folded into R1.1 spec v0.2 by Chris; recorded here so the
committed trail is doc-independent)

1. **Frame-type diversity**: phrasing banks gain declarative / interrogative /
   imperative frames ("How many does each child get?", "Work out 25 times 32");
   direct-question form ("What is X op Y?") added to BOTH S1 (in-tier → answer) and
   S2 (beyond-tier → call), straddled per the tier rule.
2. **Per-cell frame-coverage audit row**: every in-vocab cell family needs S2 frames of
   its own — mul must not live off division's template. Template-cardinality-per-frame
   becomes a standing row in the slot-diversity audit.

## Harness mitigations (no training required)

- **Call self-consistency**: sample the call twice at modest temperature before
  executing; faithful copying is near-deterministic, prior-substitution wobbles —
  disagreement is a marshalling alarm (re-prompt or flag, never execute).
- **Operand read-back** (R8 spec clause): repair loops verify operands, not just
  outputs — a trained habit of restating what was extracted, checkable by redundant
  parse.
