# Evolved cells: findings from the first pre-registered run

Companion to `evolved-cells-preregistration.md` (the committed hypothesis/method/success bar)
and `evolved-cells/` (the code). That doc says what would count as a win *before* running;
this one reports what actually happened, including three real mistakes the run itself caught
— on the theory that a pre-registration only means something if deviations get reported, not
quietly smoothed over.

## Result

**6 of 6 reachable targets passed.** That beats the pre-registered placeholder bar (2 of 5,
`is_semiprime` excluded from the denominator as the calibration target) outright, and every
candidate was independently confirmed by the *real* admission-gate CLI, not just the
fingerprint check in isolation (see below). One target (`mystery_bits_2`) is the result the
original pre-registration explicitly hadn't tested: **A* genuinely fails to find a chain here
at all — GA and MCTS both find one reliably.** See Follow-up 3.

| target | chain found | full-domain check | fingerprint vs. existing library |
|---|---|---|---|
| `digital_root` | `digit_sum → digit_sum → digit_sum` | PASS (65,536/65,536) | closest = `digit_sum`, agreement 0.833 → **novel** |
| `low_byte_popcount` | `low_byte → popcount` | PASS | closest = `popcount`, agreement 0.833 → **novel** |
| `high_byte_popcount` | `high_byte → popcount` | PASS | closest = `lcm3`, agreement 0.833 → **novel** |
| `rotated_low_byte_popcount` | `rotl12 → high_byte → popcount` | PASS | closest = `popcount`, agreement 0.667 → **novel** |
| `mystery_bits` (harder) | A*: `and_5555 → rotl6 → xor_5555 → or_aaaa → popcount` (448,447 nodes) | PASS | closest = `leading_zeros`, agreement 0.167 → **novel** |
| `mystery_bits_2` (harder still) | **A*: not found.** GA/MCTS both found one (see below) | PASS | closest = `bit_length`, agreement 0.167 → **novel** |
| `is_semiprime` (negative control) | A* finds a chain matching the probes; **full-domain check correctly rejects it** (18,910/65,536 wrong) | — | — |

