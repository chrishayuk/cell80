# Findings: the fan-out gate fails both pre-registered gates — with well-understood causes

Run 2026-07-14 against the protocol in `cell-fanout-gate-preregistration.md` (written and
committed before the search ran, including its two live amendments — the `InterpBatch`
parity bug and the tightened/capped search budget — both made and disclosed *before*
this final run, not after seeing its result). Harness: `cell80/examples/gpu_fanout_gate.rs`.
**Both pre-registered gates fail.** Neither failure is "the search didn't try hard
enough" in the uninteresting sense — each has a specific, verified cause, and each cause
is itself a real finding.

**Update, same day:** the vocabulary gap §1 identifies was closed (new `raw-arith`
pack: `add`, `sub`, `shl`, `shr`) and the whole gate re-run against the richer
vocabulary. Both gates still FAIL, and the canary's root cause is now cleanly
reclassified (vocabulary gap → pure search-power gap). See §5.

**Second update, same day, correcting the first:** the sweep's one apparently-genuine
fan-out win (`next_pow2`) was spot-checked against real Z80 cycles and initially
reported (wrongly) as surviving at 3.59×. That check used the runner's *raw*
cycle count; re-run under `cell-cost-discovery`'s own **mandatory** P-repricing
(the mul/div host trap is a flat 4 T-states, ~36× underpriced against real
software-routine cost on a chip with no hardware MUL/DIV — exactly the correction
that programme exists to enforce), the win **inverts**: the composed candidate
becomes **2.57× more expensive** than the reference, not cheaper. The sweep's
"genuine fan-out win" count reverts to **zero**, now for a precise, verified reason —
not an unexplained retraction. See §5.

## Headline

- **Canary gate: FAIL** (five runs now). Root cause originally: `cell80/cells`
  contained **no standalone free-fn cell that does raw wrapping `add(a,b)` or
  `shl(a,k)`** — every two-argument arithmetic cell was either checked/saturating or a
  bounded domain computation. Fixed (§5): a new `raw-arith` pack ships `add`/`sub`/
  `shl`/`shr`, verified correct on the real Z80 substrate, and `x·3 =
  add(shl(x,1), x)` verified by hand to compute correctly including wraparound. The
  gate still didn't find it — but now for a different, narrower reason (§5): this
  harness's GPU-interpreter vocabulary loader can't use `shl`/`shr` at all (a separate,
  pre-existing `rustmsl` limitation, not something this session introduced), and even
  the `shl`-free path (`add(add(x,x), x)`, fully expressible with `add` alone) wasn't
  found within the GA's budget. Vocabulary gap closed; search-power gap remains. See
  §1 (original root cause) and §5 (re-run, reclassification).
