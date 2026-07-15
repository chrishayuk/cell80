# Findings: C1 — superoptimization under a real cost model fails the sweep gate, with one confirmed win

Run 2026-07-14 against the protocol in `cell-superoptimization-preregistration.md`
(written and committed before the search ran). Harness: `cell80/examples/gpu_superopt.rs`.
**The pre-registered gate fails: 1 stage-2-confirmed win against a bar of ≥5.** The
failure is not "the search didn't try hard enough" in the uninteresting sense — every
target got a full budget, the two-stage repricing design worked exactly as intended
(catching one trap-hiding false positive at stage 2, the same dynamic that fooled C0),
and the one confirmed win exactly reproduces `cell-cost-discovery`'s own already-published
result.

**Update, same day:** the run as originally logged (`c1_superopt_run.log`) reported
**0** stage-2-confirmed wins, rejecting `is_weekend ← is_le(x, 1)` at 0.93× (reference
wins). That rejection was an artifact of a bug in the harness's stage-2 hand-composer,
found and fixed after the run completed (§3): `compose_source` defined each referenced
vocabulary cell as a **separate function**, which this dialect compiles to a real Z80
`CALL`/`RET`, not an inlined body — ~68 T-states of pure call overhead, enough by itself
to flip this one candidate's verdict. Fixed to genuine expression-level inlining and
re-checked (stage 1 is unaffected by this bug, so only this one candidate needed
re-verification, not the full ~2.6-hour sweep): the corrected verdict is **1.94× cheaper**,
`P_T = 0`-robust, and full-domain identical — exactly `cost-discovery`'s own
`is_weekend ← is_le[b=1]` result, found independently by this search. See §3 for the
bug and §4 for the corrected verdict. The gate tally moves from 0/5 to **1/5** — still a
clean FAIL against the pre-registered bar, but the specific finding changes.

## Headline

- **Sweep gate: FAIL.** 1 stage-2-confirmed, `P_T=0`-robust win against a pre-registered
  bar of ≥5. Per the pre-registration's own kill language: *"the authored library is
  near-optimal under [DAG-with-fan-out] composition at the searched depth."* The one
  win found (`is_weekend ← is_le(x, 1)`) is a depth-1 candidate — a gate-escape audit by
  the pre-registration's own rule (§5), not a genuine fan-out discovery, and does not
  count toward the bar even before the tally (it happens to be the only hit anyway, so
  this doesn't change the FAIL verdict, but it means the honest count of genuine
  DAG/fan-out wins is **zero**, same as C0's final tally).
- **The two-stage repricing design worked as designed.** 35 of 68 targets produced an
  IR-repriced-cheaper candidate at stage 1; 34 of those were correctly rejected by
  stage 1 itself (the IR-repriced cost was *not* actually cheaper than the reference once
  computed — i.e. stage 1's own arithmetic said "not cheaper," so these never reached
  stage 2 at all). Exactly **one** candidate (`is_weekend`) was IR-repriced-cheaper and
  reached stage 2. Stage 2 is where the design was actually tested against a live
  trap-hiding artifact, and it worked: the original (buggy) run correctly propagated a
  0.93× rejection through to the summary rather than reporting a false win — the
  composer bug made the *rejection* itself spurious, not the mechanism that produced it.
- **A real, quantified bug in the stage-2 hand-composer**, found and fixed after the run
  (§3): subroutine-call overhead (not inlining) was silently taxing every stage-2
  candidate by ~68 T-states, enough to flip the one candidate that reached stage 2 from
  a confirmed win to a rejection. Fixed via genuine `syn`-based expression-level
  inlining; regression-checked against the one candidate the bug actually affected (§4).

## 0. Coverage

68 unary total-cell targets (17 unary cells excluded upfront as partial on Z80 — same
exclusion criteria as C0), against a vocabulary pool of 170 cells (85 unary, 85 binary),
matching the pre-registration's scope exactly (§3 of that doc). `P_IR = 314.0` (IR-step
trap surcharge, measured fresh) and `P_T = 5412.0` (T-state trap surcharge, measured
fresh — an exact match to `cell-cost-discovery`'s own measurement and to this session's
`spotcheck_next_pow2_z80.rs` measurement, both independent re-derivations of the same
constant).

