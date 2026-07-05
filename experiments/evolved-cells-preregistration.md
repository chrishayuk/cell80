# Pre-registration: can selection under a cycle budget discover admission-worthy cells?

Status: **run once; results in `evolved-cells-findings.md`.** This committed to a hypothesis, a
method, and a success criterion *before* anything was executed — specifically so a good-looking
result could be checked against what we said would count as a win, rather than the write-up
quietly reshaping itself around whatever came out. That discipline is the same one
`cell80-life-findings.md` and `cell-synth-evolve.md` already used after the fact (the
crossover-bloat bug, the corrected `rapid_reproducer` decay claim); this is the same thing
stated up front instead. Result: 5/5 reachable targets passed (beating the 2/5 placeholder bar
below), the negative control correctly found nothing, and three real mistakes (a wrong
`digital_root` chain, two dialect/codegen bugs) were caught mid-run rather than discovered
later. Two of the three original scope reductions have since been closed as follow-ups: the
real admission-gate CLI (not just the fingerprint check) now confirms all 5 candidates would be
admitted, and codegen is now general (parses real cell source, not per-op hand templates). The
third (cycle-cost tracking / GA-MCTS for harder targets) was tested with one deliberately harder
target — A* still succeeded, at ~80x the search effort, so the "smooth targets, A* suffices"
reduction held at this scale but isn't proven in general. Full account in
`evolved-cells-findings.md`.

## The claim being tested

cell80 already has two authorship routes for stdlib cells: **curated** (a human writes
`cell80/cells/<name>.rs` by hand, `docs/library-growth.md`'s author→verify→admit loop) and
**generated** (`cell80::synth`'s A* discovers a chain of *existing* cells from examples,
deliberately kept narrow per that same doc). The claim: there's a third, distinct route —
**evolved** — where selection pressure, applied under an explicit cycle-cost budget rather
than just "reproduces the examples," discovers a composition that (a) is exactly correct,
(b) would be accepted by the *real* admission gate as non-duplicate, and (c) is cheap enough
in real T-states to be worth authoring as a standalone cell. If that happens even
occasionally, it's a genuinely different thing from either existing route: nobody chose the
composition, and nothing about it was "found by search for this specific target" the way
`synth.rs` is — it's a side effect of an ecology/population optimizing for survival (or
example-reproduction) under a resource constraint, the same shape of process Cell80 Life
already demonstrated for behaviour, now aimed at a library-growth-relevant question instead of
an artificial-life one.

Why this is a fair thing to ask *now*, not just an enthusiastic extrapolation: Cell80 Life
already showed selection has real, measurable teeth (the decay dose-response curve, the
`argmin3` purge) using cycle-exact execution the whole way through — most ALife work
approximates "energy" with wall-clock time or simulation step counts, both of which are noisy
and platform-dependent; cell80's `Report.cycles` is deterministic and exact by construction.
And `cell-synth-evolve` already showed GA/MCTS/hybrids searching real cell compositions
against a real alternative (A*), with a real bug caught and a real mixed result reported. The
missing piece connecting them to library growth is a cost objective and a real-admission check
— both described below, neither built yet.

## Scope, fixed now, not after seeing what's hard

**Restricted to arity-1 targets (`fn run(x: u16) -> u16`) for this first pre-registration.**
Two independent reasons force this, not a preference:

1. `cell-synth-evolve`'s `Op`/chain representation (`cell80::synth::Op::from_cell`) threads
   exactly **one** free value through a sequence of transforms, each built from a cell with
   its second argument fixed at construction time. It cannot represent a genuinely 2-free-input
   target without new work — extending it is real scope, explicitly deferred (see "What this
   would not show").
2. The real admission gate only fingerprints **arity ≤2, non-state, free functions**
   (`cell80/src/admission.rs`, confirmed by research this session — arity-3 cells and `&mut
   self` state cells are exempt from the behavioural-duplicate check entirely). Arity-1 sits
   safely inside that boundary; testing arity-2 targets is possible in principle but doubles
   the representational gap in point 1, so it's deliberately out of scope for v1 too.

This means the first version of this experiment can only ever produce evidence about
single-input transforms — a real, useful, but narrower slice of the library than "cells" in
general. Framed as a question: *does evolved authorship work at all, on the narrowest case
where it's cleanly well-defined* — not yet "how much of the library could this route cover."

## Method (what's reused vs. what still needs building)

**Already built, reused as-is:**
- `cell-synth-evolve`'s GA, MCTS, and portfolio (`experiments/cell-synth-evolve/src/main.rs`).
- `cell80::synth::Op` for the chain representation and execution.
- The real fingerprint mechanism, `cell80::{Fingerprint, DEFAULT_PROBES}` (re-exported from
  `cell80/src/fingerprint.rs`) — this experiment must call the *actual* fingerprint code the
  admission gate uses, not reimplement an approximation of it. Calling the real thing is the
  entire point; a stand-in would just be testing itself.

**Not built yet — required before any run:**
1. **Per-op cycle tracking.** `Op::from_cell` currently keeps only each precomputed
   `Fast::result`, discarding `Fast::cycles` (confirmed by reading `synth.rs`). Ranking
   discovered chains by cost needs each op's cycle cost retained (e.g. a representative or
   worst-case `T-states` per application), summed across a chain as a **search-time proxy**
   for cost — explicitly a proxy, not the final answer (see caveats).
