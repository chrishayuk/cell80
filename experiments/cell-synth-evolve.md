# Experiment: genetic search and MCTS vs A* for discovering cell chains

Status: **speculative, not on the roadmap** — same footing as `cell80-life.md`. Prompted by a
question during that experiment: could the mutation/selection mechanism built for Cell80 Life
be useful for *discovering algorithms* out of existing cells, not just for a toy ecology?

Code: `experiments/cell-synth-evolve/` (workspace member `cell-synth-evolve`,
`cargo run -p cell-synth-evolve`). The GA/MCTS/portfolio implementation lives in `src/lib.rs`
as a real public API (`evolve`, `mcts`, `portfolio`, `summarize`, ...), not just `main.rs` —
`experiments/evolved-cells` reuses it directly to test the same methods against harder,
library-derived targets rather than duplicating the search code (see `evolved-cells-findings.md`).

## What's being compared

cell80 already has an experimental synthesizer, `cell80::synth` (`cell80/src/synth.rs`): given
input→output examples, it discovers a short chain of cells that reproduces every one, via A*
search guided by Hamming distance to the target. Its own doc comment names a weak spot: "lossy"
ops (AND/OR/mask/rotate), where distance-to-target is a deceptive signal because a step can
look like it's moving away from the goal and still be necessary.

That's exactly the class of problem genetic search doesn't need a distance signal for — it
only needs a fitness score (how many examples a candidate chain reproduces), so the hypothesis
was: mutation + selection over whole candidate chains might succeed where A*'s heuristic gets
misled. This experiment builds that GA and runs it against `cell80::synthesize` on the same
benchmarks, the same op pool, the same acceptance criterion (reproduce every example exactly),
reusing `cell80::{Op, Plan}` directly so the comparison is apples-to-apples, not two
implementations of "the same idea."

The GA: a population of 150 candidate chains (each a `Vec<op-index>`, length ≤ `max_depth`),
fitness = examples reproduced exactly, elitism (top 20%) + crossover + mutation (point/insert/
delete/swap) + a small fraction of fresh random immigrants each generation, deterministic
xorshift PRNG. No distance heuristic anywhere.

