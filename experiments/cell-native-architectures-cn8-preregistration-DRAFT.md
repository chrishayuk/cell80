# CN-8: The Abstraction Threshold — surface diversity as the manipulated variable

**DRAFT v0.0 — NOT PINNED. Thresholds and predictions marked TO-PIN await Chris's
review; nothing here is registered until this header changes and the pin is committed.**

Chris Hay | CN Programme | July 2026

---

## 1. Purpose

Five programmes hit the same wall: Rogue-1 experts (100% on trained templates, 17–27%
GSM-8K, full fine-tuning WORSE at 7%), KnnStore (canonical 10/10 → narrative 0/10),
cell80 retrieval (the gcd/range_check misfire → paraphrase suite as hard gate), GPT-OSS
(`7*8=` vs `7 * 8 =`), and CN-7 R1 broker marshalling (12 surfaces / 25,000 rows; in-range
copy flawless, one lexical step off → substitution; off-register → mode-collapse basin).
The law all five instantiate: **transformers are template engines** — a model generalises
exactly as far as its template inventory reaches, and frontier models differ by inventory
(internet-scale surface diversity), not kind.

On a synthetic-corpus base, surface entropy is not free — it is a manufactured, budgeted
input. CN-8 makes it the manipulated variable and measures the missing constant: **the
abstraction threshold at 115M** — the surface cardinality at which the cheapest fit flips
from fill-the-slot to parse-the-meaning. Every prior programme treated the wall as local
failure; this one measures its height. (Scope honesty: the constant is measured at 115M
on this task family and base; cross-scale transfer is a hypothesis the curve makes
testable, not a result it delivers.)

## 2. Design

Matched midtrain arms differing ONLY in S2 surface cardinality, same token budget
(15M), same species ratios (S3 fraction raised per R1's budget verdict — TO-PIN),
S1 EOS supervision fixed (R1 filed gap), FFN policy per R1: frozen (variance) unless
the multi-seed W_f gate (§8.16 third configuration) is adopted — TO-PIN.

| arm | S2 surface cardinality per frame | source |
|---|---|---|
| C1 | 1 (R1's corpus, re-cut) | template |
| C8 | 8 | hand templates, skeleton-diverse by construction |
| C64 | 64 | templated combinatorics + frame-type bank (declarative/interrogative/imperative) |
| C512 | ~512 | LLM-paraphrased prose segments (Tiny-GSM register style transfer) |

**Rogue-1's warning, heeded**: hand-built variants share structural fingerprints. The
audit therefore carries TWO cardinality rows per frame: digit/name-normalised template
cardinality AND **function-word-skeleton cardinality** (ordered closed-class scaffold;
POS-pattern upgrade if too coarse). C8/C64 must be skeleton-diverse or they are C1 in
the currency that matters; arms are re-cut until the skeleton row matches the nominal
cardinality within TO-PIN tolerance.

**Audit compatibility for the LLM slice**: paraphrase prose segments ONLY — call spans
and injected answers keep canonical structure. Slice-specific audit: operand numerals
present in prose; injected span zero-loss; answer numeral absent from loss-bearing text.
The §3.3 mask property is unchanged and re-audited by the independent path.

## 3. Measurement

Primary: **fresh-surface marshalling accuracy** — held-out surfaces (unseen templates
AND unseen paraphrases; skeleton-disjoint from every training arm by construction),
score = emitted call has correct cell + verbatim operands. Secondary: the R1 battery
(role-NLL, P-a probes, deck, P-b/eval), per arm; call self-consistency disagreement
rate as the harness-side marshalling alarm calibration.

Curve: marshalling accuracy vs log-cardinality (nominal and skeleton), per frame type.

## 4. Pre-registered fork (the KnnStore D1 shape)

- Curve crosses usable (TO-PIN: fresh-surface marshalling ≥ 0.90) at some C* ≤ 512 →
  C* is the abstraction threshold; every future spoke's diversity budget becomes a
  lookup; R1.1 proceeds with corpus engineering priced.
- Curve saturates below usable (flat slope by C512, level < TO-PIN) → the problem was
  never data: **parse capacity** is the binding constraint, the one job the broker
  cannot delegate; effort redirects to harness mitigations (call self-consistency,
  operand read-back) and v12's parse budget. LARQL reading: the depth at which a 115M
  parser tops out, measured.
- Curve still rising at C512 without crossing → distinguishable by slope; verdict
  "insufficient diversity supplied, threshold above manufactured range" — decide
  internet-scale-style augmentation vs capacity reading BEFORE seeing which (rule
  TO-PIN).

## 5. Predictions — TO-PIN by Chris before any corpus is cut

(Explicitly left blank in this draft. Candidate registrable quantities: monotonicity;
C8-vs-C1 skeleton-controlled null; the C* range; per-frame-type ordering; whether the
S3 degenerate attractor dissolves with surface entropy or needs its own fix.)

## 6. Provenance

R1's graded record: `cell-native-architectures-cn7-preregistration.md` §8.13–8.17;
`cn7_marshalling_note.md` (the 12/25,000 confession, the verification-boundary
specimen, frame-diversity amendments). Instruments inherited: role-NLL, deck,
permutation-null yield, cn7_broker, manifest + overwrite guards, multi-seed P-d gates
(single-seed gates retired programme-wide).