Per-target outcome breakdown (68 total):

| Outcome | Count |
|---|---|
| No hit (search found no full-domain-equivalent candidate within budget) | 33 |
| Found, but not IR-repriced-cheaper (correctly rejected by stage 1's own arithmetic) | 34 |
| IR-repriced-cheaper, reached stage 2 | 1 |
| **Stage-2-confirmed win** | **1** |

## 1. The one candidate that reached stage 2: `is_weekend ← is_le(x, 1)`

`is_weekend`'s reference body:

```rust
fn run(dow: u16) -> u16 { (dow == 0u16 || dow == 1u16) as u16 }
```

(`dow` is a day-of-week code where 0=Saturday, 1=Sunday.) The candidate `is_le(x, 1)`
computes `(x <= 1) as u16` — since `dow` is unsigned, `dow <= 1` is exactly
`dow == 0 || dow == 1`. This is a real algebraic identity, not a coincidence on a
sampled subset — confirmed by exact full-domain table equality (65,536/65,536 inputs)
independently at both stage 1 (`InterpBatch`) and stage 2 (`cell80::Runner`/`cpu_run`,
per the pre-registration's belt-and-suspenders equivalence requirement).

Stage 1 (IR-step space): candidate 6.0 vs. reference 7.0 IR-repriced — flagged
IR-repriced-cheaper, triggering stage 2.

## 2. `next_pow2` did not reappear

C0's re-run found exactly one trap-hiding false positive (`next_pow2`, C0 findings §5) —
the entire reason C1's two-stage design exists. Under C1's honestly-repriced stage-1
fitness, `next_pow2` produced **no hit** at all (target 45/68 in the run log): the search
never re-found that construction, because the fitness function it's searching against no
longer rewards trap-hiding from generation zero (the pre-registration's stated purpose,
§0). This is a second, independent confirmation (beyond the `is_weekend` stage-2 rejection
discussed in §3) that the repricing design is doing its job — the exact failure mode C0
exposed does not recur under C1's fitness.

## 3. The stage-2 hand-composer bug

`gpu_superopt.rs`'s original `compose_source` built a candidate's confirmation source by
defining each referenced vocabulary cell as a **separate function**, called by `run`:

```rust
// original (buggy): candidate is_le(x, 1) against is_weekend became —
fn is_le(a: u16, b: u16) -> u16 { (a <= b) as u16 }
fn run(x: u16) -> u16 { is_le(x, 1) }
```

This dialect compiles a call to a separately-defined function as a genuine Z80
`CALL`/`RET`, not an inlined body — confirmed directly by isolated measurement: a
trivial `is_le(x, 1)` call costs ~68 T-states more than the same logic spliced inline.
That overhead was enough by itself to flip `is_weekend`'s one stage-2 candidate from a
win to a loss: the original run logged

```
Z80 repriced: 230.0 vs 213.0 (0.93x) reference wins — REJECTED
```

— this session's own `next_pow2` correction (C0 findings §5) made the discipline of
verifying a mechanism before trusting it explicit, and the same discipline applies here:
a 0.93× rejection is itself a claim, and it deserved the same scrutiny a 3.59× "win"
got. Root-caused by direct comparison against `cost-discovery`'s own already-published,
trusted `is_weekend ← is_le[b=1]` (1.94× win) — a contradiction between this harness and
a prior, independently-verified result is exactly the kind of signal that should stop and
get investigated rather than reported past.

Fixed via a proper `syn`-parsed `extract_params_and_body` + word-boundary-aware
`word_replace` + a recursive `Inliner` that walks the candidate `Expr` bottom-up,
substituting each `Call` node with its callee's own body text (parameters replaced by
the caller's argument text — spliced directly for a leaf argument, matching how a human
would hand-compose it and letting the real Z80 codegen skip a register load for a
constant-bound operand exactly as `cost-discovery`'s own partial-application saving
does; routed through a fresh top-level `let` for a non-leaf argument, since this dialect
has no block-expression-as-value — confirmed directly, `{ let a = ...; ... }` used as a
value is a compile error, so nesting must stay flat). The fixed composer produces:

```rust
fn run(x: u16) -> u16 {  (x <= 1) as u16 }
```

— genuinely flat, no call, matching `cost-discovery`'s own hand-written confirmation
style.

## 4. Corrected verdict

Re-checked with the fixed composer (`cell80/examples/c1_stage2_recheck.rs` — a
standalone re-verification of just this one candidate, not a re-run of the full search;
stage 1's GA/CEGIS logic never touches `compose_source`, so the other 67 targets'
outcomes are unaffected by this bug and did not need re-checking):

```
composed candidate source: fn run(x: u16) -> u16 {  (x <= 1) as u16 }
full-domain equivalence: CONFIRMED (65536/65536 inputs match)
P_T = 5412
Z80 repriced: candidate 110.0 vs reference 213.0 (1.94x)
Z80 raw (P_T=0): candidate 110.0 vs reference 213.0 (1.94x)
==> stage 2 CONFIRMED win (P_T=0-robust)
```

Candidate and reference carry **zero trapped ops each** (`is_le`/`is_weekend` are both
pure comparison logic — no mul/div), so the repriced and raw ratios are identical: this
win does not depend on repricing at all, the opposite failure mode from `next_pow2`. The
1.94× ratio is an exact match to `cost-discovery`'s own independently-published result
for the same identity — strong corroboration that both the fixed composer and the
underlying Z80 profiling are correct, not just internally consistent.

## 5. Depth-1 rule

Per the pre-registration §5 (*"Depth-1 hits are gate-escape audits, not discoveries"* —
the same rule C0 applied to `wilson_theorem_check ← is_prime(x)`): `is_weekend ← is_le(x, 1)`
is a depth-1 candidate (one `Call` node, no fan-out, no composition of multiple library
cells). It is reported here as the run's one stage-2-confirmed win because that is what
the pre-registered gate counts mechanically, but by the programme's own standing rule it
does **not** count as evidence the DAG-with-fan-out grammar found something pipeline
composition couldn't — `cost-discovery` (a pure-pipeline search) already found this exact
identity. The honest count of genuine multi-node fan-out wins from C1 is **zero**, same
as C0's final, corrected tally.

## What this does and does not show

**Shows:** under an honestly-repriced-from-generation-zero fitness function, DAG-with-fan-out
search over this library's vocabulary, at this depth (≤4) and this budget (population
4,096, ≤400 generations), finds no genuine multi-node composition cheaper than the
library's existing implementations on the real Z80 substrate. The one candidate that
reached stage 2 duplicates a result pipeline search already had. The trap-hiding failure
mode that produced C0's one false positive (`next_pow2`) did not recur — the design
change worked.

**Does not show:** that no such composition exists (a kill here is evidence about this
grammar/depth/budget/vocabulary combination, not a nonexistence proof — the
pre-registration's own §5 language). Nothing about arity-2 targets, `shl`/`shr`-using
candidates (excluded by the disclosed `rustmsl` `ShiftVar` gap), recursion, RV32 or any
other body's economics (gate (ii), explicitly deferred), or a search-power/win-rate claim
beyond this specific budget (still an open, separately-recommended follow-up from C0's
own findings).

## Reproduce

```
cargo run --release -p cell80 --example gpu_superopt          # full 68-target sweep (~2.6h)
cargo run --release -p cell80 --example c1_stage2_recheck     # just the one stage-2 candidate
```

## Gate tally, final

- Primary gate (i): **1** stage-2-confirmed, `P_T=0`-robust win — **FAIL** against the
  pre-registered bar of ≥5.
- Gate (ii) (per-body divergence, RV32): not attempted, as pre-registered (§5, "explicitly
  deferred, not v1").
