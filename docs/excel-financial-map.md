# Excel financial-functions coverage map — Microsoft's 55 functions mined against cell80

*Status: **landed, 2026-07-11.** The coverage-map deliverable
(`docs/real-valued-cells-spec.md` Part 2's "build before authoring anything"
discipline, applied to a new source catalogue) for a prospective **Finance80** pack.
Machine-readable form: [`cell80/data/excel_financial_catalog_map.json`](../cell80/data/excel_financial_catalog_map.json).
Classified against `docs/coverage-map-taxonomy-amendment.md`'s taxonomy by 55
independent agents (one per function, each reading the live `docs/cell-index.md` and
citing real cell80 ids rather than guessing), via `Workflow` — no cell authored yet.*

## What this is

Microsoft documents 55 functions in its dedicated "Financial functions" category
(`support.microsoft.com/en-us/excel/financial-functions-reference`, fetched
2026-07-11 — the full alphabetical list, not a curated subset). Each was classified
against the current **653-cell** cell80 library into exactly one status:

| status | meaning |
|---|---|
| `covered` | an existing cell80 cell already does this |
| `composable-skip` | existing cells compose to it, no reason to author anyway |
| `composable-author` | existing cells compose to it, but well-known/expected functionality or convention fragility justifies authoring anyway |
| `candidate` | genuinely new logic, bounded, real cell80 fit — the actual backlog |
| `host_only` | needs a dialect/harness capability that doesn't exist today |

## Headline numbers

```
55 functions total
  35 candidate            (64%)  <- the real backlog
  12 host_only            (22%)
   7 composable-author    (13%)
   1 composable-skip       (2%)
   0 covered                    (no surprise — no finance pack exists yet)
   0 out_of_scope
```

By category (Excel's own functions, grouped by the map's working taxonomy — not an
official Microsoft grouping):

| category | total | candidate | host_only | composable-author | composable-skip |
|---|--:|--:|--:|--:|--:|
| time_value_of_money | 20 | 7 | 8 | 5 | 0 |
| depreciation | 7 | 5 | 0 | 1 | 1 |
| day_count_and_coupons | 8 | 8 | 0 | 0 | 0 |
| bonds_and_securities | 15 | 11 | 4 | 0 | 0 |
| treasury_and_conversions | 5 | 4 | 0 | 1 | 0 |

## The two hard walls behind `host_only` — and the one soft wall that isn't

Every `host_only` row cites one or both of two genuine dialect/harness gaps. A third
gap (day-count conventions) shows up constantly but never alone — it always resolves
to `candidate`, not `host_only`, because it's buildable prerequisite work, not a
missing capability:

1. **Array-state fields — ~~gap~~ CLOSED (2026-07-11, `.cell` v11)**: `u16[N]`/`u32[N]`
   state fields round-trip by name (`StateCell::set_array`/`get_array`,
   `CellHost::run_state_values`; close-out in
   `experiments/sliding-window-state-cells-findings.md`). The functions it blocked —
   `IRR`, `MIRR`, `NPV`, `FVSCHEDULE` (array-only), `DURATION` (array, compounded by
   day-count), `XIRR`, `XNPV` — are now *expressible* and await authoring as their
   own wave (note the dialect still has no `[f32; N]` fields, so cash-flow arrays
   arrive as `u32[N]` bit-pattern envelopes or scaled ints; price that when
   authoring).
2. **Transcendentals — ~~wall~~ SHIPPED (2026-07-11, F2)**: owned `fexp`/`fln`/`fpow`
   kernels (`rustz80/src/softfloat.rs`, class approximate — fexp/fln measured ≤ 1 ulp,
   fpow ≤ 40 ulp over |y·ln x| ≤ 60, vs MPFR golden tables in
   `rustz80/tests/diff/f32_trans.rs`; trig is the still-unshipped F2 slice). `NPER`
   and `PDURATION` (transcendentals-only) landed the same day as the proof pair —
   the pack's first cells through `.ln()`. Still gated: `ODDFPRICE`, `ODDLPRICE`,
   `PRICE` (need the day-count plumbing composed in), `XIRR`, `XNPV` (need the
   array-input wave above too).
