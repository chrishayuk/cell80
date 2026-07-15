# Pre-registration: C0, the fan-out gate

Status: **pre-registered before any search was run.** Same discipline as
`cell-cost-discovery-preregistration.md`, `evolved-cells-preregistration.md`, and
`cell-native-architectures-cn1-preregistration.md` — the hypothesis, grammar, cost model,
and success/kill criteria are fixed here, before the harness runs, so the write-up is
judged against what was said would count.

This is C0 from `discovery-challenges.md` §2: **not a discovery challenge, a grammar
test.** `cell-cost-discovery` found 4 wins with a pure pipeline grammar (`g(f(x))`).
Known improvements are provably inexpressible in a pipeline: `x*3 = (x<<1)+x` needs the
input used twice. This experiment asks whether the smallest possible extension — one
point of fan-out — finds more, and specifically whether it finds that construction.

## The claim being tested

Does allowing a candidate to use its input more than once (and to combine two computed
values via a genuine two-argument library cell, not just a constant-bound one) surface
proved-cheaper implementations that a pure pipeline structurally cannot reach?

## Step 0: what's reused vs what's new, and why

**Reused wholesale:** `cell80/examples/gpu_discover.rs` already builds almost exactly
this grammar for a different purpose (independent-target rediscovery, not cost-beating).
Its `Expr` type (`Var`, `Lit`, `Call(name, args)`) is a tree over the library's unary
*and binary* free-fn cells, `Var(0)` may appear at more than one leaf (fan-out is already
representable — nothing about the existing type prevents it), `linearize` lowers a
candidate to `CellProgram` bytecode, and CEGIS (grow the probe set with counterexamples
until probe-perfect implies full-domain-correct) is the existing correctness loop. This
experiment reuses all of that machinery unchanged and adds: (a) targets drawn from
*existing library behaviour* instead of hand-written reference closures, (b) a
**cost-aware** fitness so the search doesn't stop at the first correct rediscovery, and
(c) the specific x·3 canary check.

**New, deliberately simplified — the cost model.** `cell-cost-discovery` costs in Z80
T-states with a measured trap surcharge (`P = 5,412`). That model is body-specific and
required a whole measurement stage. This experiment costs in **IR steps** —
`VmOut::Value(_, steps)`, the same step count `cpu_run`/`InterpBatch` already produce
during correctness checking, at zero extra cost per eval. IR steps are body-independent
by construction (`discovery-challenges.md` calls this out explicitly as C1's eventual
cost model too). This is a legitimate C0 simplification, not a hidden one: C0 exists to
test the *grammar*, not to produce a repriced, body-accurate discovery — that refinement
is explicitly C1's job. Every result below is reported as an **IR-step** win; nobody
should read a C0 hit as a Z80-cycle or RV32-cycle win without re-costing it there.

## Scope, fixed now

- **Vocabulary pool.** Exactly `gpu_discover.rs`'s existing filter: free-fn cells with
  `state.is_empty()`, scalar params/ret (`u8|u16|i16|u32|i32|bool`), arity 1 or 2, that
  lower (`CELL_PRELUDE` + `F32_KERNELS` inlining) to a *single* function and linearize
  with `n_locals <= 64`. Built once, reused across every target — the loader is a fixed
  cost, not a per-target cost.