- **Sweep gate: FAIL**, both before (1 raw hit) and after (2 raw hits) the vocabulary
  fix, against a pre-registered bar of ≥6. One hit in each run is a depth-1 duplicate
  (not a composition win, by `cell-cost-discovery`'s own precedent). The second run's
  *other* hit — `next_pow2 ← snap_up(mask_xor(x, is_zero(x)), highest_set_bit(x))`,
  3.25× cheaper under the IR-step proxy — is algebraically correct and elegant (§5
  proves it by hand, case by case) but **repricing-dependent**: it relies on two
  mul/div host-trap calls the IR-step model prices at near-zero and the real
  substrate does not, and inverts to 2.57× *more expensive* once repriced honestly
  (§5). It joins `cell-cost-discovery`'s own `is_carmichael` degenerate hit as the
  second live instance of the exact adversarial dynamic that programme's P-repricing
  discipline exists to catch — caught here on a search win, not a hand-audited one.
  What was reported earlier the same day as "the first genuine fan-out/DAG
  composition win this programme found" is corrected here: it is a genuine,
  algebraically-correct, multi-node fan-out *construction* (`x` used three times
  across two binary combinators) — but not a genuine cost *win* once repriced. See
  §2 and §5.
- **Found along the way, not exercising fan-out but real regardless:** a genuine GPU
  parity bug in `rustmsl::interp::gpu::InterpBatch` (silently zeroed probes beyond the
  pipeline's thread-per-threadgroup cap), root-caused, fixed, and now covered by a
  regression test that didn't exist before. See §3.

## 0. Coverage (nothing silently dropped)

168 vocabulary cells (85 unary, 83 binary) from the same library `cell-cost-discovery`
searched, filtered by `gpu_discover.rs`'s existing arity/scalar-type/single-function
criteria. 69 of those are unary and total (vs cost-discovery's 62 — different lowering
pipelines, disclosed in the pre-registration; the delta is a set-construction difference,
not a discrepancy to explain away, and it's exactly what surfaced the one hit reported
below, a pair cost-discovery's own vocabulary didn't contain together). 16 unary cells
excluded as partial. Plus the synthetic x·3 canary: 70 targets total, all 70 completed
(no truncation, no target skipped). (The §5 re-run, after `raw-arith` landed, saw the
same 69 total-unary targets and a 170-cell pool — see §5 for why only +2, not +4.)

## 1. The canary: infeasible vocabulary, not a failed search

Four independent runs (different budgets, different verification implementations, one
after a real Metal bug fix) all returned **`x*3 canary: NOT FOUND`**, converging on the
same non-result faster each time as the harness's plateau detection was tightened (409s
→ 90s → 54s → 52s) — the population's fitness genuinely plateaus early, which is what a
structurally-unreachable target looks like, not what a slow-but-searchable one looks
like.

Direct check of `cell80/cells` (not inference from the search): grep for any free-fn
`u16, u16 -> u16` cell doing plain `wrapping_add`/`wrapping_mul` or `<<`/`>>` by a
variable amount, unconditionally (no saturate, no check, no domain restriction) —
**none exists.** The closest candidates all fail the bar: `add_sat` saturates,
`hash_pair_sym` mixes (not raw add), `avg2` divides after adding, `rotl16`/`rotr16`
rotate (not shift), `mask_intersection`/`mask_union` are bitwise not arithmetic. Every
two-argument cell with genuine wrapping semantics found (e.g. inside `geomean2`,
`sum3`) is a bounded internal step of a larger, specific computation, not exposed as its
own callable unary-composable primitive.

**Reading `discovery-challenges.md`'s own motivating example in this light:**
`x·3 = (x<<1)+x` implicitly assumes a raw-ALU-style vocabulary (the kind a real
instruction set exposes). `cell80`'s library is deliberately curated toward
safety-checked, domain-relevant primitives — which is a *design choice* documented
across nearly every pack (`safe-arith`, `checked-arithmetic`), not an oversight. The
canary tested whether the DAG grammar could express fan-out (it demonstrably can —
`Expr` trees reuse `Var(0)` freely by construction) **and** whether the shipped
vocabulary contains the primitives the textbook rewrite needs (it doesn't). This is
exactly the redirect `discovery-challenges.md`'s own kill-condition language
anticipated: *"the authored library is near-optimal for this primitive set at this
depth... redirects effort to the primitive set rather than the grammar."* If C0-style
fan-out search is to pay off later, the actionable next step is a small set of raw
unchecked arithmetic primitives (`add`, `sub`, `shl`, `shr` as free-fn `u16,u16->u16` /
`u16,u16->u16` cells) added to the vocabulary — not a grammar change.

## 2. The sweep: zero genuine composition wins, one depth-1 duplicate

| result | count |
|---|---|
| HIT (full-domain identical, strictly cheaper) | 1 |
| found, full-domain identical, **not** cheaper | 32 |
| no full-domain-identical construction found | 36 |
| **total library targets** | **69** |

**The one HIT:** `wilson_theorem_check(x) ← is_prime(x)` — 3,414.5 vs 69,329.3 IR steps,
**20.30×** cheaper. Real, full-domain-verified (all 65,536 inputs), and a legitimate
library-improvement candidate in its own right (Wilson's-theorem primality is
famously expensive; a trial-division-style `is_prime` is the standard cheap
replacement) — **but it is a depth-1 construction** (`Var(0)` used once, no fan-out,
one cell calling another with no combination). `cell-cost-discovery-findings.md`
established the rule this falls under: *"Depth-1 hits are not discoveries: ... the
admission gate's sampled fingerprint admitted a behavioural duplicate. Those are
reported separately as gate-escape audits."* Cost-discovery's own gate-escape audit
found zero such pairs in its 62-target vocabulary; this pair exists in the 69-target
vocabulary this sweep built (the two target sets differ, per §0) and was invisible to
that earlier audit as a result. Filed here as an audit finding, not counted toward the
sweep gate — applying cost-discovery's own discipline consistently, not loosening it to
manufacture a win.

**Genuine fan-out/DAG composition wins: zero.** No candidate that combined two computed
subexpressions via a true binary cell (the entire point of C0) came back both
full-domain-correct *and* cheaper than its target's authored implementation.

**What the 32 "found, not cheaper" results show.** These are not noise — every one is a
full-domain-*proved* alternative implementation, several of them genuine, elegant
mathematical identities the search discovered blind: `leading_ones(x) =
leading_zeros(bit_not(x))` and its mirror, `is_odd(x) = is_even(is_even(x))`,
`highest_set_bit` via a reverse-bits/lowest-set-bit sandwich, `nibble_hi(x) =
nibble_lo(x·16)`. They demonstrate the grammar, the vocabulary, and the verification
pipeline all work correctly — the search is not stuck or broken, it reliably finds
*correct* constructions. It just doesn't find *cheaper* ones. Some come close:
`geomean2(x, 256)` matches `q_sqrt` at 326.6 vs 322.6 IR steps (1.2% more expensive);
`mask_has_all(1, x)` matches `is_weekend` at 8.0 vs 7.0 (14% more expensive) — a
different, non-winning path to the exact same win `cell-cost-discovery` found via
`is_le[b=1]`, discussed next.

## 2a. A validity caveat this run surfaces directly: the search is not exhaustive

`cell-cost-discovery`'s pipeline search was **breadth-first enumeration** — complete at
its declared depth, so its "zero hits" was a proof of near-optimality under that grammar.
This harness's GA+CEGIS search is **not** exhaustive, and this run demonstrates the gap
concretely: `bit_length` came back `no hit` in all four runs, despite
`cell-cost-discovery` having *already proved* a cheaper construction exists
(`leading_zeros |> abs_diff[b=16]`, 3.08× — one of the original four confirmed wins),
reachable inside this harness's own strictly *larger* grammar (a pipeline is a
degenerate DAG). The GA simply never found it within budget. This means: a **HIT** here
is still Tier-A proof (full-domain equality + a measured cost delta — nothing about a
positive result is weakened). A **"no hit"** here is *not* evidence of absence, only
evidence this search, at this budget, didn't find one — a materially weaker claim than
cost-discovery's own "zero hits." The sweep-gate FAIL above should be read accordingly:
it is not a proof the DAG grammar has nothing left to find in this library, only that
this GA run (bounded population, bounded generations, plateau-terminated for
tractability — §4) didn't surface ≥6 wins.

## 3. Side finding: a real `InterpBatch` GPU/CPU parity bug, found, fixed, covered

While tabulating targets over the full 65,536-input domain, `leading_ones` came back
full-domain-*identical* to the constant `0` — impossible (`leading_ones(0xFFFF) == 16`
by the cell's own doc comment). Bisected empirically (`cell80/examples/
diag_interpbatch_parity.rs`, a scratch harness, deleted once the finding was captured in
a real test): isolated single-cell and multi-cell tests at small probe counts matched
`cpu_run` perfectly; the *full* 65,536-probe domain reproduced the bug exactly, with the
first mismatch always at `x = 0x8000`.

**Root cause.** `InterpBatch::run` dispatched `tpg = probes.len().min(max_tpg)` threads
per threadgroup, one probe per thread, with **no loop inside the kernel** over probes
beyond that count. When `probes.len() > max_tpg`, every probe past the thread count was
simply never computed — the output buffer read back as a freshly-allocated zero,
indistinguishable from a genuine `status=0, r0=0` ("succeeded, value 0"). Silently
wrong, not an error.

**Blast radius, assessed.** Every accepted hit in this experiment is verified against
`cpu_run` (the trusted CPU reference) before counting — so the bug could only ever have
corrupted *target* ground truth (the table a candidate is compared against), never
produce a false accepted hit. It was fixed regardless, both because target ground truth
matters and because any *other* consumer of `InterpBatch` for full-domain work (there
was none yet, but there could be) would have hit the same silent corruption with no such
downstream check.

**Fix.** `InterpBatch::run` (`rustmsl/src/interp/gpu.rs`) now chunks to `max_tpg` and
stitches per-chunk dispatches into the full-sized result — no kernel change needed.
Verified by temporarily reverting to the un-chunked dispatch: the new regression test
fails at exactly `x=0x8000`, confirming the test is meaningful, not vacuous; restored,
all tests pass.

**Coverage gap closed.** `cell80/tests/msl_battery.rs` and `rustmsl/tests/corners.rs`
both battery-test `GpuBatch` (`rustmsl::runtime`, the codegen/per-cell-compiled path) —
a different engine from `InterpBatch` (`rustmsl::interp::gpu`, the bytecode interpreter
`gpu_discover.rs` and this harness both use for dynamic/evolved candidates). The
interpreter path had **zero** GPU-vs-CPU parity coverage before this run. Added:
`interp_batch_matches_cpu_run_beyond_one_threadgroup` in `rustmsl/src/interp/tests.rs`,
using the exact cell shape that exposed the bug, at the full 65,536-probe domain
(deliberately overkill versus any plausible `max_tpg`, so it still catches a regression
on different hardware).

**Also done, unprompted but in scope while touching this code:** `rustmsl/src/interp.rs`
(1,606 lines, one file) was split into `interp/{bytecode,linearize,cpu,gpu,tests}.rs` by
concern, with the public API (`linearize`, `cpu_run`, `CellProgram`, `VmOut`,
`InterpBatch`) unchanged. Verified: full workspace `cargo check`, all `cell80` examples,
and the full `rustmsl`/`msl_battery` test suites pass unchanged.

## 4. Method notes and disclosed deviations from the pre-registration

Three live amendments, each made *before* seeing the result it affected, each recorded
in `cell-fanout-gate-preregistration.md` at the time:

1. **Target tabulation and per-generation full-domain verification moved to GPU**
   (`InterpBatch`, chunked — post-fix), replacing an initial CPU `cpu_run`-only
   implementation that stalled tabulating 75 targets for 7+ minutes, and later a
   CPU-only per-generation verification path that stalled a single target
   (`is_composite`-class) for 30+ CPU-minutes because many spuriously probe-perfect
   candidates each triggered an expensive full CPU scan. Neither change altered what
   counts as a hit (Tier-A full-domain equality, either substrate agrees exactly with
   `cpu_run` post-fix) — both only changed *how fast* the same check runs.
2. **Pre-hit plateau threshold tightened** from 80 to 20 generations of no improvement,
   and a **per-generation verification cap** (4 smallest-tree probe-perfect candidates)
   added, after live runs showed unbounded per-generation and per-target cost. Neither
   changes the equivalence or cost bar a hit must clear.
3. **Even after both fixes, some targets remain genuinely expensive** —
   `is_composite`'s winning-but-not-cheaper candidate (`nibble_lo(safe_div(is_zero(
   percent_ceil(ceil_div(x, is_prime(x)), x)), wilson_factorial_mod(0, x)))`, cost
   3,506.4 IR steps) calls number-theoretic cells with real internal loops — every GPU
   fitness dispatch for a population containing such candidates does proportionally more
   per-thread work. This is inherent to the vocabulary (some cells are just expensive to
   *run*, not merely to verify), not a bug, and not something either fix above touches.

**A path-dependence worth naming, not hiding.** Two runs of the `is_weekend` target
returned different (both correct, both non-winning) constructions —
`is_gt(2, x)` at 6.0 IR steps in one run, `mask_has_all(1, x)` at 8.0 in another —
because the GPU-based full-domain check finds the *sequentially first* counterexample
where the old CPU-based check found the first counterexample in a *strided* scan order.
Different counterexamples feed different values back into the probe set, which
reshapes fitness and mutation from that point forward. This is expected GA behavior
(non-exhaustive, path-dependent search), not a correctness concern — every reported
result, whichever path produced it, is still Tier-A full-domain-verified.

## 5. The re-run: vocabulary gap closed, and a fan-out construction the cost model over-rewarded

Same day, same harness, same pre-registered protocol — only the vocabulary changed.
`cell80/cells/raw-arith/{add,sub,shl,shr}.rs` shipped (`docs/library-growth-log.md`'s
790→794 entry has the full admission story, including two false-duplicate refusals
against `add_sat`/`sub_i16` fixed by widening `DEFAULT_PROBES`, the established
house remedy for exactly this failure mode). Before re-running the search, `x·3 =
add(shl(x,1), x)` was verified by hand at the CLI, including the wraparound case
(`x=25000`: `shl(25000,1)=50000`, `add(50000,25000)=9464 = 75000 mod 65536`) — the
primitive gap named in §1 is closed, unconditionally, independent of anything the
search does.

**A second, narrower gap surfaced immediately.** The re-run's vocabulary pool grew
from 168 to 170 cells (85→85 unary unchanged, 83→85 binary — only **+2**, not +4).
`shl`/`shr` never entered the pool: `gpu_fanout_gate.rs` (like `gpu_discover.rs`)
lowers every vocabulary candidate through `rustmsl`'s bytecode linearizer for the GPU
search path, and that linearizer bails on `Expr::ShiftVar` — a shift by a *runtime*
variable amount — with `Bail::UnsupportedExpr("ShiftVar (runtime amount)")`
(`rustmsl/src/interp/linearize.rs`). This is a pre-existing, disclosed limitation of
the GPU-interpreter path specifically (the real Z80 compiler, `rustz80`, has fully
supported runtime-variable shifts for a while — `bit_is_set`/`set_bit`/`clear_bit`/
`toggle_bit` already ship using them; the CLI checks above ran `shl`/`shr` through
that Z80 path and they work perfectly). `add`/`sub` have no such issue and entered
the pool cleanly.

**This does not block `x·3`.** `add(add(x,x), x) = x+x+x = 3x`, fully expressible
with `add` alone — no `shl` required. The canary still came back `NOT FOUND` (33
generations, plateaued at `best_fit ≈ 40/48` probes matched) — a GA miss on a
findable 2-level tree within a 4,096-population, ≤400-generation budget, not a
vocabulary block. This is the same non-exhaustiveness limitation §2a already named
(the search missed `cost-discovery`'s *already-proven* `bit_length` win too), now
demonstrated a second, independent way.

**The sweep re-run found 2 hits, not 1** (bar: ≥6, still not met):

| target | winning chain | cost | ratio |
|---|---|---|---|
| `wilson_theorem_check` | `is_prime(x)` | 69329.3 → 3414.5 | 20.30× |
| `next_pow2` | `snap_up(mask_xor(x, is_zero(x)), highest_set_bit(x))` | 191.5 → 59.0 | 3.25× |

`wilson_theorem_check ← is_prime(x)` reproduced identically (same chain, same cost,
same 41 generations) — still a depth-1 duplicate, still not a composition win, per §2's
already-established rule.

`next_pow2`'s hit is different in kind: `x` appears **three times** across the tree
(once directly in `mask_xor`'s first argument, once inside the nested `is_zero(x)`,
once in `highest_set_bit(x)`), combined through two distinct binary cells
(`mask_xor`, `snap_up`) — a genuine multi-node DAG *construction*, full-domain
verified, and (below) strictly cheaper *under the pre-registered IR-step cost
model specifically*. Attribution, precisely: none of `snap_up`/`mask_xor`/`is_zero`/
`highest_set_bit` are `raw-arith` cells — this construction was reachable in the
*original* 168-cell vocabulary too, and its appearance now rather than in run 1 is
GA path-dependence (§4's already-disclosed mechanism: a different sequence of
counterexamples reshapes the whole search trajectory), not something the new
primitives caused.

**The construction is correct by algebra, not by coincidence of the probe table** —
worth spelling out, since "full-domain verified" proves it but doesn't explain it.
Each cell's exact source (`cell80/cells/{predicates/is_zero,bit-mask/mask_xor,
bit-mask/highest_set_bit,bounds/snap_up}.rs`): `is_zero(x) = (x==0) as u16`;
`mask_xor(a,b) = a^b`; `highest_set_bit(x)` isolates the value of x's top set bit via
smear-then-subtract (0 when x==0); `snap_up(x,step) = x` if `step==0 || x==0`, else
rounds `x` up to the nearest multiple of `step`. Trace every case:
- **x = 0**: `mask_xor(0, is_zero(0)=1) = 0^1 = 1` — a branchless patch for exactly
  the reference's own documented `next_pow2(0) = 1` convention (a no-op XOR for
  every other `x`, since `is_zero(x)=0` there). `highest_set_bit(0) = 0`, so
  `snap_up(1, 0)` hits its `step==0` branch and returns its own first argument, `1`.
  Result: `1`. ✓.
- **x = 2^k (already a power of two)**: `mask_xor` passes `x` through unchanged
  (`is_zero(x)=0`). `highest_set_bit(x) = x` (the top bit *is* the whole value here).
  `snap_up(x, x)`: `(x-1)/x` truncates to `0` for any `x ≥ 1`, so the result is
  `(0+1)*x = x` — unchanged, matching "the smallest power of two ≥ x is x itself".
- **2^k < x < 2^(k+1)**: `highest_set_bit(x) = 2^k`. `snap_up(x, 2^k)` is exactly
  `ceil(x / 2^k) * 2^k`; since `1 < x/2^k < 2` in this range, that ceiling is always
  `2`, giving `2^(k+1)` — correctly the smallest power of two `≥ x`, since no power
  of two lies strictly between `2^k` and `2^(k+1)`. Correct in every sub-case, not
  just the ones a finite probe set happens to sample.
- **x near the top of the domain (the reference's documented overflow case, "0 if it
  would exceed 65535")**: at `x = 65535`, `highest_set_bit(65535) = 32768`, and
  `snap_up(65535, 32768)` computes `((65535-1)/32768 + 1) * 32768 = 2 * 32768 =
  65536`, which **wraps** in `u16` to exactly `0` — verified directly at the CLI,
  matching the reference bit for bit. The construction reproduces the reference's
  overflow behavior via genuine unsigned wraparound in `snap_up`'s own multiply, not
  by luck.

So `x` is load-bearing in three structurally different roles at once — the raw value
passed through `mask_xor`, the zero-test that patches the one edge case `mask_xor`
can't otherwise reach, and the magnitude probe `highest_set_bit` reduces to a power of
two — and every branch of the case analysis lines up with the reference's own
documented behavior, including its two named edge cases (`x=0` and overflow). This is
about as strong as a fan-out win gets: not "the search got lucky within probe
coverage" but "the composition *is* the algorithm the reference loop computes,
found by a different route."

**A caveat the win invited — checked directly, then checked again, correctly.** The
reference `next_pow2` is a `while` loop whose iteration count scales with `x`'s
bit-length (up to 16 doublings); each iteration attempt charges an IR step under
this experiment's cost model (§0 of the pre-registration: IR steps, not repriced
Z80 cycles, chosen deliberately as a cheaper proxy for this grammar test). The
discovered composition is loop-free straight-line calls — and `highest_set_bit`
itself does the "same job" via an *unrolled* smear-then-subtract (four fixed
OR/shift statements, no loop at all), so its IR-step cost is constant in `x`. An
IR-step counter charges per loop-iteration *attempt* by construction
(`rustmsl/src/interp/linearize.rs:332,345`), so it structurally rewards trading a
loop for an unrolled bit-trick, independent of what either costs in real Z80
T-states — reason enough to distrust the 3.25× ratio without a hardware check.

**First pass at that check (recorded here, then corrected — not quietly redone):**
all four constituent cells compile through `rustz80` untouched (none needs
`ShiftVar`), so hand-inlining the composition into one source (`cost-discovery`'s
`confirm.rs` technique, `cell80/examples/spotcheck_next_pow2_z80.rs`) and sweeping
the full 65,536-input domain on the real Z80 body gave, on the runner's *raw*
cycle count: mean **4716.4 → 1313.0 T-states, 3.59×**, slightly better than the
IR-step estimate, with zero mismatches. This was reported as "the win survives and
grows." **That comparison was wrong** — not because the numbers are inaccurate, but
because it used the wrong cost field, and this codebase already has a name for
exactly that mistake.

**`snap_up` divides then multiplies.** `cell80::Runner` charges every `/`/`*` in
cell source a flat **4 T-states** as a host trap (`cell80/src/runner.rs:127`, "a
fast hardware op") — the identical mechanism `cell-cost-discovery`'s whole
programme exists to correct, because a real Z80 has no MUL/DIV instruction at all;
the flat trap underprices a genuine software routine by the ~36× that programme
measured (`P = 5,412` T-states). The reference `next_pow2` loop contains **no**
`/`/`*` anywhere. Re-measuring `P` fresh (reproducing `cost-discovery`'s exact
method — a trap-free shift-and-add `mul16` vs. a plain `a*b` trap cell, same
`CartridgeOpts` — inside `spotcheck_next_pow2_z80.rs` rather than importing across
crates) gives the identical **`P = 5,412.0`**, confirming it's the same substrate,
not a fluke. Repricing both candidates by their own measured trap count (composed:
2 trapped ops on every non-degenerate input, confirmed via `Report::trapped_ops`;
reference: 0, always):

| | mean raw cycles | mean trapped ops | P-repriced mean |
|---|---|---|---|
| reference `next_pow2` | 4,716.4 | 0.000 | **4,716.4** (unchanged) |
| composed `next_pow2` | 1,313.0 | 2.000 | **12,136.8** |

**The win inverts: 4,716.4 / 12,136.8 ≈ 0.389×, the *reference* now 2.57× cheaper.**
A follow-up breakdown (`cell80/examples/spotcheck_next_pow2_breakdown.rs`, isolating
each stage's own cost at representative `x`) confirms the mechanism precisely and
rules out the plausible alternative story: `highest_set_bit` alone costs ~676
T-states with **zero** trapped ops (its multi-bit shifts — Z80 has no barrel
shifter, so `>>2`/`>>4`/`>>8` cost more than `>>1` — are expensive, but not because
of any trap); `snap_up` alone costs ~531 T-states of which only 8 (2 × the flat
4-T-state charge) are the traps themselves — the *raw* comparison never saw the
traps' true cost because the model doesn't charge it there either. It's the
repricing, not the trap-call count, that does all the work: 2 traps × 5,412
T-states of real substrate cost = 10,824 T-states hidden entirely from the raw
comparison, dwarfing the entire rest of the candidate.

This is `cell-cost-discovery`'s own adversarial dynamic, materializing a second
time, on a genuine search result rather than a hand-audited one: *"under
near-free multiplication, the cheapest implementation of anything multiplicative
is 'use the trap'"* (that programme's own pre-registration) — here inverted:
under a near-free-*looking* division-and-multiply, the cheapest-*looking*
composition is the one that hides two of them behind a fixed-price host op the
comparison never re-examined. `is_carmichael`'s degenerate self-composition hit
was caught the same way, in the same programme, by the same discipline; this is
proof the discipline generalizes to a construction that has nothing else in
common with that one.

**Corrected tally, both runs combined:** 0 genuine, repricing-robust fan-out/DAG
composition wins out of 69 targets × 2 independent search attempts — the same
conclusion as the first run, now additionally covering the case where a win looked
real *and* survived one honest-seeming spot-check, and still didn't survive the
mandatory one. The sweep gate's pre-registered bar (≥6 per run) is not met, and
this correction doesn't change that verdict — it only replaces a wrong intermediate
claim with a right one before it could mislead anyone reading this file. What
*does* survive, unaffected by any of this: `next_pow2 ← snap_up(mask_xor(x,
is_zero(x)), highest_set_bit(x))` is exactly, provably, full-domain-verified
correct (§5's algebra, plus two independent numeric sweeps) — a genuine algorithmic
identity, discovered blind, that simply doesn't pay under any cost model this
programme is willing to stand behind.

## What this does and does not show

Shown: the DAG-with-fan-out grammar is real and expressible (`Expr` trees reuse `Var(0)`
freely by construction — never in doubt), the verification pipeline is trustworthy
(post the `InterpBatch` fix, and now covered against regressing), the search correctly
and repeatedly finds full-domain-*correct* alternative implementations, and — per §5 —
it found a genuine, algebraically-correct, multi-node fan-out *construction*
(`next_pow2`) that is cheaper under the pre-registered IR-step model specifically.
Also shown, the hard way: an apparent win surviving one real-hardware spot-check is
not the same as surviving the *right* one — the raw-cycle check said "3.59×, grows";
the P-repriced check (the same mandatory discipline `cell-cost-discovery` established
for exactly this failure mode: a candidate that leans on a host-trapped mul/div the
model prices near-free) said 0.389×, a clear inversion, driven entirely by two
trapped ops the raw comparison never re-examined. Both pre-registered gates fail on
both runs, and every failure — including this corrected one — has an identified,
specific, actionable cause (§1/§5 for the canary, §2/§5 for the sweep) rather than
being an unexplained negative.

Not shown: that fan-out reliably pays off here at any useful *rate*, or at all so far
— zero genuine, repricing-robust wins across 69 targets × 2 independent search
attempts is a long way from the ≥6-per-run bar, and §2a's non-exhaustiveness caveat
means the true rate isn't knowable from two runs regardless (a GA that never finds a
real win in two attempts is not proof there is none to find). The vocabulary gap
named in §1 is now closed and confirmed not to be the binding constraint (§5); the
cost-model-artifact question §5 raised for `next_pow2` is now closed too, in the
direction that costs the programme its one apparent win, not the direction that would
have banked it. What's still genuinely open is the search-power question: would a
larger population, more generations, or repeated independent runs surface a
composition that's cheaper under a defensible, trap-aware cost model *and* still not
merely a depth-1 duplicate — or does none exist at reachable depth in this library?

**This is not merely a hygiene recommendation — it's a hard precondition on that
follow-up meaning anything.** An IR-step (or raw-trapped-cycle) fitness function
doesn't just occasionally reward a trap-hiding construction; it *defines*
trap-hiding as the optimum, for any candidate arithmetic-dense enough to have one
available. `next_pow2` is what that optimum looks like when a GA gets lucky enough
to find it. A larger-budget, more-seeds search-rate study run against the *current*
fitness function would not measure "does fan-out pay in this library" — it would
mostly manufacture more `next_pow2`-shaped false positives and report their
manufacture rate as if it were an answer. Per `discovery-challenges.md`'s own
sequencing, the honest next step is therefore not another ad hoc C0 re-run but
either (a) a pre-registered *search-budget* study **with the fitness function
repriced by `P` from generation zero** (not spot-checked after the fact, and not
optional — the study is measuring an artifact rate otherwise) if the fan-out
grammar itself is still the question, or (b) moving on to C1 (superoptimization
under a body-accurate, repriced cost model, run as the real search-and-verify
protocol) where every candidate gets this scrutiny automatically, before it's ever
reported as a win.

**One thing survives independent of any of this: the identity itself.** Costs are
not ISA-portable — `next_pow2`'s composition loses on the Z80 specifically because
that body has no hardware MUL/DIV — but the algebraic identity `next_pow2(x) =
snap_up(mask_xor(x, is_zero(x)), highest_set_bit(x))` is exact regardless of body,
proved once (§5's algebra) and machine-verified twice (the IR-step full-domain
sweep and the real-Z80 full-domain sweep, zero mismatches each). On a target with
real hardware multiply/divide or a barrel shifter, the same construction could
genuinely pay — a live question for cell80's multi-target direction, not a
consolation prize. Recorded separately in `experiments/verified-identities.md` (a
new, minimal registry, first entry: this one) so a future repricing or a future
backend can re-adjudicate it for free — the expensive part, blind discovery plus
full-domain proof, doesn't need repeating.

## Reproduce

```
cargo run --release -p cell80 --example gpu_fanout_gate     # macOS/Metal only
cargo test --release -p rustmsl interp_batch_matches_cpu_run_beyond_one_threadgroup
cargo run --release -p cell80 --example spotcheck_next_pow2_z80        # §5: raw AND P-repriced
cargo run --release -p cell80 --example spotcheck_next_pow2_breakdown  # §5: per-stage cost isolation
```
