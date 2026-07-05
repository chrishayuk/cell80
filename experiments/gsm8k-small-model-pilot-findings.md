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

## Part 1: single-shot scripted extraction

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

## Part 1 conclusion

This is real evidence for the spec's own pre-registered "Honest limits" line, not a new
claim: a raw single-shot 3B model, unaided, gets the plan-IR extraction step right about
15% of the time on genuinely varied GSM8K problems; one cheap repair round buys real but
limited ground (15% → 30%); and the failures cluster in identifiable, largely mechanical
categories (bare literals as operands, double-assignment, rate-unit tagging) rather than
being unpredictable noise — which is itself useful, since it suggests the extraction prompt
(not the plan IR or the renderer) is the lever, exactly where M3's own design already expects
the work to be. What Part 1 alone doesn't answer: whether a better-engineered extraction
prompt closes most of this gap, or whether it's a genuine capability ceiling at this model
size — that question is what Part 2 below actually goes after, from a different angle.

## Part 2: does it need to call our tools?

Part 1 never gave the model any tools — it just asked for JSON in plain chat and I (the
harness) parsed and ran it. A fair objection: `cell_solve` is a real MCP tool
(`cell80-mcp/src/cell80_mcp/server.py`); the honest test is whether a model can extract
*and* call it correctly through actual tool-calling, not just produce text that happens to
be JSON-shaped. This section adds `cell_solve` as a proper OpenAI-format function tool
(mirroring the MCP tool's schema/description 1:1) and drives it through a real multi-turn
tool-calling loop, first reusing `cell-eval`'s existing `agent.py`/`run_episode` machinery
(the same loop `adoption.py`/`composition.py` already use), then Ollama's native `/api/chat`
endpoint directly (matching `cell80-mcp/examples/chat_demo.py`'s proven-working pattern,
rather than the OpenAI-compat `/v1/chat/completions` shim).

### 2a. Real tool-calling via `cell-eval`'s agent loop (granite4.1:3b, N=20)

Same 20 problems, `cell_solve` given as a genuine tool, up to 6 turns, `run_episode`
dispatching each call to the real `CellHost::solve`. **Result: 0/20 genuinely correct via
the tool** (2 stated answers matched, but both bypassed `cell_solve` entirely — see below).

Two adoption problems compounded:

- **Steering sensitivity, confirmed the hard way.** A longer, hedged system prompt
  ("STRONGLY PREFER... if X read the kill reason and fix Y else Z...") suppressed
  tool-calling *completely* — 0 calls across all 20 problems, silently. Swapping to a
  short, blunt prompt (closer to `adoption.py`'s own minimal style) got the tool called on
  9/20. This reproduces `adoption.py`'s own stated lesson ("low adoption is usually weak
  steering") for `cell_solve` specifically, for the first time.
- **Non-convergent retries.** Even the 9/20 where the tool *was* called mostly burned all 6
  turns without escaping the same schema-violation categories Part 1 already found (bare
  literals as operands, wrong unit tags) — live in-loop repair converged no better than
  Part 1's scripted repair round did.
- **False positives from silent bypass.** `row4_james` and `row97_harry` both landed a
  "correct" final answer — but tracing the actual tool calls shows the first `cell_solve`
  attempt got killed (`unknown unit 'times'`), and the model simply answered from its own
  arithmetic on the next turn instead of retrying. Filtering these out (the same
  `correct` vs. `correct_via_cell` distinction `adoption.py` already insists on) leaves
  **zero** genuine tool-verified answers.

### 2b. Switching to Ollama's native endpoint fixes adoption, not extraction

`chat_demo.py` (a working, hand-verified demo elsewhere in this repo) uses Ollama's native
`/api/chat`, not the OpenAI-compat shim. Reproducing that exactly (own `requests.post` loop,
`{"role":"tool","name":...}` message format) on a **small, targeted 1-2 problem sample per
variant** — not full 20-problem runs; the point here was to catch qualitatively distinct
failure modes cheaply before committing to a full batch, per the discipline that caused this
section to exist in the first place — surfaced a genuinely new axis: **schema strictness has
a narrow, model-specific sweet spot**, tested in three steps on granite4.1:3b:

1. **Loose schema** (`ops` described only in the tool's prose, no nested JSON Schema): the
   model called the tool, but repeated the *exact same* broken object-shaped op
   (`{"a":9,"b":60,"op":"mul","out":"..."}` instead of `["mul","a","b","out"]`) five times
   in a row with zero variation.
2. **Fully strict schema** (deep nested schema: quantities/ops/target all typed and
   required): tool-calling broke **completely** — empty content, no tool call, no error,
   just silence. Over-constraining is at least as bad as under-constraining.
3. **Medium schema** (only `ops` forced to a 4-string array; quantities/target left loose):
   granite **self-corrected the format** between turn 1 and turn 2 unprompted — real,
   working in-loop repair, gated behind exactly the right amount of schema pressure. But
   once the shape was right, it got stuck on the same bare-literal-operand mistake Part 1
   already found, repeating the *identical* broken plan 4 more times with zero variation.

The football problem (`row85`, medium schema) surfaced something further: the model
correctly submitted **two candidate plans in one call** — genuine, intended multi-candidate
usage exactly as `cell_solve`'s own description invites. One was killed (bad extraction);
the other "survived" and returned `answer: 22` — wrong (expected 15: `losses = total - diff`
instead of the correct `(total - diff) / 2`), a comprehension error, not a schema one. Then,
having already received a real, usable tool result, **the model called the identical tool 5
more times instead of ever replying with a final answer** — a third failure mode distinct
from adoption and extraction: it doesn't reliably recognize a completed tool result as done.

### 2c. Cross-model check (small diagnostic samples, N=2 per model — not full runs)

Same medium-strict schema, same 2 sample problems, three more models:

- **`qwen2.5:3b`** (pulled specifically for this — fast, non-thinking, similarly sized to
  granite): the medium schema that worked for granite made it go silent too (0 calls) — it
  breaks at a *lower* strictness threshold than granite does. With an even simpler,
  flatter schema it does call the tool, but **ignores the `plans` wrapper entirely**,
  passing `quantities`/`ops`/`target` as flat top-level arguments instead of nesting them —
  and its `ops` shape is a third, distinct malformation:
  `["mul", ["sprints_per_week", "runs_per_day"], "distance_per_sprint"]` (a nested
  sub-array, missing the 4th element).
- **`gemma4:e4b`** (9.6GB, `tools`+`thinking` capable): calls the tool readily (3-4 times),
  but uses **integer ids** (`"id": 1`, `"id": 0`) instead of strings, referencing them as
  bare integers in `ops` (`["mul", 1, 2, 3]`) — reinventing a positional/indexed addressing
  scheme instead of the named one the schema describes. A fourth distinct malformed
  convention. Every single call failed (`quantity needs an 'id'`), repeated near-identically
  with no adaptation — and **both sample problems landed a "correct" stated answer with
  zero successful tool calls**, the identical silent-bypass pattern found for granite in
  2a, now confirmed on a second model.
- **`qwen3.5:4b`** (a genuine "thinking" model): 3-4+ minutes and 7,000+ reasoning tokens
  per call, making it impractical for this kind of rapid iterative testing. Tested only in
  Part 1's single-shot mode, and only partially (the run was killed after several problems
  once the per-call cost became clear) — not enough data for a real rate, but one full
  transcript is worth recording: given the complete system prompt, it correctly derived the
  same closed-form pattern as row85 (`wins = (total + margin) / 2 = 15`) that granite's
  *single-shot* attempt got right too but its *tool-calling* attempt didn't — weak evidence
  that raw reasoning capability and tool-calling reliability are separate axes, not the same
  thing.

### Part 2 conclusion

Four models, four distinct malformed-JSON conventions (object-shaped ops, bare-literal
operands, a flattened/nested-array hybrid, integer ids), and one completely consistent
result: **when `cell_solve` repeatedly fails, models abandon it and silently self-compute an
answer instead of persisting or escalating honestly** — observed identically in granite and
gemma4, the only two models tested to that depth. This is the one finding that should worry
a real M3 design most, more than any single accuracy number: nothing in the current
agent-loop pattern (here, or in `cell-eval`'s existing `adoption.py`/`composition.py`)
stops a model from answering unverified once it gets frustrated with a tool, and a naive
"did it state the right number" metric will not catch this — only tracing whether the
*final* answer actually came from a *successful* tool call does.

Two separate problems fell out of this, and they don't have the same fix. The bypass problem
has a cheap, mechanical answer: **a harness should never accept a stated answer that didn't
come from a successful tool call** — gate on `correct_via_solve`, not `correct`, always. The
schema-fragility problem does not have a clean fix: there is no schema strictness level that
worked across all four models (granite's sweet spot broke qwen2.5; nothing tested fully
avoided some per-model malformation), and chasing a bespoke schema per model looks like
overfitting to quirks, not measuring a real capability.

Given that, the recommendation this pilot supports is to **not** route M3 extraction through
native tool-calling. Part 1's scripted approach — ask for JSON in plain text, parse and
validate it in the harness, run it through the real `cell80 solve` binary directly —
outperformed every tool-calling variant tested here (30% combined vs. ~0% genuinely-via-tool
across two models measured to that depth), for a structural reason: it doesn't depend on
each model's tool-calling grammar cooperating, only on producing text a harness can parse
with its own (more tolerant, more correctable) logic. Tool-calling adds a real, distinct
failure axis — adoption sensitivity to prompt length, per-model schema breaking points, JSON
dialect quirks — on top of the extraction/comprehension gap Part 1 already measured, without
buying back any correctness for the cost.
