# Growing the cell library — toward a large prebuilt collection

> **Before authoring a new cell, check [`docs/cell-index.md`](cell-index.md)** — every
> landed cell, grouped by pack, generated from the actual library (not hand-transcribed, so
> it can't drift the way this file's old cell-count prose once did). It's the fastest way to
> confirm a behaviour doesn't already exist before writing a duplicate the admission gate
> would just refuse.

*The goal is a **big, growing library of prebuilt cells** — hundreds of small, distinct,
deterministic integer utilities an agent can retrieve, run, and compose, organized into
**packs**. cell80's whole pitch is "millions of tiny tools, retrieved" — so the library should
be **broad**: the more genuinely-distinct behaviours sit on the shelf, the more an agent finds
one instead of writing code. This guide is how to grow it well.*

## The shape we're building toward

```
wave 1 ✓   59 cells   predicates · safe arithmetic · bounds · percent · ranking · bit/mask
wave 2 ✓   98 cells   + number theory · distance · bit/encoding · hashing · stats · conversion
wave 2.5    96 cells   + the wide u32-in-state siblings (square_wide, weighted_sum_wide);
                        4 folded into aliases by the admission gate (see below)
wave 3a   100 cells   + calendrical/checksum, first slice (is_leap_year, days_in_month,
                        day_of_week, luhn_check) — ISBN/IBAN/UPC deferred (see below)
wave 3b   103 cells   + Q8.8 fixed-point, first slice (q_mul, q_div, q_lerp) —
                        q_sqrt/piecewise sigmoid-tanh still open
wave 3c   108 cells   + agentic runtime primitives, first slice (token_bucket_step,
                        backoff_next, circuit_breaker_step, debounce_step, hysteresis) —
                        rate_window_update still open
wave 3d   111 cells   + running statistics, first slice (running_min_max_step,
                        streak_step, accumulate_step) — Welford variance still open
wave 3e   114 cells   + spatial/grid, first slice (grid_index, point_in_rect,
                        aabb_intersect) — Morton encode/decode, Bresenham still open
pilot     120 cells   + the Phase 2.3 pilot batch: packing/BCD (pack_u8, pack_nibbles,
                        bcd_encode, bcd_decode) + vector (dot2, norm2_sq) —
                        unpack_lo/unpack_hi never built (exact duplicates of
                        low_byte/high_byte, caught before authoring)
wave 3f   128 cells   + the GSM8K math campaign, M1 pack 1/5: checked/exact arithmetic
                        (mul_u16_u16_to_u32, add_checked_u32, sub_checked_u32,
                        div_exact_u32, div_floor_u32, div_ceil_u32, mod_u32, fits_u16) —
                        see `docs/math-campaign-spec.md`
wave 3g   134 cells   + M1 pack 2/5: money/basis-points (bps_of, increase_by_bps,
                        decrease_by_bps, original_before_bps_increase,
                        original_before_bps_decrease, cents_mul_qty) — tax/tip/markup
                        consolidated into one increase_by_bps cell rather than three
                        near-identical ones
wave 3h   138 cells   + M1 pack 3/5: units (same_unit_check, unit_mul, unit_div,
                        unit_cancel_check) — the campaign's first free-fn pack (all
                        prior GSM8K cells were u32 state cells)
wave 3i   142 cells   + M1 pack 4/5: verifier/ranker (sum_equals, diff_equals,
                        product_equals_u32, quotient_equals_exact_u32) — reverse-
                        equation-satisfaction checks that always return a verdict,
                        never escalate; answer_eq/multi-plan-agreement/tie-break
                        needed no new code (see the pack note below)
wave 3j   145 cells   + stateful/RNG, first slice (lcg_next, xorshift16,
                        counter_step) — bounded_rand deferred (exact duplicate of
                        safe_mod)
wave 3k   149 cells   + signed deltas, first slice (sign_i16, abs_i16, clamp_i16,
                        apply_delta_clamped) — the library's first i16 cells; also
                        widened DEFAULT_PROBES (fingerprint.rs) with a negative-i16
                        value, which fixed the long-standing snap_down false
                        positive as a side effect (see the pack note below)
wave 3l   153 cells   + scoring/choice, second slice (weighted_sum2, weighted_sum3,
                        choose_best3, is_clear_winner) — generalizes weighted_sum's
                        fixed weights to caller-supplied ones (a genuine u32
                        overflow is now reachable and escalates); choose_best3
                        picks by score where value != score, unlike argmax3
now       163 cells   + the GSM8K math campaign, M1 pack 5/5 (final): fractions
                        (frac_reduce/add/sub/mul/div/cmp/eq, is_integer,
                        frac_to_mixed, ratio_split2) — M0 (u32-across-a-call-
                        boundary) landed as Tier 2 (one u32 param per call), so
                        each cell inlines its own GCD-reduction loop rather than
                        sharing a two-u32-param gcd_u32 helper; M1 complete
next      ~200+        + cosine_score_approx
```