3. **Day-count conventions** (30/360 US/European, actual/actual, actual/360,
   actual/365 — `basis` parameter arithmetic). cell80's `calendrical-checksum` pack
   has real calendar primitives (`day_of_week`, `day_of_year`, `days_between`,
   `days_in_month`, `is_leap_year`, `is_valid_date`, `is_weekday`, `is_weekend`) but
   nothing that turns a basis code into a year-fraction. This shows up in roughly
   half the `candidate` rows (see below) but **never as the sole reason for
   `host_only`** — it's real, unbuilt prerequisite work, not a dialect wall, so a
   function blocked only on day-count lands in `candidate`, gated on that
   prerequisite landing first.

A useful confirmation of the taxonomy itself: **not one row was misjudged
`host_only` for merely being "complicated."** Every `host_only` verdict names a
specific missing capability, and every function with genuine multi-step algorithmic
complexity but no missing *capability* (Newton iteration, annuity-balance loops,
Nth-root extraction) correctly landed in `candidate` instead — the reason-2
misclassification check in `docs/coverage-map-taxonomy-amendment.md` did its job.

## A correction to the taxonomy amendment's own worked example

`docs/coverage-map-taxonomy-amendment.md` Part 4 pre-classified "PMT/FV/PV/NPV"
together as one `composable-author` group. Running the real coverage map against
Excel's actual signatures splits that group: **`NPV` is not composable-author, it's
`host_only`.** `PV`/`FV`/`PMT` take a fixed handful of scalars (`rate, nper, pmt,
[fv], [type]`) — an ordinary annuity shape with no array-state-field exposure. `NPV`'s
defining feature is the opposite: `NPV(rate, value1, [value2], ..., [value254])`
takes an arbitrary-length list of independently-valued cash flows, which is exactly
the array-state-field gap. The taxonomy doc's worked table conflated "same TVM
family" with "same classification" — corrected there; recorded here as the reason.

## The 7 `composable-author` — ready to build now, no prerequisites

| function | composes from | reason |
|---|---|---|
| `FV` | `frac_pow` + `compound_increase_by_bps` | well-known/expected; sign convention + omitted-arg defaults are fragile |
| `PV` | `compound_original_before_increase` + `geometric_series_sum` | same, plus the annuity-due `(1+rate)` multiply is easy to get subtly wrong |
| `PMT` | `frac_pow` + shipped f32 arithmetic | same fragile-convention shape as FV/PV |
| `ISPMT` | `bps_of` + `frac_of_whole_floor` | well-known; easy to conflate with IPMT's amortized formula |
| `EFFECT` | `div_floor_u32` + `compound_increase_by_bps` | well-known; divide-then-compound-then-subtract-1 ordering is fragile |
| `SYD` | `sub_checked_u32` + `triangular` | well-known depreciation method; 1-indexed remaining-life term is an easy off-by-one |
| `TBILLYIELD` | `days_between` + checked-arithmetic glue | well-known; simpler than PMT/FV even, no compounding loop at all |

## The 1 `composable-skip` — the taxonomy doc's own tentative row, now resolved

`SLN` (straight-line depreciation, `(cost - salvage) / life`) was left as "a genuine
judgment call" in the taxonomy amendment's worked example. Resolved here:
**composable-skip.** It has no sign convention, no omitted-argument default, and no
loop or case analysis to get wrong — reason 1's convention-fragility angle doesn't
apply the way it does for FV/PV/PMT, and reason 2 doesn't apply either (it's glue,
not an algorithm). Corrected in `docs/coverage-map-taxonomy-amendment.md`.

## The 35 candidates, grouped by real dependency structure

