# Growing the cell library — toward a large prebuilt collection

*The goal is a **big, growing library of prebuilt cells** — hundreds of small, distinct,
deterministic integer utilities an agent can retrieve, run, and compose, organized into
**packs**. cell80's whole pitch is "millions of tiny tools, retrieved" — so the library should
be **broad**: the more genuinely-distinct behaviours sit on the shelf, the more an agent finds
one instead of writing code. This guide is how to grow it well.*

## The shape we're building toward

```
wave 1 ✓   59 cells   predicates · safe arithmetic · bounds · percent · ranking · bit/mask
wave 2 ✓   98 cells   + number theory · distance · bit/encoding · hashing · stats · conversion
next      ~200+        + packing/BCD · scoring/choice · vector · time/budget · stateful/RNG
```

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
   signal) and bloat the shelf without adding capability.
2. **Grow in confusable families, and pay the eval tax per cell.** Retrieval only gets *teeth*
   from 3-4+ cells per family that collide in text but differ in behaviour; composition needs
   predicates + transforms that chain. A new cell ships with its eval pressure or it's just
   inventory. See the contribution rule.

So "a large number of cells" and "good evals" pull the *same* direction: more distinct
confusable cells = a bigger shelf *and* a harder, more honest retrieval benchmark.

## Principles (what makes a cell worth adding)

- **Fits the integer envelope** (`u8`/`u16`/`u32`, no float/string/syscall, bounded cycles).
  The compile error *is* the "this belongs in host code" signal. Keep cells
  **unsigned-friendly** until `i16` lands (abs via a branch, like `abs_diff`/`manhattan`).
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
- **Divide/remainder by zero:** the cell returns a sentinel — **guard explicitly**
  (`if b != 0 { a / b } else { 0 }`); `safe_div`/`safe_mod` are canonical.
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
```

## Packs (organise discovery by family via tags)

The loader reads a flat `cell80/cells/`, so a "pack" is a **tag**, not a directory. Build them
out broadly:

```
math-core      bounds        percent       ranking-stats   number-theory   distance
bitops         bit-encoding  hashing       packing         time            budget
validation     vector        decimal       random/stateful scoring/choice  conversion
```

### Landed (98 cells)

```
predicates     eq neq is_lt is_le is_gt is_ge is_zero nonzero is_even is_odd
safe-arith     add_sat sub_sat mul_sat safe_div safe_mod ceil_div avg2 square
bounds         between_exclusive wrap normalize_0_100 snap_down snap_up round_to_multiple
percent        percent permille ratio_255 scale_percent increase_percent discount_percent within_percent
ranking-stats  min3 max3 median3 argmax2 argmin2 argmax3 argmin3 sum3 mean3 range3 mode3 majority3 midrange3
bit/mask       popcount parity bit_is_set set_bit clear_bit toggle_bit mask_has_all mask_has_any mask_union mask_intersection mask_xor
number-theory  lcm gcd3 divides is_coprime is_prime is_square isqrt digit_sum num_digits factor_count triangular next_pow2 is_pow2 pow_small cube_sat pow_mod
distance       chebyshev euclid_sq          (state-cell siblings of manhattan)
bit-encoding   low_byte high_byte swap_bytes rotl16 rotr16 reverse_bits leading_zeros trailing_zeros bit_length
hashing        hash_pair fnv1a_step crc8_step mix16
bucket/convert bucket3 quantize percent_to_byte byte_to_percent
```

### Next waves (prioritized — keep them distinct)

- **packing / BCD**: `pack_u8`/`unpack_lo`/`unpack_hi`, `pack_nibbles`, `bcd_encode`/`bcd_decode`.
- **scoring / choice** (mostly state cells — need > 3 args): `weighted_sum2/3`, `score_2factor`,
  `choose_best3/4`, `is_clear_winner`, `tie_break_*`.
- **vector** (state cells): `dot2`, `norm2_sq`, `cosine_score_approx`.
- **stateful / RNG** (struct state): `lcg_next`, `xorshift16`, `bounded_rand`, `counter_*`,
  `ema_update`, `moving_avg_update`.
- **time / budget** (only the *non-alias* ones — skip `time_until`=`sub_sat`,
  `deadline_missed`=`is_ge`): `cooldown_remaining`, `used_percent`, `fits_budget`.
- **needs `i16`**: signed deltas / `lerp` / risk deltas.

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