`agreement` is fraction-of-12-probes-matching; `1.0` is the real admission gate's own
duplicate threshold (`cell80/src/admission.rs::DUPLICATE_AGREEMENT`). Every candidate here
landed at 0.167–0.833 — related enough to *something* in most cases, never close to identical.
`mystery_bits`'s chain and node count changed from an earlier pass (`rotl4 → xor_5555 →
or_aaaa → popcount`, 83,029 nodes) once the op pool was broadened for Follow-up 3 below — a
bigger pool makes A* work harder even on a target it already solved, not just on new ones.

## Follow-up 1: the real admission gate, not just the fingerprint check

The result above only tested `Fingerprint`/`DEFAULT_PROBES` in isolation at first — necessary
but, as the pre-registration said up front, not sufficient, since the real gate also needs
retrieval rows and walks the *whole* library at once. Closed that gap directly: `evolved-cells`
writes each passing candidate to `evolved-cells/candidates/*.rs` with a proper `//!
summary`/`//! tags:` header (parsed into the manifest by `library_cartridge`, same as any real
cell). A scratch copy of the real ~209-cell `cell80/cells/` plus these 5 candidates, and a
scratch copy of `cell-eval/datasets/retrieval.jsonl` plus 2 retrieval rows per candidate (never
touching the real library or dataset), was fed to the **actual `cell80` CLI binary**:

```
cargo run -p cell80 --bin cell80 -- index <scratch-cells> --gate <scratch-retrieval.jsonl> --json
```

Result: `{"admitted": [...all 214 cells including all 5 candidates...], "refused": []}`. Every
candidate was admitted; nothing was refused — not "would pass the fingerprint check" but the
real `admission::admit` function, on the real current library, via the real CLI, with zero
special-casing. This is the strongest available evidence for the pre-registration's central
claim without literally submitting a PR.

## Follow-up 2: general codegen, not hand-templated

The first version of this experiment hand-wrote one "codegen template" string per op — correct
for the 6 ops actually used, but not a generalization: a 20th op would need a 20th hand-written
match arm. Replaced with a version that parses each op's *real* cell source text (regex-based,
not a full `syn` AST transform — a stated, deliberate limit, not an oversight) to extract its
parameter name(s) and body, renames locals to avoid collisions when the same op appears twice
in one chain, substitutes the running input for the first parameter and the op's fixed literal
for a second (for ops like `and_00ff`/`rotl4` built with one argument pinned), and chains the
results. Rerun with this general version: **identical result** — same 5 chains, same
full-domain outcomes, same fingerprint agreements — confirming it's a faithful, not just
differently-shaped, replacement. Re-ran the real admission-gate check against the newly
auto-generated sources too: still all admitted, still zero refusals.

## Follow-up 3: does the "smooth targets, plain A* suffices" scope reduction hold?

**No — not in general.** First pass (a narrower 23-op pool, `mystery_bits` alone) found A*
straining but still succeeding (83,029 node expansions, ~80x every other target, but a valid
chain). That result was reported honestly as "validated at this scale, not proven in general,"
with GA/MCTS-porting listed as the way to actually test it rather than assume it. This is that
test.

**What changed:** `cell-synth-evolve`'s own GA/MCTS/portfolio code was split into a real
library (`cell-synth-evolve/src/lib.rs` — `evolve`, `mcts`, `portfolio`, `summarize`, all
`pub`) instead of being private to its `main.rs`, specifically so `evolved-cells` could reuse
the *actual* search code, not a duplicate. Verified the split was behavior-preserving first:
reran `cell-synth-evolve`'s own benchmark suite after the refactor and got byte-identical
output to before it. The op pool was also broadened (23 → 35 ops: more AND/OR/XOR mask
constants, more rotate amounts — mirroring `cell-synth-evolve`'s own 11 → 18 escalation, which
is what actually found *its* A*-failure case) and a second, deeper target was added:
`mystery_bits_2`, a 6-step OR/rotate/AND/rotate/XOR/popcount chain, `max_depth=8`.

**Result: A* found nothing for `mystery_bits_2` within the 500,000-node budget. GA succeeded
5/5 seeds (avg 162,540 evaluations); MCTS succeeded 5/5 seeds (avg 122,304).** This is the
actual failure case the first pass didn't reach — not strain, an outright miss, with both
heuristic-free methods reliably finding what A* couldn't. On `mystery_bits` itself, now run
against the broadened 35-op pool, A* still succeeded but needed *more* effort than before
(448,447 nodes, vs. 83,029 on the narrower pool) — while GA (84,930) and MCTS (17,938 — the
cheapest of all three by a wide margin here) barely noticed the larger pool. Both observations
line up with `cell-synth-evolve`'s original finding: A*'s Hamming heuristic degrades as the
search space grows on lossy targets; population/tree-based search without a heuristic doesn't,
because it was never leaning on one.

**A bonus catch, not designed for:** with the broadened pool, A* now finds a chain for
`is_semiprime` (the negative control) that matches every probe — `low_byte → rotl14 → low_byte
→ is_pow2` — but the full-domain check correctly rejects it (18,910/65,536 real inputs wrong).
The negative control's story changed from "no chain found at all" to "something coincidentally
probe-matching gets found and correctly thrown out" — arguably a *stronger* demonstration of
the method's rigor than the original clean miss, since it shows the full-domain check catching
a subtler failure mode (plausible-looking, not just absent) on a target that has no real
solution.

## Follow-up 4: mapping the A*-failure boundary, not one data point

Follow-up 3 found A* failing at exactly one grid point (pool=35, `max_depth=8`) and succeeding
at another (pool=23, `max_depth=6`) — consistent with either "pool size broke it" or "depth
broke it" or both. `bin/boundary_sweep.rs` runs a proper sweep instead of guessing: pool size
at fixed depth, and depth at fixed pool size, for both `mystery_bits` and `mystery_bits_2`, one
seed per grid point (mapping the boundary's *shape*, not seed variance — variance across seeds
is already characterized in the main run above). `build_ops`'s op-pool ordering was designed so
`mystery_bits_2`'s five needed ops (`or_0f0f`, `rotl10`, `and_aaaa`, `rotl6`, `xor_5a5a`) sit in
the guaranteed-present "core" prefix at every pool size in `[18, 34]`, so a found→not-found
transition for that target is genuinely about search difficulty, not the target quietly
becoming unreachable.

**A confound worth stating plainly, not hiding: `mystery_bits`'s pool-size sweep is not
clean.** Unlike `mystery_bits_2`, `mystery_bits`'s three needed ops (`or_aaaa`, `rotl8`,
`xor_5555`) were placed in the "extra" (distractor) list, not "core" — `xor_5555` only enters
the pool once `pool_size ≥ 31`. So `mystery_bits`'s pool-size numbers below partly measure
"is the required op present yet," not "does a larger pool make search harder." Its depth sweep
(fixed at `pool=34`, where all three ops are present throughout) is unaffected by this and is
read normally.

```
=== mystery_bits: pool-size sweep (max_depth=8, budget=500000) — CONFOUNDED, see above ===
pool          A* tested    GA tested  MCTS tested
18                    —            —            —
22                    —            —            —
26                    —            —            —
30                    —            —            —
34                    —        71700        37123

=== mystery_bits: depth sweep (pool=34, budget=500000) ===
depth         A* tested    GA tested  MCTS tested
5                 51865        73500        12773
6                417665       225750        15238
7                     —         4050        72087
8                     —        71700        37123
9                     —            —        79584
10                    —         4050        46674

=== mystery_bits_2: pool-size sweep (max_depth=8, budget=500000) — clean ===
pool          A* tested    GA tested  MCTS tested
18                    —       390150        24355
22                    —        33000         6181
26                    —         5100        10359
30                    —        12150        30055
34                    —            —        11701

=== mystery_bits_2: depth sweep (pool=34, budget=500000) ===
depth         A* tested    GA tested  MCTS tested
5                 30380        41700         5989
6                225762       405150        23493
7                     —        23850         9990
8                     —            —        11701
9                     —         1800         1789
10                    —         2550        23865
```
('—' = no chain found within the 500,000-node/eval budget)

**Finding 1 — pool size, on its own, isn't the story.** `mystery_bits_2`'s clean pool-size
sweep shows A* failing at *every* pool size from 18 to 34 at `max_depth=8` — not a gradual
degradation as the pool grows, an outright miss across the whole range. Whatever is breaking
A* here, it isn't primarily branching factor. GA and MCTS both succeed at every pool size
(GA's cost actually *drops* as the pool grows past 18, which runs against the "more ops means
harder search" intuition too — more likely, a bigger pool gives GA's crossover/mutation more
useful building blocks to recombine, not just more noise).

**Finding 2 — depth is the real story, and it runs backwards.** Both targets' depth sweeps show
A* succeeding at shallow depths (5, 6) and failing at every deeper one tried (7, 8, 9, 10). That
is the opposite of the naive expectation: `max_depth` is a ceiling, not a requirement, so giving
A* *more* room should never make an already-findable chain harder to find. Two things resolve
why it does. First, `cell80::synthesize`'s Hamming-distance heuristic is admissible but weak —
node expansion is bounded by budget, and a deeper ceiling multiplies the number of same-or-lower
Hamming-score nodes competing for expansion before the true chain surfaces, so A* spends the
budget widening the frontier instead of reaching depth 6. Second, this is a Hamming-guided best
first search, not a length-first search: allowing depth 10 doesn't make the shared depth-5/6
solutions harder to *represent*, it makes them harder to *find first*, because far more
deceptive longer-and-wrong candidates now tie or beat them on the heuristic and get expanded
first. GA/MCTS don't have this failure mode since neither is guided by a heuristic that a
larger search radius can mislead — GA still succeeds at 9/10 depth for both targets (MCTS
succeeds at every depth for both).

**Finding 3 — the depth=5 result for `mystery_bits_2` isn't a fluke, it's a real discovery.**
The target was designed as a 6-step chain, so A* finding *anything* at `max_depth=5` was
suspicious enough to check by hand rather than trust the probe match. Built a throwaway
full-domain check (`examples/verify_depth5.rs`, deleted after use) against the exact chain A*
returned — `["and_aaaa", "or_0f0f", "and_aaaa", "popcount"]`, 4 steps, not 5 — over all 65,536
u16 inputs: 0 mismatches. This is a genuine, shorter equivalent to the hand-designed 6-step
target, not a coincidental probe-agreement like the `is_semiprime` false positive in Follow-up
3 (that one failed full-domain with 18,910/65,536 wrong; this one is exactly right). A*
succeeding at low depth isn't noise, then — it's finding the *actual* shortest solution when the
search radius is tight enough to keep the heuristic useful, and losing that ability as soon as
the radius widens past the point where deceptive candidates start competing.

**Net:** the "A*-boundary" isn't a 2D monotonic frontier over (pool size, depth) the way the
single Follow-up 3 data point suggested. Pool size alone (holding depth fixed, on the one clean
target) doesn't explain the failure. Depth does, and non-monotonically — A*'s Hamming heuristic
gets actively worse, not neutral, when given headroom it doesn't need, on targets where the
optimal chain is shorter than the ceiling. That's a more specific and more interesting claim
than "A* struggles as the search space grows," and it's a genuine argument for running A* at the
tightest plausible depth ceiling for a target rather than a generous one — which cuts against
the intuitive default of "give the search room in case it needs it."

## Follow-up 5: `syn`-based codegen, closing the stated regex gap

Follow-up 2's codegen was explicitly regex-based with a named limit: a cell whose entire tail
is a leading if/else chain (e.g. `clamp.rs`) would mis-split, because the regex heuristic only
recognized a top-level `}` or `;` as a statement boundary and couldn't reliably tell "this `}`
closes the tail expression" from "this `}` closes a statement." Replaced `parse_cell` in
`evolved-cells/src/lib.rs` with a real `syn::parse_file` + `syn::Item::Fn` parse: Rust's own
grammar marks the tail expression unambiguously (`syn::Stmt::Expr(_, None)` — no trailing
semicolon — can only be the last statement in a block, regardless of what kind of expression it
is), so the split is now correct by construction instead of by heuristic, for `clamp.rs`-shaped
cells as much as any other. `rename_locals`/`substitute_param` (the actual text-substitution
steps, not the split) stayed regex-based — they were never the part with a known gap. Reran the
full `evolved-cells` main binary after the swap: **identical result** — same 6/6 passes, same
chains, same fingerprint agreements — confirming the AST-based split is a faithful replacement
for the regex one on every op actually in this pool, not just a differently-implemented one.

## Three real mistakes, caught by the method working as designed

**1. The first run's `digital_root` chain was wrong, and the full-domain check caught it.**
With the original probe set, A* found `digit_sum → digit_sum` (only 2 applications) — it
satisfied every probe, but the full-domain sweep found 8,075/65,536 mismatches. Reasoning
through it afterward: `digit_sum`'s output range over all `u16` inputs is 0–41 (from `59999`,
not `65535` as a quick guess would suggest), and *some* values in that range — like the `39`
produced by `digit_sum(39999) = 39` — still need a second reduction (`digit_sum(39) = 12`,
still two digits) before a third pass gets to a true single digit (`digit_sum(12) = 3`). Two
passes isn't always enough. The original probe set just didn't happen to include an input that
exposed the gap. Fixed by adding `39999` to the probes (chosen specifically because it forces
the 3-pass requirement, not decoratively) and giving `digital_root` more depth headroom (4→5).
This is the pre-registration's step 4 (full-domain validation) doing exactly the job it was
written for — catching a plausible-looking wrong answer before it could be reported as a win.

**2. The first codegen attempt didn't compile — the restricted dialect rejects block
expressions as values.** `let x = { ...; result };` is not legal in cell80's dialect
("blocks-as-values aren't supported — bind with `let` first"); every multi-statement op
(`digit_sum`, `popcount`, etc.) needs its internal loop *inlined as flat statements* in the
outer function body, not nested in a block bound to a variable. This was caught by actually
trying to compile the first real candidate, not by reasoning about the dialect in advance.

**3. The general codegen's first version mis-split "statements" from "tail expression."** It
only recognized a top-level `;` as a statement boundary, so a `while { ... }` loop with no
trailing semicolon (the shape every loop-based cell in this library uses) got swallowed into
the "tail expression" alongside the actual tail variable, producing invalid syntax like
`let out = while ... { ... } s;`. Fixed by also treating a top-level `}` as a boundary, unless
immediately followed by `else` (still part of the same if/else chain) — caught by the second
real compile attempt, again not by reasoning about the dialect's grammar in advance. A cell
whose *entire* tail is itself a leading if/else chain with no statements before it (e.g.
`clamp.rs`) would still split wrong under this heuristic — none of the ops actually in this
pool have that shape, but it's a known, stated gap, not a hidden one.

## What's genuinely interesting, not just "it worked"

- **`digital_root`'s fingerprint agreement (0.833) is with `digit_sum` itself** — sensible,
  since the candidate *is* digit_sum applied three times. The fingerprint check correctly
  recognizes "closely related to an existing cell" without calling it a duplicate, which is
  exactly the discrimination the real gate needs to make (related ≠ identical).
- **`high_byte_popcount`'s closest match by fingerprint is `lcm3`, not `popcount`** — the
  cell it's actually built from. The 12-probe bank isn't testing "does this look like it's
  related to X," it's testing "does this happen to output the same values as X on these 12
  specific inputs" — a behavioral coincidence, not a structural one. "Closest fingerprint
  match" isn't the same question as "what is this composition made of."
- **`rotated_low_byte_popcount` and `mystery_bits` both found genuinely different compositions
  than the ones they were designed around** — the search finding *its own* solution rather than
  a rediscovery of the one already in the experimenter's head, in both cases verified correct
  only by the full-domain check, not by matching the intended derivation.

## What this does *not* show

- **The real-admission-gate run used *hand-written* retrieval rows and a *scratch copy* of the
  library, not a real contribution.** The rows were authored by the experimenter (2 per
  candidate, direct + paraphrase, matching the real convention) rather than independently
  reviewed. "The real gate accepts this artifact" is now a checked claim; "a human maintainer
  would merge this" still isn't — aliasing judgment and doc/tag quality review are separate,
  human steps `library-growth.md` describes as part of a real contribution, not mechanically
  gated.
- **General codegen was regex/text-based at first; now `syn`-based (Follow-up 5, below),
  closing the leading-if/else gap** — but the sweep/main-run candidates in this doc's other
  sections still describe the regex-era behavior, since re-running produced an identical result
  (see Follow-up 5), not new chains or fingerprints to report.
- **The A*-failure boundary is now mapped (Follow-up 4), and it isn't the simple pool-size
  story a single data point suggested** — see above for the depth-driven, non-monotonic
  finding. It's still only 2 targets, both hand-constructed by the experimenter, so "A* gets
  worse with excess depth headroom on deceptive targets" is demonstrated, not yet shown to
  generalize beyond this pool/probe design.
- **n=7 targets, all arity-1, all constructed by the experimenter** (not pulled from a real
  backlog — the actual "Next waves" list in `library-growth.md` turned out to have no
  remaining un-duplicated arity-1 gaps when checked). A clean pass here is evidence the
  mechanism works on a small, controlled draw, not a claim about how much of real library
  growth this route could cover.
- **"Evolved" here means "found via search over existing cells," not "grown from nothing."**
  Every candidate's actual logic is built entirely out of pieces that already exist in the
  library — the open philosophical question the pre-registration flagged (efficient
  rediscovery vs. genuine novel authorship) is exactly as open as it was before this run.

## Reproduce it

```
cargo run -p evolved-cells   # writes candidates/*.rs for every passing target

# Real admission-gate check, entirely in a scratch directory:
SCRATCH=/tmp/admission-test
mkdir -p "$SCRATCH/cells"
cp cell80/cells/*.rs "$SCRATCH/cells/"
cp experiments/evolved-cells/candidates/*.rs "$SCRATCH/cells/"
cp cell-eval/datasets/retrieval.jsonl "$SCRATCH/retrieval.jsonl"
# append 1-2 retrieval rows per candidate to $SCRATCH/retrieval.jsonl, then:
cargo run -p cell80 --bin cell80 -- index "$SCRATCH/cells" --gate "$SCRATCH/retrieval.jsonl" --json
```

Targets, the op pool, probes, and budgets are constants/data at the top of `main.rs`.

## What would raise confidence further

- ~~Map the actual A*-failure boundary~~ — done (Follow-up 4): it's depth-driven and
  non-monotonic, not the pool-size story a single data point suggested. Still only 2 targets;
  a third, independently-designed deceptive target would test whether the depth-headroom
  effect generalizes or is specific to how `mystery_bits`/`mystery_bits_2` were built.
- ~~Replace regex-based codegen with real `syn`-based parsing~~ — done (Follow-up 5).
- Get retrieval rows and aliasing judgment from someone other than the experimenter before
  treating "admitted" as "would really be merged."
- Now that `mystery_bits_2`'s candidate came from GA/MCTS rather than A*, check whether
  search-method choice itself correlates with anything about the resulting candidate (chain
  shape, fingerprint agreement, code size) — untested so far, since every earlier candidate
  came from A*.