A third method was added afterward: **MCTS (UCT)** — tree-structured like A*, but like the GA
it needs no distance heuristic at all. Each iteration selects down the tree via UCB1, expands
one new node, rolls out randomly to `max_depth` (or an early exact match), and backpropagates
the *fraction of examples matched* as the reward. It's a genuinely different point in the
design space from the other two, not a restatement: tree-structured search (like A*) driven by
reward statistics from random rollouts instead of a hand-designed distance heuristic (like the
GA's fitness, but exploited through a tree rather than a population).

## A bug worth naming, not hiding

The first version of `crossover` didn't cap child length at `max_depth` — single-point
crossover between two ≤8-length parents can produce a child up to 16 long, and that compounds
across generations (genetic-programming "bloat," a known failure mode, not a novel one). It
was caught empirically: a run reported `avg_depth=46.8` on a `max_depth=8` benchmark, which is
only possible if that bound is missing. Fixed by capping the crossover child's length
(`main.rs`, `crossover`). This mattered for the results, not just code hygiene: the hardest
benchmark's GA success rate dropped from a bugged 5/5 to a real 5/12 once bloat couldn't
inflate the apparent hit rate — see below.

## Results

Two op-pool sizes were tested, because the size of the pool turned out to change which method
wins — that's itself the headline finding, not a footnote.

**Small pool (11 ops, `max_depth=5`, budget 50,000 evaluations each — GA vs A* only, MCTS
added later and not rerun at this scale):** A* solved every benchmark, including the lossy
ones — slower on lossy targets (174–1379 expansions) than smooth ones (1–7), matching what the
module's docs claim, but it never actually failed within budget. GA also solved everything
(5/5 seeds each), and was not clearly better: it lost to A* on 3 of 4 lossy-ish benchmarks by
evaluation count. At this scale, the original hypothesis ("GA wins on lossy ops") did not hold
up — worth stating plainly rather than only reporting the later, more favorable result.

**Larger pool (18 ops, `max_depth=8`, same 50,000-evaluation budget, 12 seeds each for GA and
MCTS):**

| benchmark | truth depth | A* | GA | MCTS |
|---|---:|---|---|---|
| smooth-2step | 2 | found, 18 expansions | 12/12, avg 1250 evals | 12/12, avg 993 evals |
| smooth-3step | 3 | found, 1 expansion | 12/12, avg 375 evals | 12/12, avg 79 evals |
| lossy-2step (mask+rotate) | 2 | found, 35,448 expansions | 12/12, avg 1037 evals | 12/12, avg 903 evals |
| lossy-3step (mask+xor+rotate) | 3 | **budget exhausted** | **12/12, avg 612 evals** | **12/12, avg 1370 evals** |
| lossy-4step-deep | 4 | found, 17,317 expansions | 12/12, avg 6625 evals | 12/12, avg 11,675 evals |
| lossy-6step-deep | 6 | **budget exhausted** | **5/12, avg 22,380 evals** | **2/12, avg 28,444 evals** |

A solution provably exists for every benchmark here (each was generated from a real chain, at
or under `max_depth`) — so "budget exhausted" means the search space outgrew the budget, not
that nothing was findable.

## Two hybrids

With three methods showing three different profiles and none dominating, the next question
was whether combining them beats any of them alone. Two were tried, both reusing the same
benchmarks/budget/12 seeds:

**Portfolio** — run all three, keep whichever succeeded for the least effort (modeled here as
`min(tested)` across A*, GA, and MCTS for each seed; a real parallel run would cost the
*maximum* wall-clock of the three, at up to *3x the total compute* of a single method, since
all three actually run — "wins for free" only holds if you already have the machines idle).

**GA seeded from an A*-style harvest** — spend a small sub-budget (2,000 of the 50,000) on a
cheap local best-first search guided by Hamming distance (not a fork of `cell80::synth` itself,
which doesn't expose its search frontier — a from-scratch, much smaller stand-in purely to
harvest "what looked promising"), then seed the GA's initial population with the 20
lowest-distance chains found instead of starting from nothing but random chains. Total `tested`
counts the harvest and the GA together, so it's comparable to the other methods' totals.

| benchmark | Portfolio | GA seeded from harvest | plain GA (reference) |
|---|---|---|---|
| smooth-2step | 12/12, avg 18 | 12/12, avg 2150 | 12/12, avg 1250 |
| smooth-3step | 12/12, avg 1 | 12/12, avg 2150 | 12/12, avg 375 |
| lossy-2step | 12/12, avg 515 | 12/12, avg 6800 | 12/12, avg 1037 |
| lossy-3step | 12/12, avg 506 | 12/12, avg 4900 | 12/12, avg 612 |
| lossy-4step-deep | 12/12, avg 6493 | **12/12, avg 2762** | 12/12, avg 6625 |
| lossy-6step-deep | **6/12**, avg 25,426 | 2/12, avg 14,600 | 5/12, avg 22,380 |

**Portfolio's headline number isn't really the average-cost column — it's the 6/12 on the
hardest benchmark**, which beats *every single method's own best* (GA's 5/12). That's not just
"pick whichever is cheapest" — GA and MCTS apparently fail on partly different seeds, so
unioning their successes covers more ground than either alone. That reliability gain, not the
cost, is portfolio's real value here — and it costs roughly 3x the compute to get it, since all
three genuinely ran.

**The seeded hybrid is a real, honest mixed bag — not a clean win.** It only helped on one
benchmark (`lossy-4step-deep`: 2762 vs 6625, less than half the evaluations), and actively hurt
on the other five: pure overhead on every easy/already-cheap benchmark (the fixed 2,000-eval
harvest cost more than plain GA needed to solve those outright), and on the *hardest*
benchmark it made things worse — success rate dropped from GA's 5/12 to 2/12. Plausible read:
Hamming distance is most deceptive on exactly the target where this hybrid needed it most, so
seeding from it can bias the GA's initial population toward a misleading local optimum instead
of just failing neutrally — an actual regression, not a wash. This first-cut version used a
fixed sub-budget (2,000) and fixed harvest size (20) for every benchmark regardless of
difficulty; that's the most likely fixable cause, not a fundamental flaw in the idea.

## What this actually shows

- **The pool-size sensitivity is the real result.** At 11 ops A* wins comfortably; at 18 ops
  (just a bigger op pool, same targets *in kind*) A* fails outright on two benchmarks it would
  have solved at the smaller scale, while GA barely notices — its evaluation counts on the
  lossy benchmarks it *had* already solved (612 vs. the small pool's would-be-harder case)
  stayed in the same ballpark. A*'s Hamming heuristic doesn't degrade gracefully as branching
  factor grows on lossy targets; population-based search without a heuristic is comparatively
  insensitive to that growth, because it was never leaning on the heuristic's guidance in the
  first place.
- **"GA/MCTS wins" is not "reliable."** On the hardest benchmark tested (6 lossy steps), GA
  found a valid chain in 5 of 12 seeds and MCTS in 2 of 12, both using most of the budget when
  they did. That's a genuine result (something beats A*'s zero), not a strong one — this is
  closer to "worth trying as a fallback when A* fails" than "a better default."
