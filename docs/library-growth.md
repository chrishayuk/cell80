# Growing the cell library — what's useful

*A prioritized guide for expanding `cell80/cells/`. The goal is a **tiny deterministic
integer standard library** — the boring, universally-useful computations an agent reaches for
constantly and should not re-derive by hand (math, bounds, percentages, ranking, scoring,
bit/flag ops, checksums) — that doubles as a good substrate for the three evals
(`retrieval` / `adoption` / `composition`).*

## What a good cell is

> A tiny, deterministic utility an agent needs often but shouldn't spend tokens re-deriving.

```
small · deterministic · easy to test · easy to describe · useful in many workflows
cheap enough to run constantly · confusable enough with siblings to pressure retrieval
```

Think **stdlib of boring useful computation**, not games and not agent-control policy. The
first wave below is exactly this shape.

## Grow deliberately — for the evals, not just the count

A bag of unrelated functions is a *weak* library: retrieval becomes trivial (every cell is
easily separable, so there's no signal) and composition has nothing to chain. Grow along the
axes the evals reward:

- **retrieval** wants **confusable families** — 3-4+ cells per family that collide in text but
  differ in behaviour (the only thing that gives top-1 retrieval teeth).
- **composition** wants **predicates + transforms** that chain — especially boolean cells you
  can branch on (data-dependent composition is the part that actually needs a graph).
- **adoption** wants **plausible tools** a user would naturally ask for.

## Principles (what makes a cell worth adding)

- **Fits the integer envelope** (`u8`/`u16`/`u32`, no float/string/syscall, bounded cycles).
  The compile error *is* the "this belongs in host code" signal — lean into it. Keep new cells
  **unsigned-friendly** until `i16` lands (do abs via a branch, like `abs_diff`/`manhattan`).
- **Small and pure** — tens of bytes of behaviour, deterministic, cycle-honest.
- **Belongs to a family with confusable siblings** (retrieval depth).
- **Composes** — produces/consumes values others use; include **boolean predicates** (`-> 0/1`).

### What the compiler now gives you (use it)

The dialect supports the ergonomics these cells lean on, so author them *clean*:

- **Comparisons are values:** `fn run(a: u16, b: u16) -> u16 { (a < b) as u16 }` — a predicate
  is a one-liner. All six (`< <= > >= == !=`) materialise to `1`/`0`.
- **`&&` / `||`** (short-circuit): `((lo < x) && (x < hi)) as u16`.
- **Runtime bit shifts:** `x << bit` / `x >> bit` with a *variable* amount — bit ops are
  one-liners (`x | (1 << bit)`); a shift ≥ 16 saturates a `u16` to `0`.

### Standardise these semantics

- **Predicate convention:** `false = 0`, `true = 1`; nothing else. (Built on `bool as u16`.)
- **Divide/remainder by zero:** the cell returns a sentinel, so **guard explicitly**
  (`if b != 0 { a / b } else { 0 }`) — `safe_div`/`safe_mod` are the canonical pattern.
- **`u16` overflow is silent** (wraps), like the seed `weighted_sum`. Saturating cells
  (`add_sat`, `mul_sat`, …) cap at `65535`; the percent/scale cells assume their product fits
  `u16` (`value·scale ≤ 65535`) — beyond that is the host-code signal.

## The contribution rule (every new-cell PR)

```
1. cell80/cells/<name>.rs                       — header (//! summary, //! tags:) + fn/struct
2. cell-eval/datasets/retrieval.jsonl           — direct + paraphrase (+ adversarial) rows;
                                                   verify the direct query ranks the cell #1
3. composition or adoption task (if user-facing) — composition_tasks.jsonl / tasks.jsonl
4. cell80/tests/library.rs                       — edge-case rows (the host oracle)
```

A new cell without eval pressure is just inventory; with confusable siblings and a task it
becomes signal.

## The first wave (landed) — a boring integer stdlib

51 cells across six confusable families (the 8 seeds — `abs_diff`, `clamp`, `gcd`, `manhattan`,
`min`, `max`, `range_check`, `weighted_sum` — plus these):

```
predicates (→0/1)  eq neq is_lt is_le is_gt is_ge is_zero nonzero is_even is_odd
safe arithmetic    add_sat sub_sat mul_sat safe_div safe_mod ceil_div avg2 square
bounds             between_exclusive wrap normalize_0_100 snap_down snap_up round_to_multiple
percent / ratio    percent permille ratio_255 scale_percent increase_percent
                   discount_percent within_percent
ranking / stats    min3 max3 median3 argmax2 argmin2 argmax3 argmin3 sum3 mean3 range3
bit / mask         popcount parity bit_is_set set_bit clear_bit toggle_bit
                   mask_has_all mask_has_any mask_union mask_intersection
```

Each predicate + a transform is a composition seed (e.g. `median3 → is_ge`, `popcount →
is_even`, `scale_percent → clamp`); each family is a retrieval family.

## Next waves (prioritized)

Keep growing the **boring stdlib** before anything domain-specific:

- **number theory** (siblings of `gcd`): `lcm`, `divides`, `is_prime`, `isqrt`, `digit_sum`,
  `pow_mod`.
- **distance** (siblings of `manhattan`/`abs_diff`): `chebyshev`, `euclid_sq`.
- **packing / hashing**: `pack_u8`/`unpack_*`, `checksum`, `crc8_step`, `fnv1a_step`,
  `hash_pair`.
- **scoring / choice**: `weighted_sum2/3`, `choose_best3`, `is_clear_winner`, `bucket3`,
  `quantize`.
- **stateful** (struct state): `lcg_next`, `xorshift16`, counters, `ema_update`.
- **budget / time arithmetic**: `remaining`, `used_percent`, `fits_budget`,
  `cooldown_remaining`.

Organise discovery by **pack** via tags (e.g. `bits`, `percent`, `bounds`) — the loader reads a
flat `cell80/cells/`, so packs live in metadata, not directories.

## Mine the ecosystem first

`chuk-math` / `chuk-mcp-math` / `chuk-synthetic-data` likely already hold integer kernels worth
porting straight in — cheaper than authoring from scratch, and it ties the loop.

## After authoring: re-run the evals

Each new **family** (≥3 confusable members) is a retrieval test case; each **predicate +
transform** pair is a composition test case. Add their queries to
`cell-eval/datasets/retrieval.jsonl` and re-run `cell-eval retrieval` / `composition`.

A bigger, confusable library is what makes those numbers trustworthy. The first wave already
shows the point: **direct** retrieval stays strong (P@1 ≈ 0.91) while **paraphrase** P@1 drops
(≈ 0.40) as confusable siblings multiply — the brittleness of token-overlap search, and the
case for the type-led / capability index (rank by typed signature + `kind = predicate |
transform | …` first, embeddings as the tiebreaker).
