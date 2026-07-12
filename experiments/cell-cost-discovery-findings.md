# Findings: cost pressure found four cheaper, proved-identical implementations in the stdlib

Run 2026-07-13 against the protocol in `cell-cost-discovery-preregistration.md` (written and
committed before the search ran). Harness: `experiments/cell-cost-discovery/` (`main` =
search, `confirm` = the pre-registered inline-and-recompile confirmation). **The
pre-registered success criterion is met: 4 targets have full-domain-identical, strictly
cheaper implementations found by composition search, all four confirmed after inlining, two
of them non-obvious.** A fifth raw hit is a degenerate self-composition that the pre-planned
P = 0 sensitivity lane correctly exposed — kept as an exhibit, not counted.

## The cost model in practice

- **P = 5,412 T-states** (soft shift-add mul16 mean 5,562 vs trap-mul mean 150 over the same
  u8×u8 grid). The flat 4-cycle trap charge under-priced a 16-bit multiply by roughly **36×**
  against its own substrate's software replacement — the mispricing the pre-registration
  predicted would silently kill multiplicative rewrites had it been left in place.
- Every confirmed hit also survives P = 0, so none of the four discoveries *depends* on the
  repricing — but the repricing is what makes the isqrt win rank first instead of looking
  like a modest 4.8×.

## Coverage (nothing silently dropped)

790 cell files. 541 are state cells (method entry, no free `run`) — out of scope by
pre-registration, confirmed as the *only* compile-failure bucket (`CCD_ERRORS=1`). All 249
free-fn cells compiled: 79 unary `u16 → u16`, of which 17 are partial somewhere in the domain
(halt/escalate/div-zero/budget), leaving **62 total targets**; 85 binary `(u16,u16) → u16`,
which after totality-at-constant filtering yield 1,013 constant-bound ops — **1,075 ops**
total. Depth 2: 1,152,389 viable chains, 283,984 distinct composed tables (under the 500k
cap — no truncation anywhere in the run). Depth 3: frontier × 62 unary extensions. The
sandboxed and permissive load paths produce identical results (the delta is state cells
only), so the run is genuinely library-wide for the free-fn slice.

## The four confirmed discoveries

Confirmed = hand-composed into one source, recompiled, **equal on all 65,536 inputs**
(Tier A, proved), strictly cheaper inlined. Chain-sum ratios were conservative in every case,
as the pre-registration predicted (the chain pays d call overheads, the inlined cell one).

| target | found chain | inlined repriced | inlined raw (P=0) |
|---|---|---|---|
| `isqrt` | `geomean2[b=1]` | **78.96×** | 4.79× |
| `bit_length` | `leading_zeros \|> abs_diff[b=16]` | 3.08× | 3.08× |
| `is_weekend` | `is_le[b=1]` | 1.94× | 1.94× |
| `is_odd` | `mask_intersection[b=1]` | 1.87× | 1.87× |

**`isqrt` is the headline, and it is a genuine algorithm discovery.** The authored cell calls
the `isqrt` intrinsic, which spends ~171 mul/div traps per run ((986,128 − 59,820) / 5,412).
`geomean2` computes floor(sqrt(a·b)) via a **division-free bitwise integer square root**
inlined in its own body; partially applied at b = 1 the product folds away and what remains
is a better isqrt than the isqrt cell — 4.79× cheaper in raw cycles, 79× repriced. The
library contained a superior algorithm for one of its own cells, hidden inside a *different*
cell, and cost pressure over partial applications is what surfaced it. No behavioural search
could have: the two tables are identical, so there was no behavioural hole to fill.

**`bit_length` is the aggregation-dependence exhibit the pre-registration flagged.**
`bit_length = 16 − clz` is an old identity, but *why it wins here* is the interesting part:
the authored loop runs once per bit of the value (mean ≈ 15.6 iterations on the uniform
domain), the clz loop once per leading zero (mean ≈ 1). Same function, mirrored loop counts —
and at the worst-case input the ranking **inverts** (x = 0: clz does 16 iterations,
`bit_length` does 0). The pre-committed mean-over-domain rule is what makes this a win; under
worst-case aggregation it would not be. Exactly why the aggregation had to be fixed in
advance.

`is_weekend` (`dow == 0 || dow == 1` → `dow <= 1`) and `is_odd` (`x % 2 == 1` → `x & 1`; the
canonicalizer already folds the `%2`, so the delta is the redundant `== 1` compare) are
peephole-class, but they are *proved* peepholes, found blind, and each also documents a
full-domain-equivalent pair the sampled admission fingerprint has no way to flag across
arities.

## The degenerate hit — the cost-model adversary, on schedule

`is_carmichael_number ← min[b=65280] |> is_carmichael_number` (1.005× repriced, **worse** at
P = 0). The search composed the target *with itself*, prepending a clamp that redirects the
domain's expensive tail (all inputs ≥ 65,280 have the same answer) into a cheaper region of
the same function. Full-domain identical, strictly cheaper under the letter of the criterion
— and correctly labelled `repricing-dependent` by the pre-planned sensitivity lane, which is
why it doesn't count. v2 should exclude chains containing their own target and/or require
wins at both pricings; recorded here rather than patched after the fact.

## Gate-escape audit

**Zero** unary duplicate pairs: no two total unary cells share a full-domain table, so the
admission gate's sampled fingerprint has no full-domain escapes in this slice. Worth having
checked; now checked by proof.

## What this does and does not show

Shown: the essay-level claim survives its first contact — *behavioural equivalence + exact
cost = discovered algorithms*, run mechanically over a real library, with proof-grade
equivalence (all hits exact on the full domain) rather than test-grade. The confirmed wins
are real library improvements: `isqrt`'s body should plausibly be replaced by the bitwise
loop (a 4.79× raw win for every downstream isqrt call) — left as a library decision, not made
unilaterally here.

Not shown (pre-registered limits): anything about fan-out/DAG wiring (`x*3 = (x<<1)+x`
remained inexpressible — pure pipelines only), recursion or divide-and-conquer (the Karatsuba
calibration rung stays deferred until the combinator grammar exists and a hand-computed
oracle confirms Karatsuba beats schoolbook at the chosen width under this exact cost model),
arity-2 targets (2³² domains want the GPU path), state cells, other bodies' cost regimes
(RV32's hardware mul inverts the trap economics — the same search there should *reject*
mul-avoiding rewrites, an untested prediction worth a cheap follow-up).

## Reproduce

```
cargo run --release -p cell-cost-discovery            # search (logs: run1.log/run2.log)
cargo run --release -p cell-cost-discovery --bin confirm   # inline-and-recompile confirmation
CCD_ERRORS=1 cargo run --release -p cell-cost-discovery    # compile-failure audit
```
