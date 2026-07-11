# Coverage-map taxonomy — an amendment to the coverage-oracle discipline

*Status: **draft, 2026-07-11.** Adds one classification axis to
`docs/real-valued-cells-spec.md` Part 2's coverage-map discipline (`covered` /
`composable` / `candidate` / `host_only` / `out_of_scope`). Motivated by the Excel80
pack review but stated project-wide — it is not scoped to any one pack. Supersedes the
same day's earlier draft (`composable-author` gated on "compatibility-namespace packs
only"), which was too narrow: the reasons below are general, Excel80 just happens to
produce an unusually large number of rows that clear them.*

**One sentence:** `composable` splits into `composable-skip` (the existing default —
don't author) and `composable-author` (author anyway, with a named reason), because
well-known/expected functionality and genuine algorithmic complexity are both legitimate
reasons to pay a library's retrieval-cost tax on a technically-composable function — not
only when the function also happens to match an external compatibility API.

---

## Part 1 — the rule being amended, restated

`docs/real-valued-cells-spec.md` Part 2 classifies every candidate cell against the
existing library before authoring anything:

| status | meaning |
|---|---|
| `covered` | an existing cell (or alias) already does this — name it |
| `composable` | existing cells compose to it — **do not author**; record the composition |
| `candidate` | good cell: exact or boundable, tiny, distinct — enters a wave |
| `host_only` | escalation territory (needs capability the dialect doesn't have) |
| `out_of_scope` | not wanted at either layer |

This rule exists for a measured reason, not a purity preference: every additional cell
pays a retrieval-precision tax (paraphrase/adversarial probes get harder to separate as
near-duplicates accumulate — this is what tripped round 3's kill-gate,
`docs/library-growth.md`). `composable` cells are refused by default because the tax is
real and the marginal cell adds no new *capability*, only a shorter path to one that
already exists.

## Part 2 — the amendment

`composable` is not a single bucket. Split it:

- **`composable-skip`** — the default, unchanged. Existing cells compose to it, neither
  reason below applies, don't author.
- **`composable-author`** — existing cells compose to it, but at least one of these
  holds, named explicitly per row:
  1. **Well-known / expected functionality.** Domain convention already treats this as
     an atomic, named operation, so a caller reasonably expects it to exist as its own
     cell rather than being asked to assemble it — whether the name comes from an
     external compatibility API (Excel's `PMT`) or general algorithmic convention
     (`gcd`, `crc32`, a well-known numerical method). **Convention fragility**
     strengthens this case but isn't required on its own: a sign convention,
     omitted-argument default, or rounding/date-basis rule that's easy to get subtly
     wrong re-deriving by hand each time is exactly the kind of thing a canonical cell
     earns its keep encoding once, tested.
  2. **Genuine complexity.** The "composition" is actually a non-trivial algorithm
     (iteration, a convergence rule, multi-step case analysis) rather than simple glue
     of one or two existing outputs. **If this is the only reason that applies, check
     the row wasn't mis-classified as `composable` in the first place** — real
     algorithmic complexity usually means it isn't glue-composable at all, and belongs
     in `candidate` instead. This reason exists mainly to catch that misclassification.

A `composable-author` row must record which reason(s) applied — this is a decision, not
a default, so it doesn't get silently re-litigated later.

**Existing-codebase precedent, not hypothetical:** `gcd_u32` already shipped despite
being expressible as a loop over already-existing primitives (`mod`, comparison) — it
was authored because it's a well-known named algorithm callers expect to find directly,
the same reasoning this amendment now names explicitly rather than leaving implicit.

## Part 3 — scope: applies project-wide

No pack is exempt and none gets a lower bar. GSM8K/MATH/AIME's already-refused rows
(`count_divisors`, `dist_sq`, `is_divisible_by_k` — `docs/math-campaign-spec.md`'s
MATH/AIME section) stay refused under this same amendment, not because those packs are
carved out of it: they're pure aliases of an already-existing cell, with no domain
convention naming them as a separate atomic operation and no algorithmic complexity —
they fail both reasons cleanly. What changes with Excel80 is not the bar but the
distribution: a compatibility namespace is unusually dense in functions that clear
reason 1, because "well-known, expected, externally-named operation" is close to the
definition of what such a namespace contains.

## Part 4 — worked examples

| function | technically composable? | classification | reason |
|---|---|---|---|
| `PMT`/`FV`/`PV` (Excel) | yes — checked arithmetic + repeated-multiply compounding (the `frac_pow` idiom) | `composable-author` | reason 1: recognizability + convention fragility (sign convention, omitted-arg defaults) |
| `gcd_u32` (already shipped) | yes — a loop over `mod`/compare | `composable-author` (post-hoc naming of why it was right to author) | reason 1: well-known named algorithm |
| `SLN` straight-line depreciation, `(cost - salvage) / life` | yes — one division | `composable-skip` (resolved, see below) | reason 1 doesn't apply (no fragile convention) and reason 2 doesn't apply (glue, not an algorithm) |
| `IRR`/`RATE` (Excel) | nominally, but really an algorithm (Newton iteration + convergence rule) wearing a composable disguise | `candidate` | reason 2 flags this as a misclassification risk — file as `candidate`, not `composable-author` |

**Correction, 2026-07-11 (the real coverage-map pass, `docs/excel-financial-map.md`):**
this table originally grouped `NPV` with `PMT`/`FV`/`PV` as `composable-author`. Running
the actual classification against Excel's real signatures found that wrong: `NPV(rate,
value1, [value2], ..., [value254])` takes an arbitrary-length list of independently-valued
cash flows — the array-state-field gap, not a fixed annuity shape — so `NPV` is
`host_only`, not `composable-author`. "Same TVM family" is not the same as "same
classification"; each function's *actual* signature decides, not its neighbors'. The
`SLN` row, left "tentative" here originally, is likewise now resolved
(`composable-skip`) by that same pass, both updated above rather than left as stale
predictions once real evidence existed.

## What this note does not do

It does not run the Excel coverage map, does not decide Finance80's phase ordering, and
does not authorize authoring anything. It fixes the classification rule any future
coverage-map pass — Excel80's or otherwise — should use.
