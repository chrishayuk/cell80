# CN-1 real build — results write-up: behaviour vs. language as a tool address

**Status (2026-07-14, superseding 07-13):** **mechanism confirmed; usable level deflated on
correction; scale-invariance is now the deciding experiment.** The headline is now: *behaviour-as-
address works as a mechanism (the matched-item arm contrast — fingerprint ≪ shuffled ≈ random on
held-out, plus the seen-cell inversion — is robust and novel), and its usability is a single, well-
specified scale question.* Two headline **numbers were corrected downward** after a first-N
eval-sampling bug (the eval read the first 200 items of a cell-grouped file): held-out median rank
**~21 → ~114 of 790**; enrichment **6.73× → ~2.7× median** vs the all-790 null. And the bar itself
was corrected: per-cell top-50 recall (0.25) was an **arbitrary** standard; the execution-derived
target is *absolute rank ≤ K_exec* (~260 CPU / ~100k GPU executable candidates per token), which
rank-114 **already clears at 790** — so the deciding metric is whether absolute rank stays bounded
as the library grows (the retrained library-scale curve), not per-cell recall. **This is a
different, more honest paper than 07-13's:** a confirmed mechanism, currently at a level whose
usefulness at scale is exactly one experiment away from being decided. The reasoning trail
(including the retracted neighbourhood reframe and the inflated numbers) is preserved in the
findings and pre-registration. This document consolidates the arc; lab record is
`cell-native-architectures-findings.md`, frozen design + all corrections in
`cell-native-architectures-cn1-preregistration.md`.

## 1. The question — corrected against the literature

**Not** "can a model call cells as vocabulary." That is settled: ToolkenGPT (an embedding per
"toolken") and ToolGen (each tool a unique token) established tools-as-tokens, and we do not claim
it. The novel question sits in a gap between two literatures that do not cite each other:

- **Tool-learning** solves *unseen*-tool generalization exclusively through **language** about the
  tool — documentation/description comprehension (GenTool, TOOLVERIFIER, Re-Invoke, RaTA-Tool; the
  state of the art, CoTools, selects from natural-language descriptions over a frozen LLM; Tool2Vec
  is query/usage-derived — still language). ToolkenGPT itself "cannot use unseen tools without
  retraining" and "exhibited a strong bias toward a small subset of tools it had memorized."
- **Program-embedding** learns representations from **execution traces** (DYPRO, LiGer, Trex,
  sem2vec), precisely because syntactically similar programs behave differently — but only for
  program *analysis*, never as an LLM's token embedding.

Our conjunction is the unoccupied intersection: **executed behaviour as the tool token's address,
giving unseen tools a zero-shot address.** The precondition — exhaustively executing the whole
library, cheaply, to compute every cell's behavioural fingerprint — is what the GPU interpreter
buys; it is what makes this experiment possible at all. The paper's spine is therefore **behaviour
vs. language as a tool address**, and the state-of-the-art comparison is against description-routing
(CoTools), not against un-tooled prompting alone.

## 2. Apparatus (built and validated this session)