All five originally-planned wave-3 packs (calendrical/checksum, fixed-point, agentic
runtime, running statistics, spatial/grid) landed a first slice; each deferred its harder
items (see the per-pack notes below and `docs/cell-index.md`'s "planned" section). The
Phase 2.3 pilot batch (packing/BCD + vector) exercised the author→verify→admit loop
end-to-end for the first time — see the "Phase 2.3" section below for what it found.

**The GSM8K math campaign (`docs/math-campaign-spec.md`), M1 pack 1/5: checked/exact
arithmetic.** `mul_u16_u16_to_u32`, `add_checked_u32`, `sub_checked_u32`, `div_exact_u32`,
`div_floor_u32`, `div_ceil_u32`, `mod_u32`, `fits_u16` — all `u32` state cells (a free-fn
entry still can't take/return `u32`, the same constraint the calendrical/checksum pack
found). "Checked" here means the escalation contract (`halt(0xFF05)`, `needs_wider_math`,
Phase 3.2), not the saturating/sentinel convention `add_sat`/`safe_div` already use — a
caller can tell "this genuinely didn't fit" apart from an ordinary result, which a
saturated or sentinel value can't. `div_ceil_u32` computes via `a/b` plus a remainder
check rather than `(a+b-1)/b`, so a large `a` can't overflow the *overflow guard itself* —
"checked" was the whole point. Confirmed by testing directly: **M0's u32-across-a-
call-boundary prerequisite is still unbuilt**, even after `Cond32` (u32 comparisons)
landed from a parallel session mid-pack — a local helper function can't yet take or
return `u32`, so the fraction pack (which wants a shared `gcd_u32` reducer) stays blocked
until that lands.

**M1 pack 2/5: money/basis-points.** `bps_of`, `increase_by_bps`, `decrease_by_bps`,
`original_before_bps_increase`, `original_before_bps_decrease`, `cents_mul_qty` — basis
points (1% = 100 bps), never float percentages, so `value * bps / 10000` stays exact
integer math. All six escalate (`halt(0xFF05)`) on multiply overflow via the same
"divide back and compare" trick as the checked-arithmetic pack
(`let p = a.wrapping_mul(b); if a != 0 && p / a != b { halt(...) }`) rather than needing a
u64. Checked `docs/cell-index.md` before authoring and dropped several near-duplicates of
the checked-arithmetic pack from the original spec's money list (`cents_add`/`cents_sub`
duplicate `add_checked_u32`/`sub_checked_u32`; `cents_div_qty`/`unit_price_cents`
duplicate `div_floor_u32`; `price_total`/`change_due` duplicate `cents_mul_qty`/
`sub_checked_u32`), and consolidated `tax_bps`/`tip_bps`/`markup_bps` — identical formula
under different names — into one canonical `increase_by_bps` cell instead of shipping
three copies. `cents_mul_qty` is kept distinct from `mul_u16_u16_to_u32` even though both
multiply: `mul_u16_u16_to_u32` takes two `u16`s and always fits `u32` exactly, while
`cents_mul_qty`'s `unit_cents` is already a wide `u32` and can genuinely overflow — a
real behavioural difference, not a renamed duplicate. "Cents" names the minor unit of any
decimal currency (cents, pence, kopecks, ...), not USD specifically — kept over a more
generic `minor_units` name because it's the de facto term in decimal-money code regardless
of actual currency (Stripe et al. use `amount_in_cents` the same way).

