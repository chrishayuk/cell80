# Growing the cell library — what's useful

*A prioritized guide for expanding `cell80/cells/` beyond the 8 seeds. The goal is a
**larger useful library** that an agent retrieves, runs, and composes — and that doubles as a
good substrate for the three evals (`retrieval` / `adoption` / `composition`).*

## Grow deliberately — for the evals and the wedge, not just the count

A bag of unrelated functions is a *weak* library: retrieval becomes trivial (every cell is
easily separable, so there's no signal), and composition has nothing to chain. Grow along the
axes the evals reward:

- **retrieval** wants **confusable families** — 3-4 cells per family that collide in text but
  differ in behaviour (the only thing that gives top-1 retrieval teeth).
- **composition** wants **predicates + transforms** that chain — especially boolean cells you
  can branch on (data-dependent composition is the part that actually needs a graph).
- **adoption** wants **plausible tools** a user would naturally ask for.
- the **stated beachhead** is reward/validation kernels for rate-decoupled RL (e.g. SOMA) — so
  cells that are *both* a real tool and something a SOMA environment computes earn their place
  twice (and close the cell80 ↔ SOMA loop: cells become SOMA's deterministic verifiers).

## Principles (what makes a cell worth adding)

- **Fits the integer envelope** (`u8`/`u16`/`u32`, no float/string/syscall, bounded cycles).
  The compile error *is* the "this belongs in host code" signal — lean into it. Keep new cells
  **unsigned-friendly** until `i16` lands (do abs via a branch, like `abs_diff`/`manhattan`).
- **Small and pure** — tens of bytes of behaviour, deterministic, cycle-honest.
- **Belongs to a family with confusable siblings** (retrieval depth).
- **Composes** — produces/consumes values others use; include **boolean predicates** (`-> 0/1`).
- **Bonus: it's a reward/validation/game kernel** (the wedge + the SOMA loop).

## Families to grow (prioritized)

### 1. Composition glue — add these first
They unlock *data-dependent* chains (branch on an intermediate result):
- predicates (`-> 0/1`): `is_le`, `is_ge`, `eq`, `divides`, `is_even`, `in_range`
- safe arithmetic: `add_sat`, `sub_sat`, `mul_sat`, `div`, `mod`

### 2. Confusable families — give retrieval teeth
- **distance** (siblings of `manhattan`): `chebyshev`, `euclid_sq`
- **number theory** (siblings of `gcd`): `lcm`, `divides`, `mod`, `is_prime`, `isqrt`,
  `pow_mod`, `digit_sum`
- **bounds** (siblings of `clamp` / `range_check`): `wrap`, `saturate`, `quantize`/`bucket`,
  `snap_to_grid`, `map_range`/`lerp`
- **extremum** (siblings of `min` / `max`): `median3`, `clamp_low`, `clamp_high`

### 3. SOMA reward/validation kernels — the strategic pick
Literally what a rate-decoupled-RL environment computes; serves the wedge *and* the ecosystem
loop: `phase_of(tick, period)`, `tick`, `move_valid(x,y,dir,w,h)`,
`collision(x1,y1,x2,y2,r)`, `step_toward(cur,target)`, `reward_delta`, `grid_index` /
`index_to_xy`.

### 4. Cell-native showcase — where the substrate shines
- **bit/encoding**: `popcount`, `parity`, `hash`, `pack`/`unpack`, `checksum`/`crc8`
- **RNG/sequences** (stateful, via struct state): `lcg_next`, `xorshift32`, `bounded_rand`,
  `fibonacci`/`is_fibonacci`

## A concrete first batch (~12, spans all four priorities)

`is_le`, `divides`, `in_range`, `add_sat`, `sub_sat`  · `chebyshev`, `euclid_sq` (distance
siblings) · `lcm` (gcd sibling) · `wrap`, `median3` (bounds/extremum siblings) ·
`phase_of`, `collision` (SOMA) · `popcount` (cell-native).

That single batch gives retrieval three new confusable families, composition a full set of
predicates + transforms, and the SOMA loop two real kernels.

## Mine the ecosystem first

`chuk-math` / `chuk-mcp-math` / `chuk-synthetic-data` (e.g. its fibonacci verifier) likely
already hold integer kernels worth porting straight in — cheaper than authoring from scratch,
and it ties the loop.

## After authoring: re-run the evals

Each new **family** (≥3 confusable members) is a retrieval test case; each **predicate +
transform** pair is a composition test case. After a batch, add their queries to
`cell-eval/datasets/retrieval.jsonl` (the LLM-paraphrase generator can scale these) and re-run
`cell-eval retrieval` / `composition` — a bigger, confusable library is what makes those
numbers trustworthy and the embedder/rerank design decisions real.
