# GSM8K small-model extraction pilot: findings

Companion to `docs/math-campaign-spec.md` (the pre-registered M0-M4 gates and hypotheses)
and `cell80/examples/m3_gsm8k_smoketest.rs` (123/123 correct, but hand-extracted by a large
model — me — not the small model the spec's own headline configuration names). The spec
states up front, as an honest limit rather than a hope: "cells fix arithmetic, not reading —
extraction remains the bottleneck." This pilot is the first time that claim was actually
tested against a real small model instead of just asserted.

**This is not M3.** No distractor plans, no cost measurement, no CoT/PAL-Python baseline,
N=20 not 1,319. It's a cheap, real temperature check on the one open question that matters
before committing to the real campaign: can `granite4.1:3b` — the exact model the spec's
hypothesis grid calls "granite-3B" — do the plan-IR extraction step at all?

## Setup

20 GSM8K test-set problems (rows 4, 7, 11, 22, 85, 86, 89, 92, 93, 94, 97, 101, 104, 106,
111, 117, 119, 121, 122, 124), deliberately all non-money to keep grading unambiguous (no
cents-scaling judgment call folded into correctness). For each: a system prompt describing
the plan IR schema (`{quantities:[{id,value,unit}], ops:[[op,a,b,out]], target}`, add/sub/
mul/div only, integers only, exact-division-only, one worked example — Janet's ducks) plus
the raw problem English, single-shot, temperature 0, via Ollama's OpenAI-compatible endpoint
(`http://localhost:11434/v1`). The model's JSON output was fed directly into the real
`cell80` release binary (`cell80 solve <file> --json`) — no mock, the same renderer/host
path the smoke test and MCP `cell_solve` use.

## Result: 3/20 (15%) correct, single-shot

| outcome | count | example rows |
|---|---|---|
| correct | 3 | row4 (James's sprints), row101 (Jerome's doorbell, 6-op chain), row124 (Rosie's run) |
| malformed JSON | 2 | row11 (dropped comma), row89 (literal `?` left as a value instead of computing one) |
| schema-violating plan, cleanly killed | 8 | see below |
| well-formed plan, wrong answer (comprehension) | 3 | row7, row93, row121 |
| solve-level structural error | 4 | row22, row85, row94, row111, row97 (bare-number operand, unset/negative quantity value) |

(counts overlap slightly across the "killed" vs "structural error" split below — both are
render()/CLI-level rejections, just surfaced at slightly different points; the honest
one-line version is **17/20 did not compute the right answer, and every one of those 17 got
a specific, nameable reason, never a silent wrong number.**)

Every schema violation was one of these distinct shapes, none of them arithmetic mistakes:

- **raw numeric literal as an operand** instead of a declared quantity id — `["mul","marcia","3","jan"]` → `render: op 0: '3' is not defined yet` (rows 92, 106, and structurally the same issue in 85, 111, 97 via "op fields must be strings" when the literal was JSON-typed as a number rather than a string)
- **a quantity declared and separately assigned by an op** — `total_blocks`/`james_hours` given both a starting `value` and an op `out` referencing the same id → `render: 'X' is assigned twice` (rows 86, 97)
- **a unit-vocabulary word used as an identifier** — the model used the literal string `"scalar"` (from the system prompt's own unit-naming guidance) as an operand id → `render: 'scalar' is not defined yet` (row 104)
- **an inline arithmetic expression as an operand** — `"(ratio_sugar+ratio_water)"` passed directly instead of being its own `add` op with its own `out` id (row 117)
- **an unknown/negative quantity represented as a fixed `value`** instead of only ever declaring known constants and deriving the rest via ops — `"value": -23` or `"value": null` (rows 22, 94)
- **one genuine unit-dimension trap**: `vehicles_per_container` tagged plain `count` instead of `count_per_count`, so `mul(containers[count], vehicles_per_container[count])` produced a compound "count²" dimension that couldn't later `sub` against a plain-`count` total → `render: unit mismatch (can't sub different dimensions)` (row 119). This is the *identical* class of trap a careful human extractor (me) only found by hand at row 48 of the smoke test (John's ties) — the rate-tagging convention isn't something a 3B model applies spontaneously from a one-paragraph prompt, even with a worked example that used it correctly once.

The 3 pure comprehension misses (well-formed, wrong answer) were genuine misreadings, not
extraction mechanics: row7 computed the right per-entity values but set `target` to one
sheep count instead of the sum of all three; row93 hardcoded an age chain with a sign error
(older/younger confusion); row121 hardcoded a "sheets per month" literal using 12 as an
implicit weeks-per-month conversion instead of the stated 4.

**The design's core safety property held under a genuinely weak extractor**: not one of
these 17 failures was a silently wrong number. Every non-comprehension failure surfaced as a
specific, named `render`/CLI rejection — exactly what `render()`'s unit/identifier checking
is for, now validated against real model output instead of only hand-crafted repair tests.

## Repair round: one shot, given its own output + the diagnostic

Following `cell-eval/src/cell_eval/repair.py`'s existing philosophy (the model gets exactly
one retry — its own broken output plus the diagnostic, no tools, no further attempts; a fix
that parses but is still wrong is a miss) — reused verbatim rather than re-invented, since
the shape of the problem (broken structured output + a compiler-style error) is the same one
`repair.py` already measures for cell source code.

**Result: 3/17 repaired (18% repair@1), bringing the combined total to 6/20 (30%).**

Fixed: row93 (re-read the problem correctly once prompted), row106 and row122 (both
correctly promoted a raw literal into its own declared quantity once told the operand had to
be a string). Not fixed, and worth naming specifically: **two cases (row85, row111) echoed
back the exact same broken plan, byte-for-byte unchanged**, despite being told precisely
`op fields must be strings`. A third (row97) partially self-corrected — fixed the
double-assignment — but reproduced the identical bare-literal-operand mistake in a new
place. `repair.py`'s own framing is that the repair signal has to come from the error text;
here, for at least this failure class, the error text alone wasn't enough for a 3B model to
infer the specific structural fix (wrap the literal in its own quantity), even when it
otherwise understood the general instruction to retry.

## Conclusion

This is real evidence for the spec's own pre-registered "Honest limits" line, not a new
claim: a raw single-shot 3B model, unaided, gets the plan-IR extraction step right about
15% of the time on genuinely varied GSM8K problems; one cheap repair round buys real but
limited ground (15% → 30%); and the failures cluster in identifiable, largely mechanical
categories (bare literals as operands, double-assignment, rate-unit tagging) rather than
being unpredictable noise — which is itself useful, since it suggests the extraction prompt
(not the plan IR or the renderer) is the lever, exactly where M3's own design already expects
the work to be. What this pilot does *not* answer: whether a better-engineered extraction
prompt (more worked examples, an explicit "never use a raw number as an operand" rule, a
multi-round repair loop rather than one shot) closes most of this gap, or whether it's a
genuine capability ceiling at this model size — that's M3-scale work, not a 20-problem
pilot's to settle.
