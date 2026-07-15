# 18 — The Discovery Challenges: candidate targets and approach (draft v0.1)

**Status:** draft for review
**Depends on:** cell-cost-discovery (4 confirmed wins, P=5412 repricing, 62d6d4e),
deterministic-ecology (EX-0…EX-5: the selection/counterfactual machinery), evolved-cells
(GA/MCTS > A* on lossy ops), the WS-E GPU interpreter (flat to 500k, ~23 ns/eval),
admission gate + fingerprints, full-domain verification (arity ≤2: 2³² in ~12 s on M3).

## 0. The reframe

The ecology's null (0/15 composed genes beat their parent) was not a failure of the
substrate — it was **a world that never required composition to score**. Purifying
selection is the correct outcome when extra computation only adds cost and failure modes.

So: **the ecology is not a world. It is a search engine.** Organisms are candidate
programs, resources are test cases, reproduction is measured performance, and the fitness
function is supplied by a *challenge* rather than invented biology. That collapses
ecology, synthesis, and cost-discovery into one programme with one grammar, one cost
model, and one counterfactual instrument.

**What cell80 brings that no other discovery system has:** an *exact* verifier
(full-domain equality, not sampled tests), an *exact* cost model (IR steps, canonical
across four bodies), a *proof* of equivalence rather than a test suite, and enough
throughput to search populations at 10⁸–10⁹ evals/s. AlphaEvolve-class systems search
with an LLM and validate by testing. This searches by composition and validates by proof.

## 1. The screen — what makes a challenge tractable here

Any candidate challenge is scored against six properties **before** it is adopted. Most
attractive-sounding problems fail on #2 or #3, and those failures are what burn months.

| # | property | why it matters |
|---|---|---|
| 1 | **exact automatic scoring** | no LLM judge, no subjective fitness — the whole point |
| 2 | **local, graded reward** | mutation needs a gradient. A global all-or-nothing objective (one bad element ruins the score) gives a cliff-strewn landscape where evolutionary search is known to fail |
| 3 | **bounded, exhaustively-checkable domain** | verification by *proof*, not sampling — this is the differentiator; give it up and cell80 is just another GP system |
| 4 | **composition must pay** | existing single cells must not already solve it. If they do, purifying selection returns |
| 5 | **compact executable output** | the artifact must be an algorithm, not a disguised lookup table (see §4) |
| 6 | **fits the dialect** | u16/Q-format, bounded steps, typed slots. A challenge needing 10⁴-length sequences and unbounded state is two hard problems at once |

**Erdős discrepancy fails #2 and #6**, and it's worth saying why explicitly because it's
the seductive one: the reward is a *global* property (max over all d, k of partial sums),
so a one-bit change to the generator flips discrepancy at a distant progression — a cliff
landscape with no gradient. The SAT constructions that pushed EDP bounds were targeted
solvers, not population search. It's a Challenge-4 target at best, and possibly never.

## 2. The ladder

### C0 — The fan-out gate *(the unlock; do this regardless of what follows)*

**Not a discovery challenge — a grammar test.** Cost-discovery found 4 wins with a
*pipeline-only* grammar (`g(f(x))`). Known improvements are provably inexpressible in it:
`3x = (x<<1)+x` needs the input twice. So does `5x = (x<<2)+x`, popcount decomposition,
branchless min/max, and every strength-reduction rewrite in a real compiler's book —
i.e. **the entire class where the wins actually live.**

**Approach.** Minimal typed DAG over existing cells: nodes = library cells + constants,
edges = typed values, fan-out allowed, intermediate reuse allowed, no recursion yet.
Lower to existing IR via the same `linearize` path composition already uses. Keep the
artifact dumb — no CellFamily registry, no provenance graph. That comes after the gate.