- **Targets.** Two groups, both required to pass for C0 to succeed:
  1. **The x·3 canary.** `|x: u16| x.wrapping_mul(3)`, a synthetic reference closure —
     no library cell computes this (checked: no `mul3`/`times3`/`triple`/`x3`-named
     free-fn `u16 -> u16` cell exists). This operationalizes the doc's literal litmus
     test. Success requires a full-domain-verified candidate whose `Expr` tree uses
     `Var(0)` **at least twice** (a pipeline-equivalent single-use tree finding some
     other correct-but-degree-1 construction would not satisfy the canary — the point is
     fan-out, not just correctness).
  2. **The total-unary library sweep.** Every unary cell in the vocabulary pool
     (arity 1) whose own linearized program is `VmOut::Value` on **all 65,536 inputs**
     (the same totality bar `cell-cost-discovery` used, recomputed fresh inside this
     harness's own lowering path rather than imported — the two target sets are
     constructed by different lowering pipelines, cost-discovery's via `rustz80`
     canonicalize+compile, this one via the GPU-cell `CELL_PRELUDE` lowering, so exact
     set identity isn't assumed; the count and overlap are reported, not asserted).
     A candidate tree that calls the target's **own** cell anywhere is excluded before
     verification — this is the Goodhart guard `cell-cost-discovery-findings.md`
     recorded and did not yet implement (the `is_carmichael`-with-itself degenerate hit).
- **Grammar.** `Expr::Var(0)` (the input, reusable), `Expr::Lit(k)` with `k` drawn from
  the same 14-constant set `cell-cost-discovery` pre-registered
  (`{0,1,2,3,4,5,8,10,16,255,256,0x00FF,0xFF00,0xFFFF}` — kept identical for
  comparability, not re-chosen), `Expr::Call(name, args)` for any pool cell with
  `args.len()` matching its arity, built recursively. **Max tree depth: 4** (enough for
  `combiner(op(x), x)` at depth 2 with headroom; no recursion, no self-reference — `Expr`
  is a tree by construction, so acyclicity is structural, not enforced separately).
  **Known bias, disclosed:** a tree duplicates a repeated subexpression's *computation*
  (e.g. `f(g(x), g(x))` runs `g` twice), so its IR-step cost is inflated relative to a
  true DAG that shares the computed value once. This biases *against* discovery, the same
  direction as cost-discovery's chain-sum bias — hits are reported as conservative, not
  optimistic. Real DAG sharing is explicitly deferred (`discovery-challenges.md`: "no
  CellFamily registry, no provenance graph — that comes after the gate").
- **Cost.** Mean IR steps (`VmOut::Value(_, steps)`) over the full 65,536-input domain.
  Mean, not worst-case — the aggregation rule is fixed now, before any run, per the
  standing trap `discovery-challenges.md` §4 names (`bit_length`'s mean-vs-worst-case
  inversion is exactly why this is committed in advance rather than picked to fit a
  result).
- **Equivalence.** Exact table equality on all 65,536 inputs — Tier A, proved, via an
  exhaustive final scan (not the strided pre-check `gpu_discover.rs` uses to reject fakes
  cheaply; that stays as a fast first filter, but nothing counts as a hit without the
  full scan passing).
- **Search.** GA + CEGIS, reusing `gpu_discover.rs`'s population/mutation/elitism
  structure and probe-growth loop, with one addition: fitness is **lexicographic**
  (probe-match count first, tie-broken by *lower* mean IR steps among probe-perfect
  candidates — inverting the existing tie-break, which currently prefers smaller tree
  *size* as a proxy for cost; here cost is measured directly, so size is no longer the
  proxy). The search does **not** stop at the first full-domain-verified hit: it keeps
  the cheapest verified-correct candidate seen and continues for a fixed budget looking
  for something cheaper, matching the actual win condition (cheaper, not just correct).
  **Budget, fixed now:** population 4,096, up to 400 generations per target, up to 128
  counterexamples admitted to the probe set before giving up on that target (mirroring
  `gpu_discover.rs`'s existing constants) — no budget is raised mid-run to chase a
  result. **Early stop, disclosed (tightened once during the live run):** a target whose
  population fitness plateaus stops before exhausting the 400-generation ceiling —
  originally 80 generations pre-hit with no new best raw fitness, tightened to 20 after
  the first live run showed marginal fitness ticks letting a target crawl for the full
  budget with zero visibility into progress (a logging gap, fixed alongside it); 40
  generations post-hit with no cheaper verified find, unchanged. This bounds wall-clock
  across ~75 targets and never changes what counts as a hit — it only stops searching
  once the population has stopped improving. **Per-generation verification cap,
  disclosed (also found necessary during the live run):** the first cut ran the
  full-domain CEGIS check on *every* probe-perfect candidate in a generation; with a
  small probe set, many population members can spuriously agree on all of it at once,
  and one target stalled for 30+ CPU-minutes in a single generation as a result. Capped
  to the 4 smallest-tree probe-perfect candidates per generation (smallest first, as the
  cheapest-looking and likeliest to survive) — bounds per-generation cost without
  changing the equivalence or cost bar a hit must clear, and the run still does not stop
  at the first solve across a target's whole search.

## Success / kill, pre-registered

- **Canary gate:** the search finds a full-domain-verified, fan-out-using (`Var(0)`
  used ≥2 times) construction for x·3 within budget. **Kill:** it doesn't — the grammar
  extension is present but the search can't reach the one motivating example, which
  falsifies the "days, easy win" framing regardless of the sweep below.
- **Sweep gate:** count of total-unary targets with a full-domain-identical,
  strictly-cheaper (mean IR steps) DAG construction, excluding self-composition.
  **Success:** count **materially exceeds 4** — pre-registered bar: **≥ 6**.
  **Kill:** count ≤ 4 — the DAG grammar (at this depth, this vocabulary, this budget)
  finds nothing pipelines didn't; per `discovery-challenges.md`'s own kill language,
  that's a real result (the authored library is near-optimal here) redirecting effort to
  the primitive set rather than the grammar, not a failure to hide.
- **Both gates are reported independently.** A world where the canary passes but the
  sweep doesn't (fan-out is expressible and findable for a hand-picked case, but doesn't
  pay off across the library at this budget) is a coherent, reportable outcome — not
  collapsed into a single pass/fail.

## Amendment, found during the live run: an `InterpBatch` parity gap

Target tables were originally built with one `InterpBatch` (GPU bytecode-interpreter)
dispatch over the full domain, for speed. Live, `leading_ones` came back reporting a
full-domain match to the constant `0` — impossible (the cell's own doc comment gives
`leading_ones(0xFFFF) == 16`). Cross-checked against direct CPU `cpu_run`: 32,768/65,536
mismatches, all at `x >= 0x8000` — a real parity bug in `InterpBatch`'s handling of the
`& 0x8000 != 0` pattern, not a search artifact. `cell80/tests/msl_battery.rs` batteries
`GpuBatch` (`rustmsl::runtime`, the codegen/per-cell-compiled path) — a different engine
from `InterpBatch` (`rustmsl::interp::gpu`, the bytecode interpreter this harness and
`gpu_discover.rs` both use for dynamic/evolved candidates). The interpreter path has no
equivalent parity suite, so this gap was real and previously uncaught; filed as a
follow-up (see the findings doc), not fixed here — out of scope for a grammar test.
**Blast radius, assessed:** every accepted hit is verified against `cpu_run` (via
`counterexample`/`tabulate`) before it counts, so this could only have corrupted *target*
ground truth (fixed: targets are now tabulated via `cpu_run` too, matching how
candidates are verified), never let a false hit through. It may still cost the search
some power — a candidate tree that calls an `InterpBatch`-affected cell internally gets a
noisier fitness signal during the generational search — but cannot produce a wrong
accepted result.

## What this would NOT show

Nothing about C1 (superoptimizing every library cell against a repriced, body-accurate
cost model), per-body divergence, recursion, held-out generalisation (C2), or any claim
beyond "the DAG grammar, minimally implemented, finds more (or doesn't) than the pipeline
grammar did, under an IR-step proxy cost." A sweep hit here is a *candidate* for a
library PR, not a merged one — it still wants the hand-composed single-source
confirmation step `cell-cost-discovery`'s `confirm.rs` established, which is out of
scope for this pass and left as follow-up if the sweep gate passes.

## Method

`cell80/examples/gpu_fanout_gate.rs` (macOS/Metal only, matching `gpu_discover.rs`).
`cargo run --release -p cell80 --example gpu_fanout_gate`.