TinyModel v11 (115M, PyTorch/MPS, weight tying native — the pilot's precondition holds for free),
vocab extended 71261→72052 with 792 atomic cell/delimiter tokens (append-only; base rows
byte-preserved). Three-way tying: each cell token's row is `W_f(fingerprint)`, used for **both**
the input embedding and the tied output head — the load-bearing design, since a held-out cell must
be both readable and *emittable*. Constrained decoding collapses to a single-step mask over the
790 cell ids (each cell is one token), admitting held-out cells so the mask measures *selection*,
not vocabulary membership. Corpus = the H1 factory (built from scratch: `CellHost` oracle +
compositional descriptors + behavioural I/O demos), two factorized held-out axes (cells; and
template×pack compositions), four eval buckets. Every script has a structural self-test; the
gate-(ii) mechanism (a gradient step on a *seen* row moves a *held-out* row through shared `W_f`)
is verified before any training.

## 3. The experimental arc

**Behavioural-only grounding is unlearnable at this scale (a smoke-slice finding).** A corpus
grounding calls purely in I/O demonstrations made the task few-shot function identification with no
surface cue; both frozen and lightly-unfrozen bases collapsed to one cell. A probe isolated why:
the base's hidden state at the call site had cosine 0.982 within-cell vs 0.985 between-cell
(separation ≈ 0) — no operation signal for any embedding to condition on. This surfaced *before*
any GPU-hour, which is the point of a pre-registered smoke slice.

**Compositional descriptors make it learnable.** Contexts carry an operation description built from
the cell's snake_case name-words + pack via a controlled abbreviation vocabulary, so words recur
across the library. A controlled diagnostic (10 cells, ~90 examples each) then learned cleanly
(cell-acc 0.08→0.40), root-causing the collapse as *data density*, not the approach.

**The three-way control + the double dissociation (the result that matters).** Four arms share the
identical descriptor corpus; only the cell-token row source differs. Median rank of the true cell
among 790 (chance ≈ 395), one seed, fixed config (top-16, LR decay, 8000 steps):

| arm | cell-row source | seen top-1 | seen rank | **held-out rank** |
|---|---|---|---|---|
| fingerprint | `W_f(behaviour)` | 0.27 | 72 | **43** |
| shuffled | `W_f(scrambled behaviour)` | 0.475 | 2 | 566 (worse than chance) |
| random | free learned row | 0.785 | 0 | 519 (worse than chance) |

Two facts, jointly decisive:
1. **Held-out transfer is behavioural.** Scrambling the behaviour↔cell map collapses held-out
   ranking from 43 to 566 — from ~9× better than chance to worse than chance. The address signal is
   the fingerprint↔behaviour correspondence, not the projection layer, not name-similarity.
2. **A double dissociation kills the "better init" alternative.** On *seen* cells the ordering
   inverts and is monotonic along a "freedom to memorize" axis: fingerprint worst (0.27) < shuffled
   (0.475) < random best (0.785). The skeptic's default ("fingerprint just has a better-conditioned
   init / shared projection aids optimization") predicts fingerprint ≥ shuffled everywhere; the
   seen-cell inversion is the opposite. What remains is the mechanism: behavioural geometry
   constrains similar cells to similar rows — costing rank-1 precision on seen cells, buying an
   address for unseen ones. Generalization traded against memorization, along the predicted axis.
   The inversion was a prediction the hypothesis makes that nobody wrote down; it is now
   **pre-registered before the 3-seed run** so replication is confirmatory.

**Literature corroboration (unrequested).** The random arm's behaviour — memorizes seen tools,
biased to a subset, no unseen transfer — is exactly ToolkenGPT's documented failure mode. We
reproduce it and explain it mechanistically.

## 4. Honest scope

This is a **ranking** result, not yet invocation: held-out top-1 is 0.000 for every arm. On v11
that is confounded — a TinyStories base has never seen arithmetic or tool syntax, so "seen top-1
rises (0.065→0.27 under better optimization) but held-out top-1 stays 0" cannot distinguish a
ceiling from a missing prior. One seed. The `novel_cell×novel_comp` bucket (n=48) shows an
unresolved sign flip and is **not reported as a result** pending more n. Nothing here clears a
pre-registered *gate*.

## 5. Registered and in progress

- **Top-1 discriminator (running):** the SmolLM2-135M swap — size-matched, code/math-pretrained,
  weight-tying verified (or the fingerprint arm would silently null). Does a relevant prior convert
  rank→top-1? Registered outcomes distinguish *prior* from *capacity*; scaling v11 is deprioritized
  to last (uninterpretable null).
- **Mandatory description baseline (arm d), registered:** each row = `W_d(sentence_encoder(descriptor))`
  — the CoTools-style *language* address, same machinery, `bge-small-en-v1.5`. Central question:
  **does behaviour beat language as a tool address?** Pre-registered against the live risk that
  description wins.
- **The synthesized-cell ace, registered:** hold out **description-stripped** cells (behaviour only).
  Description-routing is structurally blind to an undocumented synthesized cell; behaviour still
  computes its address. If fingerprint addresses them where arm (d) falls to chance, it is the one
  experiment no description method in the literature can match — and it is exactly this library's
  own case, since cost-discovery/evolution mint cells with behaviour and no prose.
- **3 seeds** throughout, including the registered seen-cell inversion prediction.

**Bottom line:** behaviour-as-address is real — controlled, dissociated, mechanistically explained.
What is owed to make it a gate: description-vs-behaviour (the mandatory baseline), top-1 on a base
with the right prior, 3 seeds, and the synthesized-cell ace. The one novel claim in the infusion
thesis has produced its first controlled yes.