**M1 pack 3/5: units.** `same_unit_check`, `unit_mul`, `unit_div`, `unit_cancel_check` —
the campaign's first *free-fn* pack (every prior GSM8K cell was a `u32` state cell; these
are plain arity-2 `u16` functions, so they go through the admission gate's fingerprint
check for real rather than being exempted as a state cell — confirmed no collisions).
Dimension codes: `0=count, 1=money, 2=time, 3=distance, 4=area, 5=volume,
6=rate_money_per_count, 7=rate_distance_per_time` — a fixed small enum with hand-written
pairwise composition rules (`count*money=money`, `distance*distance=area`,
`money/count=rate_money_per_count`, same-unit-divided-by-itself always cancels to
`count`, ...), not a general symbolic exponent-vector algebra; unmodeled pairs escalate
(`halt(0xFF06)`, `out_of_domain` — genuinely a *different* escalation reason than the
arithmetic packs' `needs_wider_math`, since a unit mismatch isn't a wide-math problem).
`same_unit_check` returns the shared dimension code on a match (useful for tagging an
addition's result) rather than a bare boolean, so it doubles as the compatibility check
for both `+` and `-` (same requirement) instead of shipping a second, redundant
`unit_add_check`. `unit_cancel_check` is `unit_div`'s table restated as a non-escalating
predicate, for a caller (e.g. a future plan verifier) that wants to try several candidate
unit pairs without committing to a halt. Landed 4 of the spec's `~10` estimate — the
smaller set is what's concretely load-bearing for the one worked example in
`docs/math-campaign-spec.md` (`count * rate_money_per_count = money`, round-tripped
through `unit_div(money, count)` first); a full exponent-vector unit algebra is deferred
until real GSM8K plans demand it.

**M1 pack 4/5: verifier/ranker.** `sum_equals`, `diff_equals`, `product_equals_u32`,
`quotient_equals_exact_u32` — each re-derives one side of a candidate plan's claimed
equation and returns a plain `0`/`1` verdict, **never escalating**: a verifier's whole job
is to answer, so a genuine overflow (`product_equals_u32`) or a divide-by-zero
(`quotient_equals_exact_u32`) is just a `0` (the claim doesn't hold), not a
`halt(0xFF05)` hand-off — a deliberately different contract from the arithmetic packs,
which compute a value and escalate when they can't. `sum_equals` widens its addition to a
local `u32` internally (no function boundary, so M0 doesn't block it) so a genuine `u16`
overflow can't wrap into a false match. Checking `docs/cell-index.md` and the spec's own
`~20`-cell estimate before authoring found most of it already covered by existing cells,
so nothing new was built for: `answer_eq` (an exact alias of the predicates pack's `eq` —
no new code), multi-plan agreement and tie-breaks (`majority3`/`mode3`, ranking-stats,
already do "do at least two of three plans agree" and "which value repeats"), and range
constraints (`range_check`, validation pack). `answer_in_options` (checking an answer
against an arbitrary-length option list) is deferred — GSM8K is free-response, not
multiple-choice, so the motivation is thin, and a real implementation would need an array
state field this session hasn't risked yet.

**Stateful/RNG, first slice.** `lcg_next` (seed = seed \* 1664525 + 1013904223 mod 2^32,
Numerical Recipes constants, top 16 bits returned), `xorshift16` (x ^= x<<7; x ^= x>>9;
x ^= x<<8 — a distinct recurrence from lcg_next, no multiply), `counter_step` (a modular
increment-and-wrap counter for round-robin dispatch). `bounded_rand` — the fourth item on
`library-growth.md`'s own next-waves list — was **not** built: `raw % bound` (0 on
`bound == 0`) is an exact behavioural duplicate of the already-shipped `safe_mod`, the
same reasoning that folded `wrap` into `safe_mod` as an alias earlier. Verifying these
cells surfaced an important, easy-to-miss point about `StateCell`/`Runner::run`'s
contract: **a state cell does not persist memory across separate `.run()` calls** —
`Runner::run`'s own doc says "memory the previous run touched is zeroed first, so
repeated runs start from the same clean state." A naive test that calls `.run()` twice on
the same instance expecting `self.seed`'s prior mutation to carry over silently resets to
0 — not a compiler bug (confirmed by reproducing the identical pattern against the
already-shipped `streak_step`, whose own host-oracle test already re-`set`s the carried
field from the previous `.get()` before every call, per `cell80/tests/library.rs`). Every
"step" cell in the library (this pack included) relies on the *caller* threading the
carried field through explicitly, matching the real host/agent loop's own calling
convention (`run_state`/`run_state_fast` take the full current field set on every call) —
documented here since it cost real debugging time to pin down and will bite the next
stateful pack's author again if it isn't written down.

**Signed deltas, first slice.** `sign_i16` (-1/0/1), `abs_i16` (magnitude as `u16`,
correctly handling `i16::MIN`'s 32768 which doesn't fit back in `i16`), `clamp_i16` (the
signed counterpart of `clamp`), `apply_delta_clamped` (apply a signed delta to an unsigned
value, clamped to `[0, cap]` — a health/resource/score adjustment that can't go negative
or over a cap; the "risk delta" use case). The library's first `i16` cells, now that the
dialect supports it (confirmed directly: `i16` params, returns, comparisons, unary
negation, and `as`-casts between `i16`/`u16` all work). `lerp_i16` (interpolating between
two signed values) is deferred — signed multiply/divide's rounding direction and overflow
safety haven't been worked out.

Authoring `sign_i16` surfaced a second real fingerprint-probe gap, the same class as the
`luhn_check`/`is_zero` case from Wave 3: every value in `DEFAULT_PROBES` is non-negative
when reinterpreted as `i16` (the largest, `1230`, is still far short of the `i16` sign
bit), so `sign_i16`'s negative branch never fired on the bank alone — it degenerated to
`nonzero` (agreement 1.00, both only ever emitting `0`/`1`). Fixed the same honest way:
widened `DEFAULT_PROBES` (`cell80/src/fingerprint.rs`) with `[65531, 3]` (`-5` as an
`i16` bit pattern), rather than touching `sign_i16`. That widening had a welcome side
effect: it also separated the long-documented `snap_down`/`round_to_multiple` false
positive (they'd agreed on the whole ten-probe bank since Wave 3 but diverge at e.g.
`x=8, step=5`) — the twelfth probe happens to be one of the inputs where they disagree
(`snap_down(65531, 3) = 65529` vs `round_to_multiple(65531, 3) = 65532`), so the gate is
now fully clean: 149 admitted, 0 refused, not the long-standing 1.

**Scoring/choice, second slice.** `weighted_sum2`/`weighted_sum3` generalize
`weighted_sum`/`weighted_sum_wide`'s fixed weights (1, 2, 3) to caller-supplied ones —
`score_2factor` from the original next-waves list is the same formula as `weighted_sum2`
under a different name, so its vocabulary was folded into `weighted_sum2`'s tags rather
than shipping a duplicate. Unlike their fixed-small-weight siblings (whose additions can
never overflow `u32`), arbitrary weights genuinely can, so both escalate
(`halt(0xFF05)`) on a real `u32` overflow instead of silently wrapping — verified directly
(`a=b=wa=wb=65535` overflows; `a=1000,wa=1000,b=1,wb=1` doesn't overflow `u32` but does
saturate the `u16` return, exactly like `weighted_sum_wide`'s existing convention).
`choose_best3` picks the *value* of whichever of three (value, score) pairs has the
highest score — genuinely different from `argmax3`, which only works when the value you
want back **is** the value being compared; ties go to the lowest index, matching
`argmax3`'s own convention. `is_clear_winner` checks whether a margin is decisive (top −
second ≥ margin) rather than just picking a winner, catching the "basically a tie" case
`argmax3`/`choose_best3` can't express. `tie_break_*` from the original list was too
under-specified to build — every existing ranking cell already bakes in its own concrete
tie-break rule (lowest index for `argmax3`/`choose_best3`, "value that repeats" for
`mode3`), so a separate abstract tie-break cell has no clear GSM8K/agent-facing use case
yet.

**Fractions — GSM8K math campaign, M1 pack 5/5 (final).** `frac_reduce`, `frac_add`,
`frac_sub`, `frac_mul`, `frac_div`, `frac_cmp`, `frac_eq`, `is_integer`, `frac_to_mixed`,
`ratio_split2` — every fraction is a `(u32, u32)` numerator/denominator pair; every op
reduces the result to lowest terms via an inline Euclidean GCD. `frac_cmp`/`frac_eq`
cross-multiply (`na*db` vs `nb*da`) instead of reducing first, so they work correctly on
unreduced-but-equivalent fractions (`1/2` vs `2/4`) without extra steps. `frac_sub`
escalates (`halt(0xFF05)`, `needs_wider_math`) if the result would be negative — an
unsigned fraction can't represent it, the same convention `sub_checked_u32` already
uses for "this would go negative." Every op escalates (`halt(0xFF06)`, `out_of_domain`)
on a zero denominator, and `frac_div` additionally on a zero-numerator divisor
(dividing by a zero fraction). `frac_floor`/`frac_ceil` from the spec's own list were
never built — checking `docs/cell-index.md` before authoring found they'd be exact
duplicates of the already-shipped `div_floor_u32`/`div_ceil_u32`.

M0 (u32-across-a-call-boundary) landed from a parallel session mid-session, as **Tier
2**: at most one `u32` parameter per call (and it must be the first), a `u32` return, and
nothing more — confirmed directly, and confirmed this does *not* fully unblock the
originally-envisioned design. A shared `gcd_u32(a: u32, b: u32) -> u32` reducer (what the
whole session assumed "M0 landing" meant) still can't be called: **two** `u32` params in
one call still isn't supported — `docs/10-dialect-semantics.md` calls this out itself as
"the honest residual: a two-wide-param kernel... still doesn't fit three registers —
widen inside, or pass through state." The actual unblock is narrower and was available
all along: a `while` loop over `u32` **local variables**, entirely inside one cell's own
`run` method, was never gated by the call-boundary limitation at all (only actual
function *calls* passing `u32` arguments are) — confirmed directly with a standalone
inline-GCD test before writing any real cell. So every fraction cell duplicates its own
short Euclidean-GCD loop rather than sharing one; a genuine `gcd_u32` helper (for the
next pack that wants one) still needs a further compiler feature, not this one.

Cells are also **modular** now: a shared kernel prelude (`gcd`, `imin`, `imax`, `iabs_diff`,
`isqrt`, `clamp_to`) is appended to every cell and dead-code-eliminated, so `lcm` calls `gcd`
and `chebyshev` calls `iabs_diff`/`imax` instead of re-implementing them — and a cell that uses
no kernel stays byte-identical to having no prelude.

Big is the point — but **big and distinct**, not big and padded. Every cell earns its place by
being a *different behaviour*, not a renamed one.

## What a good cell is

> A tiny, deterministic utility an agent needs often but shouldn't spend tokens re-deriving.

```
small · deterministic · easy to test · easy to describe · useful in many workflows
cheap enough to run constantly · a distinct behaviour · part of a confusable family
```

## Two rules that keep a *large* library strong

A library that just accumulates functions rots in two ways — these rules prevent it:

1. **No behavioural duplicates — aliases live in metadata, not in code.** `time_until(now,
   deadline)` is `sub_sat`; `deadline_missed` is `is_ge`; inclusive `between` is `range_check`.
   Don't ship a second cell with the same behaviour — add the alias as a **tag/summary** on the
   existing one so search still finds it. Duplicates *hurt* retrieval (two right answers = no
   signal) and bloat the shelf without adding capability. **Enforced, not just requested:** the
   Phase 2.2 admission gate (`cell80 index --gate`, `cell80/src/admission.rs`) found four
   cells that had shipped as exact duplicates without anyone noticing — `argmin2` ≡ `is_gt`,
   `argmax2` ≡ `is_lt`, `quantize` ≡ `safe_div`, `wrap` ≡ `safe_mod`, the identical formula
   under a different name for every `u16` input. All four were removed and their vocabulary
   merged into the surviving cell's tags.
2. **Grow in confusable families, and pay the eval tax per cell.** Retrieval only gets *teeth*
   from 3-4+ cells per family that collide in text but differ in behaviour; composition needs
   predicates + transforms that chain. A new cell ships with its eval pressure or it's just
   inventory. See the contribution rule.

So "a large number of cells" and "good evals" pull the *same* direction: more distinct
confusable cells = a bigger shelf *and* a harder, more honest retrieval benchmark.

## Principles (what makes a cell worth adding)

- **Fits the integer envelope** (`u8`/`u16`/`u32`/`i16`, no float/string/syscall, bounded
  cycles). The compile error *is* the "this belongs in host code" signal. `i16` has landed
  (signed compare/divide/`>>`), so signed deltas are in-envelope; the unsigned abs-via-a-branch
  idiom (`abs_diff`/`manhattan`) remains fine where a cell doesn't need negatives.
- **≤ 3 args, or a state cell.** The calling convention takes 3 args (`HL`/`DE`/`BC`); a cell
  that needs more (4-point distance, multi-weight scoring) is a **state cell** (a `struct` with
  named fields + `fn run(&mut self)`), like `manhattan`/`chebyshev`.
- **Small and pure** — tens of bytes of behaviour, deterministic, cycle-honest.
- **Composes** — produces/consumes values others use; include **boolean predicates** (`-> 0/1`).

### What the compiler gives you (author cells clean)

- **Comparisons are values:** `fn run(a: u16, b: u16) -> u16 { (a < b) as u16 }`. All six.
- **`&&` / `||`** (short-circuit): `((lo < x) && (x < hi)) as u16` — and a short-circuit guard
  is the right way to keep a loop in-budget, e.g. `while r < 255 && (r+1)*(r+1) <= n { … }`.
- **Runtime bit shifts:** `x << bit` / `x >> bit` with a *variable* amount (a shift ≥ 16
  saturates a `u16` to `0`) — bit/rotate/encoding cells are one-liners.
- **Multi-function cells:** a cell may define helpers and call them; the entry is `run`
  (e.g. `lcm` calling a local `gcd`).

### Standardise these semantics

- **Predicate convention:** `false = 0`, `true = 1` (built on `bool as u16`).
- **Divide/remainder by zero:** unguarded `/` and `%` **halt the cell**
  (`Halt::DivByZero`, the Phase 0.3 default) rather than returning a value — **guard
  explicitly** when zero is a valid input (`if b != 0 { a / b } else { 0 }`); `safe_div`/
  `safe_mod` are canonical.
- **`u16` overflow is silent** (wraps); saturating cells (`add_sat`, `mul_sat`, `sum3`, …) cap
  at `65535`; percent/scale/`euclid_sq`/`triangular` assume their product fits `u16` (beyond is
  the host-code signal).

## The contribution rule (every new-cell PR)

```
1. cell80/cells/<name>.rs                       — header (//! summary, //! tags:) + fn/struct
2. cell-eval/datasets/retrieval.jsonl           — a direct row that ranks the cell #1
                                                   (verify with `cell80 search`), + paraphrase
3. composition or adoption task (if user-facing) — composition_tasks.jsonl / tasks.jsonl
4. cell80/tests/library.rs                       — edge-case rows (the host oracle)
5. docs/cell-index.md                            — regenerate (command at the top of the file)
```

Steps 1-2 are enforced, not just requested: `cell80 index cell80/cells --gate
cell-eval/datasets/retrieval.jsonl` (the Phase 2.2 admission gate, `cell80/src/admission.rs`)
refuses a candidate that's behaviourally identical to an already-shipped cell (alias it in
metadata instead) or that carries no retrieval rows to survive. Run it before opening a PR.

## Packs (organise discovery by family via tags)

The loader reads a flat `cell80/cells/`, so a "pack" is a **tag**, not a directory. Build them
out broadly:

```
math-core      bounds        percent       ranking-stats   number-theory   distance
bitops         bit-encoding  hashing       packing         time            budget
validation     vector        decimal       random/stateful scoring/choice  conversion
```

### Landed (163 cells)

See **[`docs/cell-index.md`](cell-index.md)** for the full, generated, per-pack list — not
duplicated here, so there's exactly one place this can go stale (and it's checked against
the real library every time it's regenerated).

**Calendrical / checksum, first slice (wave 3): `is_leap_year`, `days_in_month`,
`day_of_week`, `luhn_check`.** A real constraint surfaced authoring this pack: a free-fn
cell's calling convention is 16-bit registers, so **`u32` can only exist as a state field,
not a call param or return type** — a classic multi-digit checksum (Luhn over a real
13-19-digit card number, ISBN-13, IBAN mod-97) needs far more digits than even a `u32` state
field holds. `luhn_check` is scoped to a `u16` input (≤ 5 decimal digits, documented via
`//! limits:`) to stay a plain free function; ISBN-10/13, IBAN mod-97, and UPC are deferred
until either a state-cell version (carrying digits as array/state fields) or wider host-side
preprocessing is worth the design cost.

**Q8.8 fixed-point, first slice (wave 3): `q_mul`, `q_div`, `q_lerp`.** Sidesteps the same
`u32`-in-a-free-fn constraint by keeping params/return as `u16` and widening only as a
*local* (`a as u32 * b as u32`, `>> 8u32`) — the pattern any Q8.8 free function should
follow. `q_lerp` also serves as an EMA step (`q_lerp(prev, sample, alpha)`) — deliberately
*not* shipped as a second `q_ema` cell, since the formula is identical; the admission gate
would refuse it anyway. `q_sqrt` and piecewise `q_sigmoid`/`q_tanh` are still open.

**Agentic runtime primitives, first slice (wave 3): `token_bucket_step`, `backoff_next`,
`circuit_breaker_step`, `debounce_step`, `hysteresis`.** All genuinely need state (each
depends on outcomes from prior calls, not just this call's arguments), unlike the other
"time/budget" names already flagged in Next waves below — `used_percent`/`fits_budget`/
`cooldown_remaining` turned out to be aliases of `percent`/`is_le`/`sub_sat` respectively
and were never built, exactly the kind of check `docs/cell-index.md` is for. `backoff_next`
guards against a real overflow: doubling `current` directly can wrap past `u16::MAX` before
the cap check runs, so it compares against `cap / 2` first and only multiplies when doubling
is provably safe. `rate_window_update` is still open.

**Running statistics, first slice (wave 3): `running_min_max_step`, `streak_step`,
`accumulate_step`.** Deliberately doesn't reach for Welford's algorithm (which needs care in
fixed point) or a histogram (which needs array state fields, not yet exercised by any landed
cell) — instead `accumulate_step` keeps a running sum + count and composes with the
already-landed `safe_div` for the mean, rather than shipping a monolithic "running mean"
cell that would just re-implement `safe_div` internally. A fixed-point running variance and
percentile-from-histogram are still open, gated on that array-state-field question.

**Spatial / grid, first slice (wave 3): `grid_index`, `point_in_rect`, `aabb_intersect`.**
`grid_index` is a plain arity-3 free function; the other two are state cells purely for arg
count (6 and 8 named fields respectively), not width. Both containment checks are
half-open — edge-touching does not count as inside/overlapping, verified by hand for both.
Morton encode/decode were deliberately not attempted this slice: encoding a full `u16` x/y
pair needs a 32-bit interleaved result, so — like the calendrical/checksum pack's
discovery — it would need a `u32` state field, and the bit-interleaving loop itself
(computed shift amounts on a wide accumulator) hasn't been risked yet. A Bresenham line
stepper is also still open.

**Packing/BCD + vector, the Phase 2.3 pilot batch: `pack_u8`, `pack_nibbles`, `bcd_encode`,
`bcd_decode`, `dot2`, `norm2_sq`.** The first real run of the author→verify→admit loop
(below) — and it immediately earned its keep: checking `docs/cell-index.md` before
authoring found `unpack_lo`/`unpack_hi` would be exact duplicates of the already-landed
`low_byte`/`high_byte`, so they were never written at all (a real duplicate caught before
the gate ever had to refuse one). `cosine_score_approx` was scoped out for a different
reason — an honest one: exact fixed-point cosine similarity needs `sqrt(norm_a * norm_b)`
without overflowing a `u16` return, and that hasn't been worked out yet. `dot2` is a state
cell purely for arg count (4 fields: two 2D vectors), not width.

### Next waves (prioritized — keep them distinct)

- **scoring / choice — second slice landed** (see the pack note above): `weighted_sum2`,
  `weighted_sum3`, `choose_best3`, `is_clear_winner`. `score_2factor` folded into
  `weighted_sum2`'s tags (identical formula); `choose_best4` and `tie_break_*` still open
  (the former is a straightforward generalization when a 4th candidate is actually
  needed; the latter is under-specified — no concrete use case beyond what
  `argmax3`/`choose_best3`/`mode3` already bake in).
- **vector, still open**: `cosine_score_approx` (see above — blocked on an overflow-safe
  fixed-point sqrt-of-a-product).
- **stateful / RNG — first slice landed** (see the pack note above): `lcg_next`,
  `xorshift16`, `counter_step`. `bounded_rand` was checked against `docs/cell-index.md`
  and found to be an exact duplicate of `safe_mod` — not built. (`ema_update`/
  `moving_avg_update` — skip, `q_lerp` already is this: `q_lerp(prev, sample, alpha)` is
  one EMA step.)
- **time / budget** — checked against `docs/cell-index.md` before building: `used_percent` is
  `percent`, `fits_budget` is `is_le`, `cooldown_remaining` is `sub_sat`, `time_until` is
  `sub_sat`, `deadline_missed` is `is_ge` — all aliases, none of these get built as new cells.
- **signed deltas — first slice landed** (see the pack note above): `sign_i16`, `abs_i16`,
  `clamp_i16`, `apply_delta_clamped`. `lerp_i16` still open (signed multiply/divide
  rounding direction and overflow safety not yet worked out).

## Phase 2.3 — growing toward ~1,000 cells

Wave 3's 20 cells were each authored, then hand-traced against known reference values
(Zeller's congruence checked against 2000-01-01 and 2024-01-01, state-machine transitions
walked by hand) before being written to source. That doesn't scale to ~886 more cells.
The **author → verify → admit** loop below keeps the same rigor but makes the verify step
mechanical instead of hand-traced, so it can run at batch size:

1. **Spec** — one line per candidate: pack, id, intended behaviour, arity hint (free-fn
   ≤3 args vs state cell — remember `u32` can only be a state field, never a free-fn
   call param/return, the constraint the calendrical/checksum pack found). Pull specs from
   this file's "Next waves" list first — already-scoped, not invented fresh.
2. **Author** — draft the cell source + 2-3 retrieval rows (direct/paraphrase) + 2-3
   proposed host-oracle `(args, expected)` triples, using the dialect gotchas already
   learned the hard way this session:
   - `self.field = if ... else ...` **directly** is not supported — bind to a `let` first,
     then assign the field (hit twice: `backoff_next`, `accumulate_step`).
   - `!` is supported logical-not on the 0/1 boolean convention (not bitwise).
   - value-`match` needs a `_` arm; range/or-patterns lower to if-chains.
   - `.saturating_add/sub/mul` work on `u8`/`u16` now (means what it means in host Rust).
3. **Verify — mechanical, not agent-judgment.** This is the step that replaces
   hand-tracing:
   - Compile the candidate; a compile error gets one repair attempt (same shape as the
     `cell-eval repair` eval), else discard.
   - **Actually run** the proposed oracle rows against the real compiled cell and require
     the output to match the claimed expected value — a mismatch means either the cell or
     the claimed expectation is wrong, so it's discarded/flagged either way, never trusted
     on say-so alone.
   - Run `cell80 index --gate` (Phase 2.2) against a scratch copy of the library +
     retrieval dataset with the candidate included — a refusal gets the same treatment as
     every real Wave 3 refusal (alias if it's a true duplicate, discard if not, never
     silently forced through).
4. **Admit** — only candidates passing all three land for real: source file, golden
   regenerated, host-oracle rows, retrieval rows, `docs/cell-index.md` regenerated, a
   `cell-eval curve` checkpoint after the batch (not per-cell) recorded in
   `cell-eval/baselines/library-scale-curve.json`.

**The kill-gate.** Mirroring the escalation ladder's own standing rule (θ calibrated
against a 0.75 precision-on-answered floor, `docs/escalation-ladder.md`): if a checkpoint's
retrieval precision on the paraphrase or adversarial split drops meaningfully from
checkpoint 1's baseline (114 cells: direct 0.94 / paraphrase 0.42 / adversarial 0.39;
`cell-eval/baselines/library-scale-curve.json`), **pause cell growth** and prioritize
discovery/retrieval work instead of adding more cells. A 1,000-cell library nobody can
search is worse than the 114-cell one that can be searched today. Checkpoint 5 (units
pack, 138 cells) dipped *under* the adversarial baseline (0.38 vs 0.39), traced to two
pre-existing confusable pairs re-ranking from a corpus-wide TF-IDF weight shift, not a
units-pack collision; checkpoint 6 (verifier/ranker, 142 cells) recovered to 0.41,
confirming it was noise, not a trend. The kill-gate rule itself only names paraphrase/
adversarial, but **direct P@1 has now declined for three checkpoints running**:
checkpoint 8 (signed-deltas, 0.9363 — `abs_i16`/`abs_diff` vocabulary overlap),
checkpoint 9 (scoring/choice, 0.9255 — `weighted_sum2`/`weighted_sum3` losing their own
direct query to the pre-existing `weighted_sum`), and checkpoint 10 (fractions, 0.9181).