2. **A cost-aware acceptance rule.** Currently GA/MCTS/A* all accept the first chain that
   reproduces every example. This needs a secondary objective (minimize summed proxy cycles,
   or a length/cycle Pareto front) among *already-correct* candidates, not a replacement for
   correctness.
3. **Chain → single-cell codegen.** A discovered chain is a sequence of `(existing cell name,
   fixed arg)` pairs threading one value — not itself a `.rs` source file. Turning it into a
   candidate cell means mechanically extracting each source cell's body expression and
   chaining them via `let` bindings into one `fn run(x: u16) -> u16 { ... }`, since cells can't
   call each other at the source level (each compiles independently). This is real,
   non-trivial engineering (some form of expression extraction/substitution), not a formatting
   step — flagged here so it isn't casually estimated as "just glue code" later.
4. **Full-domain correctness, not example-based.** Since `Op` tables are already precomputed
   over the entire `0..=65535` domain, correctness for the *final candidate* should be checked
   exactly over all 65,536 inputs against the target spec — a stronger bar than the
   example-based fitness `cell-synth-evolve` used, and available basically for free given the
   representation already in hand.
5. **Real fingerprint run.** Compile the codegen'd candidate as an actual `Cartridge`
   (`Cartridge::compile`, `CellConfig::sandboxed()`), compute `Fingerprint::of` against
   `DEFAULT_PROBES` on the real `Runner`, and compare (`Fingerprint::agreement`) against every
   cell currently in `cell80/cells/*.rs` — the literal mechanism `admission::admit` uses,
   applied to a candidate outside the gate rather than through it (this experiment never calls
   the real gate or touches `admission.rs`, matching the fenced-experiment discipline the rest
   of this arc has kept).

**Target list — to be fixed before running, not chosen after seeing what works.** Draw
candidates from `docs/library-growth.md`'s own "Next waves" backlog and
`docs/math-campaign-spec.md`, filtered to genuinely arity-1, `u16 -> u16` behaviours not
already in the library. Using the project's real stated gaps rather than invented toy targets
matters here specifically because the question is about library-growth relevance, not just
whether search works on an arbitrary benchmark.

## Proposed success criterion — needs your sign-off, not decided unilaterally

Draft: **at least 2 of 5 pre-selected arity-1 targets produce a discovered composition that is
(a) exactly correct over the full `u16` domain and (b) fingerprints as non-duplicate
(agreement `< 1.0`) against every cell currently in the library.** This is a placeholder
number, not a considered one — the actual bar is yours to set (or reject the whole framing as
not worth a numeric bar at all). The point of writing a number down now is that "2 of 5" can't
quietly become "well, 1 of 5 with an asterisk" after the run without that being visible as a
deviation.

## What this would *not* show, even on a clean pass

- **Passing the fingerprint is necessary, not sufficient, for real admission.** The actual
  gate also requires retrieval-dataset rows (`RefusalReason::NoRetrievalRows`) and, per
  `library-growth.md`, real aliasing judgment is often a human call beyond the mechanical
  check. A pass here is "would clear the automated novelty bar," not "would be admitted."
- **This says nothing about arity-2 cells** — the more common shape in the library — only
  arity-1. Extending requires a different search representation (two free inputs), not just a
  bigger budget.
- **The composition is still built entirely from existing cells' behaviour**, recombined.
  Whether "evolution found a shortcut nobody had assembled yet" counts as genuinely novel
  authorship, versus just an efficient rediscovery, is a fair question this doesn't resolve —
  it only makes the question answerable with real numbers instead of intuition.
- **The chain-cycle proxy (build step 1) is an estimate, not the final cell's true cost** — the
  actual compiled candidate (after codegen) needs its own real `Report.cycles` measurement as
  the final word, since compilation could plausibly change the real cost either direction from
  the sum-of-steps estimate.

## Next step

Nothing runs until the target list and the success criterion above are confirmed. If you want
to adjust the arity-1 scoping, the target list source, or the success bar, that happens here,
before any of the four unbuilt pieces get written.
