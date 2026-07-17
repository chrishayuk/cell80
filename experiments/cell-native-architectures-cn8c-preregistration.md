# CN-8c: Is the positional ceiling a PE artifact? — the peel grammar under a position intervention

**Pre-registration v1.0 — both branches priced before any CN-8c training row exists**

Chris Hay | CN Programme | 2026-07-17

---

## 1. Why this exists

CN-8b's peel arm scored 0.000 one digit past its training range; the mechanism is
length-addressing failure, not arithmetic (findings §6.1). The tempting sharpened claim —
"a self-generated tape inherits the network's positional ceiling, and cells supply the
length-addressing the tape cannot" — is, on present evidence, a fact about **RoPE (θ=10⁴)
at 115M and nothing else.** The literature contains the counterexample: Zhou et al.
(2023/2024) and the abacus-embedding line (McLeish et al. 2024) show trained scratchpads
*do* length-generalize under position encodings designed for it. The sharpened claim is one
embedding choice from falsification. CN-8c is the falsifier, registered before it runs so
the result cannot be reframed after the fact.

**This registration explicitly does not stake the cells argument on the outcome.** The
load-bearing reason to prefer a cell is verification (a checked answer at any length), which
never depended on the tape failing. CN-8c decides only whether the *secondary*
positional-ceiling claim earns unconditional phrasing or is retracted as a PE artifact.
Both branches are written in §5 with equal weight.

## 2. Pinned baselines (inherited, as measured)

| # | Quantity | Value | Provenance |
|---|---|---|---|
| E1 | Peel arm B′ s80 B0/B1/B2 exact | 1.000 / 0.000 / (TBD) | cn8b_eval_bp_s80 |
| E2 | Peel arm B′ s81 | TBD (seed rule pending) | cn8b chain |
| E3 | B′ first-error at B1 | peel-copy 200/200 | §6 |
| E4 | B′ oracle-trace NLL at B1 | 0.751 | §6 |
| E5 | Corpus (identical to CN-8b B′) | 51,031 problems, peel grammar, ≤4-digit operands | cn8b_corpus_stats.json |

CN-8c reuses CN-8b's corpus, bands (train 1–4 digit; B1 = 5; B2 = 6), the seed-90 eval
sets, and the peel grammar **unchanged**. The *only* manipulated variable is the position
encoding. This is a one-factor experiment by construction.

## 3. The intervention (frozen)

One arm, `B''`, trained on the identical peel corpus, with a single change to v11's
attention position handling. Candidate interventions, in preference order; the choice is
frozen at §6 (the pre-run engineering check) and disclosed:

1. **Randomized position offsets (Ruoss et al. 2023 style)**: during training, sample a
   random start offset for the position ids per sequence (positions stay monotonic, just
   shifted), so absolute position is decorrelated from token role. Cheapest; no architecture
   change; directly targets the "trained on positions 0..N, tested at N+" failure.
2. **Abacus-style digit-position embeddings (McLeish et al. 2024)**: add an embedding keyed
   to each digit's position *within its number* (ones, tens, …), so column identity is
   available independent of absolute sequence position. Strongest published length-gen
   effect; larger change (new embedding table, corpus must expose per-digit position).

Default: **(1), randomized offsets** — it is the minimal intervention that isolates
"absolute-position dependence" as the cause, and it needs no grammar change (§6 verifies
v11's RoPE application admits an offset hook; if it does not, fall back to (2) and re-freeze
this section before training). Two seeds (80, 81), seed rule in force.

## 4. Protocol

Identical to CN-8b §5: free-running greedy, n=200, the same seed-90 B0/B1/B2 sets, peel
grading against the model's own prior state, per-production first-error classes, oracle-trace
NLL secondary, carry-depth strata. Sanity gate: B0 ≥ 0.90 both seeds (the intervention must
not break in-range learning; if it does, that is a void, not a result, with one escalation
round as in CN-8b).

## 5. Both branches, priced before the run

| # | Prediction | Threshold | Meaning if it holds |
|---|---|---|---|
| Q1 | B0 sanity | ≥ 0.90 both seeds | intervention preserves in-range learning |
| Q2 | **The discriminator** | B'' B1 exact | — |

Q2 resolves into exactly one of two registered readings:

- **Branch A — B'' extrapolates (B1 ≥ 0.50, both seeds).** The CN-8b positional ceiling was
  a **RoPE artifact**. The sharpened tape claim's *unconditional* form is **retracted**:
  a self-generated tape *can* carry length-addressing given the right PE, so "cells supply
  what the tape cannot lay out" **falls**. The cells argument retreats — with no loss — to
  its real ground: **verification** (the tape now extrapolates but is still *sampled*; at
  B1, per-column error compounds, and CN-2's 1.6% residue is the price). Frame-projection
  (findings §8) is then a property of *default* PE, not of transformers, and gets rescoped
  accordingly. This branch **weakens the tape-vs-cell story and strengthens the honesty of
  the programme** — it is the branch that most threatens a prior commitment, and it is
  written here first on purpose.
- **Branch B — B'' still collapses (B1 ≤ 0.05, or fails the CI-disjoint test).** "Positional
  ceiling at 115M" becomes a **two-PE result** (RoPE and the intervention) and earns the
  unconditional phrasing §6.1 currently only borrows. The frame-projection phenomenon is
  then robust to PE and a stronger candidate for the mechanistic face of L8. Cells supply
  length-addressing that *this class of model* cannot, at this scale, under two position
  schemes.
- **Partial (B1 ∈ (0.05, 0.50)):** "PE helps but does not close the gap at 115M" — reported
  as directional; neither unconditional phrasing nor full retraction; a scale axis (v12)
  is then the open question, registered separately.

**The rival reading kept on the record** (findings §6.2): if *both* CN-8b and CN-8c collapse,
the residual honest position is that scratchpads do not help length generalization at this
scale at all, and the tape's only surviving contributions are decomposition + legibility +
verification — none of which is "the tape is where math happens."

## 6. Pre-run engineering check (gate, no training until it passes)

Before any B'' training: (a) confirm v11's RoPE application exposes a per-sequence offset
hook (or select intervention (2) and re-freeze §3); (b) smoke-train 30 steps and confirm
the position intervention is actually active (log the offset distribution / embedding norms);
(c) re-run CN-8b's grammar audit unchanged (the corpus is identical, so it must still pass).
Only then does the graded run start.

## 7. Commitments

1. One manipulated variable (position encoding); everything else inherited from CN-8b.
2. Both branches above are frozen before the run; neither is the "hoped-for" branch —
   Branch A is the one that costs the programme a talking point, and it is listed first.
3. The cells argument is **not** graded by this experiment; verification stands either way.
4. Seed rule (§8.15): no headline on one seed. Two seeds, both must clear.
5. Runs only after the CN-8b chain completes and MPS is free (single-user, memory-aware).