- **Two heuristic-free methods, two different profiles — neither dominates the other.** GA
  beat MCTS on the hardest benchmark (5/12 vs 2/12) and used fewer evaluations on 4 of 6
  benchmarks overall. MCTS was markedly more efficient on the easiest one (`smooth-3step`: 79
  evals vs GA's 375 — UCB1's exploitation of a quickly-found good branch pays off when the
  answer is genuinely close by) but fell further behind as depth grew (`lossy-4step-deep`:
  11,675 vs GA's 6,625). Plausible read: GA's population keeps many diverse partial solutions
  alive at once, which matters more as the tree gets deeper and a single UCT rollout path is
  more likely to commit early to an unproductive branch; neither this doc nor the code has
  tested that explanation directly, e.g. by trying a bigger UCB1 exploration constant.
- **This directly extends `route_by_examples`' scoring idea to compositions.**
  `CellHost::route_by_examples` already ranks single library cells by how many examples they
  reproduce, with no distance heuristic. This experiment is the same scoring signal applied to
  a *chain* of cells instead of one — the natural generalization, not a new idea bolted on.

## What this does *not* show

- Two op-pool sizes and six benchmarks is a spot check, not a sweep — there's no curve showing
  *where* A* starts to fail as the pool grows, only two points either side of a threshold.
- Both the GA and MCTS here are first-cut, untuned implementations (fixed population size /
  fixed mutation rates for the GA; a fixed UCB1 constant and no rollout-policy bias for MCTS).
  A* is the one being compared against, not the one being tuned — a fairer test would also try
  improving A*'s heuristic (this is exactly the seam `synthesize_with` exposes for a learned
  value heuristic) before concluding either heuristic-free method "wins."
- "Tested"/evaluation counts aren't perfectly comparable across all three methods. A*'s
  `tested` is distinct-state node expansions (deduplicated via a visited-set). The GA's is raw
  fitness evaluations per generation (population_size × generations, not deduplicated). MCTS's
  is rollout steps (one selection+expansion, plus every step of its random rollout) — closer in
  spirit to the GA's counting than to A*'s, but still a different unit. Same spirit (search
  effort spent) across all three, not the same unit.
- MCTS wasn't run at the smaller (11-op) pool size, so there's no data on whether it also
  would have looked unremarkable there the way the GA did — the "MCTS vs GA" comparison above
  is only established at the larger, harder scale.
- Portfolio's `min(tested)` framing models perfectly parallel hardware with no coordination
  cost. It does not model (and this doc doesn't claim) the sequential case, where you'd pay
  close to the *sum* of all three methods' costs, not the minimum.
- The seeded hybrid used one fixed sub-budget (2,000) and harvest size (20) for every
  benchmark. The clean win on `lossy-4step-deep` and the regression on `lossy-6step-deep` might
  both be artifacts of that one-size-fits-all choice rather than the idea itself — untested
  here.

## Reproduce it

```
cargo run -p cell-synth-evolve
```

Benchmarks, op pool, `max_depth`, and `BUDGET` are constants at the top of `main.rs` — edit and
rerun to test other pool sizes or depths. Output is two tables: the base three-method
comparison, then a second "Hybrids" table (portfolio, seeded GA, plain GA for reference).

## What would raise confidence further

- ~~Sweep op-pool size (or `max_depth`) continuously~~ — done, in `evolved-cells`
  (`bin/boundary_sweep.rs`, see `evolved-cells-findings.md` Follow-up 4), not here: pool size
  alone (holding depth fixed) turned out not to explain A*'s failure on the one target where
  the sweep was clean; depth did, and non-monotonically — A* succeeded at shallow depths and
  failed at deeper ones on the *same* target, because a wider Hamming-guided frontier lets
  deceptive longer candidates out-compete the true chain. Worth a second pass on this repo's
  own benchmarks (`mystery`/`lossy-*`) to check whether the depth-driven, non-monotonic shape
  generalizes beyond the two targets tested there.
- Try `cell80::synthesize_with` with a smarter heuristic before concluding either heuristic-free
  method has an edge — the honest comparison is against A*'s best available heuristic, not just
  the hand one.
- Tune the GA (population size, mutation/crossover rates) and MCTS (UCB1 exploration constant,
  a biased rather than uniform-random rollout policy) instead of using first-guess values for
  both, and check whether tuning closes or widens the gap between them on the hardest benchmark.
- Test recombination/rollout at the `CellGraph` level (branching, not just a linear chain) — the
  linear-chain representation here is inherited from `cell80::synth`'s own representation, not
  a limit of either search method itself.
- Make the seeded hybrid's sub-budget adaptive (e.g. a fraction of the remaining budget, or
  cut it short once GA's own progress rate exceeds the harvest's) instead of one fixed number,
  and re-test whether the `lossy-6step-deep` regression goes away.
- Verify the portfolio's reliability gain (6/12 beating every single method's own best) isn't
  a fluke of these 12 specific seeds — rerun with more seeds and check whether GA's and MCTS's
  failures really do land on different problem instances rather than overlapping.