**Gate.** Re-run cost-discovery's 62 unary targets under the DAG grammar. **Does the
verified-win count materially exceed 4?** Plus the specific check: does it find
`x*3 ← (x<<1)+x` (or the library's equivalent) unprompted?
**Kill.** DAG grammar finds nothing pipelines didn't → the authored library is
near-optimal for this primitive set at this depth, which is itself a real result and
redirects effort to the primitive set rather than the grammar.
**Cost.** Days. **Unblocks:** cost-discovery, synthesis, ecology novelty, and the
hardware demo — one grammar, four lanes.

---

### C1 — Superoptimization over the library *(first real challenge; friendly terrain)*

**The problem.** For each library cell, find the cheapest DAG that is **full-domain
identical** to it. This is classical superoptimization — and cell80 is unusually well
suited: the equivalence oracle is exact (65,536 or 2³² exhaustive), the cost model is
exact (IR steps + repriced traps), and the search space is typed compositions of verified
cells.

**Screen:** passes all six. Reward is local and graded (cost is a continuous scalar),
domain is exhaustively checkable, composition demonstrably pays (`isqrt ← geomean2[b=1]`
already proved it at 4.79× raw), the output is a DAG, and it lives entirely in-dialect.

**Approach.** Population search (GA/MCTS — `evolved-cells` already showed both beat A* on
lossy ops) over DAGs, fitness = mean repriced cost, hard constraint = full-domain
equality. Evaluate populations × probe/full domains on the GPU interpreter. Admission
gate for dedup; P=0 sensitivity lane on every win (this is what caught the
`is_carmichael` self-composition Goodhart).

**Gates.** (i) Beat the four hand-found pipeline wins — i.e. ≥5 confirmed, robust under
P=0. (ii) **Per-body divergence:** run the same search against the RV32 cost model, where
hardware multiply inverts the economics. *Prediction: it rejects the mul-avoiding
rewrites `isqrt ← geomean2` depends on.* If that holds, the claim upgrades from
"identical behaviour on four bodies" to **"identical behaviour, per-body optimal
implementations"** — a strictly bigger claim, and one only a multi-target verified
substrate can make.
**Payoff.** Every win is a library PR. The discovery loop pays for itself immediately.

---

### C2 — Held-out generalisation: the Sidon-set family *(first mathematical construction)*

**The problem.** A cell decides membership: `include(n, state) -> bool`, generating a set
in [1, N] with all pairwise sums distinct. Maximise |S| subject to the Sidon property,
penalising IR steps, state bytes, and DAG size.

**Why it passes the screen where discrepancy fails.** Membership is a *local* decision
(gradient exists — one more admitted element is one more point of fitness), the property
is checkable exactly on a bounded N, and — crucially — **the hidden-N split separates an
algorithm from a lookup table**: train on N ∈ {64, 128, 256}, score on N ∈ {512, 1024}
never seen during search. A memorised set scores zero on held-out N; a *rule* generalises.

**Gate.** A discovered generator that (a) is valid at held-out N, (b) beats the best
single library cell, (c) is not a disguised table (see §4's size/entropy bound), and
(d) has its winning mutation identified by counterfactual revert.
**Kill.** Everything that scores well at trained N collapses at held-out N → the search
is table-fitting; tighten the size bound and re-run, or the challenge class is wrong.

---

### C3 — Improve a known frontier *(the first outward-facing claim)*

Only here does "discovery" get claimed publicly: take a published finite bound,
construction size, or program-cost frontier and beat it. Candidate lanes, all of which
have exact scoring and bounded domains: minimal straight-line programs for small fixed
functions (a literature exists, with known-optimal results to rediscover first), small
sorting networks (comparator count — a classic evolutionary-search target with known
optima to calibrate against), or the Sidon/B₂-set finite bounds from C2 at larger N.

**Approach:** rediscover the *known* optimum first (calibration), then push. Never claim
a frontier improvement without the full verification ladder (§4) and a counterfactual
lineage.

---

### C4 — Open problems *(parked, honestly)*

Erdős-class conjectures, open constructions. Requires the grammar to reach recursion, the
verification ladder to be trusted, and probably a collaborating mathematician who knows
which version of the question is actually open. **Not a target; a horizon.** Adopting it
early is how this programme would waste a year.

## 3. Sequencing

```
C0 fan-out gate (days) ──┬── unblocks cost-discovery, synthesis, ecology, hardware
                         │
                         └─► C1 superoptimization (friendly terrain, immediate PRs)
                                 │
                                 ├─► C1(ii) per-body divergence — the bigger claim
                                 │
                                 └─► C2 Sidon / held-out generalisation
                                         └─► C3 known-frontier improvement
                                                 └─► C4 (parked)
```

**The ecology's role, restated:** EX-0…EX-5 built the selection engine, the mutation
operators, the counterfactual instrument, and the RV32 export seam. C0–C3 supply the one
thing it lacked — a reward where novel computation pays. Nothing gets rebuilt; the world
gets replaced by an objective.

## 4. The standing traps (cross-cutting; every challenge inherits these)

- **The disguised lookup table.** Any unbounded-size candidate will memorise. Enforce a
  hard program-size/DAG-node bound *and* score on held-out instances. If a candidate's
  size grows with N, it isn't an algorithm.
- **The Goodhart composition.** `is_carmichael`-with-itself-behind-a-domain-clamp was
  technically identical and technically cheaper and epistemically worthless. The **P=0
  sensitivity lane** exposed it. Every challenge needs its structural analogue: a
  pre-planned instrument that asks *"is this win an artifact of the cost model?"*
- **The probe-perfect impostor.** `evolved-cells` already showed a candidate can be
  probe-perfect and wrong. Verification ladder, always: visible probes → rotating hidden
  probes → counterexample-guided refinement → exhaustive bounded → out-of-distribution N.
- **The cost model is a claim, not a given.** The trap repricing (P=5412; flat 4-cycle
  charge under-priced 16-bit multiply ~36×) means cost-model errors are *large* and
  systematic. Every headline win reports its margin under both the repriced and raw
  models, as C1's wins already do.
- **Pre-register the aggregation rule.** `bit_length ← leading_zeros |> abs_diff[b=16]`
  wins on *mean* cost and loses at x=0 — mean-vs-worst-case decided the winner. Commit
  the rule before the run, every time.

## 5. Why this is the right frame

It gives the counterfactual instrument something worth pointing at. "Candidate DAG M
appeared at generation 418; reverting M loses the improvement; inserting M into its parent
reproduces it; inserting it into other lineages tests whether the gain was contingent" is
*the same machinery* as EX-4, now attached to a result an outsider can evaluate.

And it produces the end-to-end story the ecology couldn't: **a novel algorithm, discovered
by population search, proved identical-or-better by exhaustive verification, its winning
mutation causally isolated by replay, compiled unchanged to RV32 and run on silicon.**
Every clause of that sentence is a capability that already exists. Only the reward function
was missing. see experiments folder for experiments