**Checkpoint 10 is the first to actually meet the kill-gate's literal condition.**
Paraphrase dropped to 0.4016 — measurably below checkpoint 1's 0.4247 baseline (a ~2.3
point drop, an order of magnitude past the ~0.005 deltas earlier checkpoints called
"noise"). Of 6 flipped retrieval cases, 3 losses trace directly to the fractions pack:
`frac_sub`/`frac_cmp`/`frac_add`'s summaries lead with generic arithmetic verbs
("subtract," "compare," "add") that now outrank `sub_sat`, `eq`, and `same_unit_check` on
those cells' own established queries. This was raised explicitly to the user as a
decision point rather than logged and continued past silently — see the session record
for the resolution. The underlying lesson: a *math-themed* pack that legitimately reuses
common arithmetic vocabulary is more prone to this than a narrowly-scoped pack (units,
RNG) was: `docs/cell-index.md` catches true behavioural duplicates before authoring, but
it doesn't catch "this wording will out-rank an existing cell on a shared verb" —
that's what the retrieval curve is for.

**The response: paused growth, fixed the live index (checkpoint 11), not just noted it.**
Per the user's explicit call ("pause growth, fix retrieval first") over the alternatives
(a narrow wording patch, or continuing regardless). Diagnosed all 3 losses directly: each
was a simple 2-arg free-fn (`sub_sat`, `eq`, `same_unit_check`) losing to a 4-6-field
fraction state cell sharing one core verb — a real same-verb-different-*shape* confound.
Fix: `TfidfIndex::search` (the live path everything routes through — `CellHost`, the CLI,
MCP, and `cell-eval`'s own `lib.search`) now breaks near-ties toward the structurally
simpler cell (a free-fn's param count or a state cell's field count), the same
length-normalisation instinct BM25 applies to document length, applied to shape instead.
Swept γ 0.0–0.3 on the full 327-query set first; 0.05 was the best overall point and the
only one positive on every split but direct. Deliberately scoped to `search`'s *ranking
order* only, never `scored`'s exposed cosine magnitude — that value feeds `cell-eval`'s
tiered-retrieval margin gate, calibrated against raw tf-idf cosine (a per-embedder θ,
`cell-eval/src/cell_eval/tiers.py`); rescaling it would have silently drifted an
already-tuned threshold without re-running that calibration.

Result, honestly: **partial, not complete, recovery.** Adversarial jumped well above
checkpoint 1's baseline (0.47 vs 0.39). Paraphrase recovered about a third of checkpoint
10's drop (0.41 vs 0.40, still under the 0.4247 baseline). Direct ticked down another 0.6
points, continuing a now-four-checkpoint decline — though that decline's *rate* has been
slowing each checkpoint, consistent with a growing corpus's natural denominator effect
rather than something this specific fix caused. This was reported to the user as
validated-but-partial progress, not declared a full fix — see
`cell-eval/baselines/README.md`'s checkpoint 11 entry for the complete numbers.

**Checkpoint 12 fully resolves it.** Asked to keep pushing, a fresh diagnostic (dumping
every current miss across all three splits, not just the fractions-specific ones) found
a different, more widespread root cause than checkpoint 11's: several of the library's
*oldest* cells (`gcd`, `min`, `max`, `chebyshev`, `pack_u8`, `same_unit_check`) were
authored early in the project with much sparser tags than the richer, synonym-heavy
convention every later pack settled into — so newer, better-tagged siblings (`gcd3`,
`min3`, `max3`, `manhattan`) routinely out-ranked them on their *own* queries. Six
targeted tag/wording fixes, each verified individually against
`examples/retrieval_compare` before and after (a seventh — adding vocabulary to
`abs_diff` — measurably regressed adversarial and was reverted; not every added synonym
helps, verify each one). Result: **paraphrase (0.459) and adversarial (0.50) are both now
above checkpoint 1's baseline for the first time since checkpoint 7**, direct (0.9181)
recovered fully to checkpoint 10's pre-fix level (ending the four-checkpoint decline),
and overall P@1 (0.7034) now exceeds checkpoint 1's own overall (0.6974) despite the
library growing 114→163 cells. The kill-gate concern is resolved, not just mitigated —
see `cell-eval/baselines/README.md`'s checkpoint 12 entry for the full breakdown and the
six specific fixes.

**Known gaps, not yet blocking, worth tracking as the library scales further:**
- The admission gate's fingerprint check only covers arity-≤2 free-fn cells (state cells
  and arity-3 cells are exempt — `cell80/src/admission.rs`'s own doc explains why:
  `Fingerprint`'s probe bank only drives two scalar registers). Generalizing it to typed
  state and arity-3+ is real future work, not done here.
- The admission report isn't wired into CI yet — worth doing whenever CI config is next
  touched, so a duplicate can't land even by accident.
- Keep synthesis (`cell80/src/synth.rs`) gated and narrow, per
  `docs/escalation-ladder.md`'s own "do-not-build" list — it's for outcome-specified tasks
  over a known op family, not a general cell-generation path.

## Mine the ecosystem first

`chuk-math` / `chuk-mcp-math` / `chuk-synthetic-data` likely already hold integer kernels worth
porting straight in — cheaper than authoring from scratch, and it ties the loop.

## After authoring: re-run the evals

Each new **family** is a retrieval test case; each **predicate + transform** pair a composition
test case. Re-run `cell-eval retrieval` / `composition`. Expect direct P@1 to stay strong while
paraphrase stays in coin-flip territory as the library grows (8 → 98 cells: direct ≈ 0.92,
paraphrase 0.53 → 0.45) — that gap is the standing case for the **type-led / capability index** (rank by
typed signature + a `kind = predicate | transform | …` first, embeddings as the tiebreaker). A
big confusable library is precisely what makes that benchmark trustworthy.