**Buildable today, no prerequisite pack needed (13):** `IPMT`, `PPMT`, `NOMINAL`,
`RATE`, `RRI`, `CUMIPMT`, `CUMPRINC` (time_value_of_money — each needs genuine new
algorithm: annuity-balance loops, Newton/secant iteration, or Nth-root extraction,
but no missing capability) · `DB`, `DDB`, `VDB` (depreciation — declining-balance
loops with per-step case logic, same story) · `DOLLARDE`, `DOLLARFR`
(treasury_and_conversions — pure digit/decimal-literal encoding, no date dependency
at all) · `TBILLEQ` (treasury — its own row confirms "uses raw actual-day DSM with
no basis parameter," so it never touches the day-count-convention gap).

**Gated on a day-count-convention prerequisite pack (21), plus one semi-gated (1):**
every `day_count_and_coupons` function (`ACCRINT`, `ACCRINTM`, `COUPDAYBS`,
`COUPDAYS`, `COUPDAYSNC`, `COUPNCD`, `COUPNUM`, `COUPPCD` — 8) · all 11
`bonds_and_securities` candidates (`DISC`, `INTRATE`, `MDURATION`, `ODDFYIELD`,
`ODDLYIELD`, `PRICEDISC`, `PRICEMAT`, `RECEIVED`, `YIELD`, `YIELDDISC`,
`YIELDMAT`) · `AMORDEGRC`/`AMORLINC` (depreciation — 2). `TBILLPRICE` (treasury) is
the one semi-gated case — its own row calls it "cell80's first actual/360 day-count
arithmetic, trivial here" — it only needs the plain actual/360 divide, not the
fuller 30/360/actual-actual family. One shared foundation unlocks most of the
21: `COUPNCD`'s own row flags "add N months to a date with end-of-month clamping"
as the primitive the whole `COUP*` family needs, and a basis-dispatch year-fraction
cell family (30/360 US, 30/360 EU, actual/actual, actual/360, actual/365) unlocks
essentially every `bonds_and_securities` candidate at once.

**This is the single highest-leverage prerequisite in the whole map.** 21 of 35
candidates (60%), plus `TBILLPRICE`'s lighter touch, are gated on it.

## The 12 `host_only` — was: none reachable; now (2026-07-11): 11 shipped, 1 priced out

Same-day close-out of the wave: `NPER`, `PDURATION` (the F2 proof pair), then
`NPV`, `FVSCHEDULE`, `IRR`, `MIRR`, `XNPV`, `DURATION`, `PRICE`, `ODDFPRICE`,
`ODDLPRICE` — **all shipped**. Conventions the wave set:

- **Cash-flow arrays** ride `u32[N]` state fields carrying f32 bit patterns (the
  dialect has no `[f32; N]`); the host writes `f32::to_bits` per element and the
  cell reinterprets with the new zero-cost `f32_from_bits` builtin. Envelope sizes
  are **cycle-budget-priced, per cell**: NPV/FVSCHEDULE 16, MIRR 12, IRR 8 (each
  secant walk re-walks the array), XNPV 4 (each flow pays a full `fexp`).
- **Iteration is walk-priced**: IRR runs a bounded secant (5 walks ≤ 6 flows / 4 at
  7–8, convergence checked on the iterate *before* paying the next walk);
  DURATION abandoned the per-period walk for the **geometric closed form**
  (O(log N) via square-and-multiply — any realistic schedule fits, where
  mduration's landed per-period loop tops out under ~20 periods at the default
  budget).
- **`XIRR` is the one that stays out — priced, not killed**: each XNPV evaluation
  costs ~1 full `fexp` per flow (~330K T), and a secant needs 4–6 evaluations —
  ≥ 4M T-states at even 3–4 flows against the 2M default budget. It becomes
  buildable if (a) hosts pass a raised budget through the MCP surface, or (b) a
  cheaper owned `fexp` lands. Recorded here so nobody re-derives the dead end.
- The wave also exposed and fixed a **gate blind spot**: every same-shape
  frequency-gated bond cell escalated on every probe (no probe carried a valid
  frequency), so the gate false-refused the landed `excel_oddlyield` as a
  "duplicate" of the new `excel_oddlprice` — fixed at the root by widening
  `DEFAULT_PROBES` with `[2, 0, 1]` (the signed-deltas precedent).

Every one of these has a real fixed-arity variant worth naming even though it
doesn't match Excel's actual signature: `irr_3`/`npv_4`-style cells over a small
fixed number of named scalar cash-flow fields would dodge the array-state gap
entirely and could land as ordinary `candidate` cells today — several rows note this
explicitly. Not built here because the map classifies against Excel's real
signature, not a workaround shape, but it's a legitimate design option if Finance80
wants partial IRR/NPV coverage before the array-state-field harness gap closes.

## Update (2026-07-11): Wave 1 landed — 44 of 48 authored, 697 cells

All 42 non-`host_only` functions (7 `composable-author` + 35 `candidate`) plus the
6-cell day-count prerequisite pack were authored, mechanically verified (compiled,
run against hand-worked test values, not trusted on say-so), and put through the
real admission gate. **44 survived, 4 were backed out** — library moved
**653 → 697 cells**, gate clean (0 refusals), full `cargo test -p cell80` green,
retrieval kill-gate **not tripped — all three splits improved** (direct 0.8042→0.8136,
paraphrase 0.3866→0.4140, adversarial 0.5000→0.5125 on the 697-cell corpus).

**Backed out (4):** `day_count_30_360_us` (fingerprint-agreed with `days_between` at
1.00 — investigated, genuinely not a true duplicate, a probe-bank false positive of
the same class as this project's documented `snap_down`/`luhn_check` cases, backed
out anyway per the standing "never force a flagged pair through" rule) ·
`excel_coupdaysnc` and `excel_coupncd` (each fingerprint-agreed with a sibling —
`excel_coupdays`/`excel_couppcd` respectively — same probe-coincidence story,
confirmed genuinely different algorithms, backed out anyway) · `excel_yielddisc`
(this one **is** a real duplicate: Excel's own well-known quirk that
`(redemption-pr)/pr*(B/DSM)` is algebraically identical to `excel_intrate`'s
formula — folded into `excel_intrate`'s tags instead of shipped separately).

**Real repairs during verification, not just rubber-stamping:** `excel_nominal`'s
Newton loop originally used O(npery) repeated multiply/add and blew the default
cycle budget at npery≥4 — rewritten with binary exponentiation (the technique
`excel_db`/`excel_rri` already needed). `excel_rri` and 22 other excel-financial
cells needed `//! kernel_bank: on` to fit the real 8192-byte sandboxed cap once the
full f32-kernel family was inlined (8.7–11.8 KB unbanked) — this also required
patching `cell80/tests/codegen_golden.rs` itself, which unconditionally compiled
cells unbanked. `excel_received`'s claimed test value was wrong, not the cell (exact
real-division vs. the true correctly-rounded f32 answer — cross-checked against
numpy); widened the harness to a relative float tolerance. `excel_yield` genuinely
needs ~12-15M cycles (documented in its own manifest, same cost-scaling precedent as
`is_prime_u32`) — priced, not treated as a defect.

**Retrieval regression check, done properly, not just gated on aggregate:** isolated
"old queries against the new 697-cell library" and found 13 rank-1 flips — 10
unrelated corpus-wide TF-IDF noise, 3 real finance-vocabulary collisions
(`effect_size_r` vs `excel_effect`, `decrease_by_bps` vs `excel_pricedisc`/
`excel_disc`, `mul_sub_checked_u32` vs the coupon family). Fixed the cleanest one
(`effect_size_r` gained "effect"/"degrees-of-freedom" tags, verified no
regressions); the other two are reported, not chased further, since the aggregate
gate is net positive and further tag-chasing has its own regression risk.

**What's left of the original 55**: the whole former-`host_only` twelve resolved
2026-07-11 — `NPER`/`PDURATION` with the F2 kernels, then `NPV`, `FVSCHEDULE`,
`IRR`, `MIRR`, `XNPV`, `DURATION`, `PRICE`, `ODDFPRICE`, `ODDLPRICE` in the
ex-host_only wave the same day. Only `XIRR` remains, and it is *priced out of the
default cycle budget*, not blocked on a capability (the arithmetic is in the
host_only section above).

## How this gates authoring

Per `docs/real-valued-cells-spec.md` Part 2 and `docs/coverage-map-taxonomy-amendment.md`:
a proposed cell whose row here says `host_only` is refused outright; `composable-skip`
is refused by default. The 7 `composable-author` rows and 35 `candidate` rows are the
real backlog — but re-check each against the *current* `docs/cell-index.md` before
authoring (this snapshot is 2026-07-11; the library keeps growing). The admission
gate itself remains the final, authoritative check at authoring time, exactly as the
math-server map's own equivalent section states.
