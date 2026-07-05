# PlanFix → Verified Cell Composition — findings & roadmap

An experiment (branch `experiment/planfix`, 2026-07-05) that started as *"can we repair the
broken JSON plans small models emit for `cell_solve`?"* and ended as a complete, working
system: **a model writes an algorithm, its named operations are resolved to already-verified
library cells, the whole thing is compiled to one re-checkable cell, and an answer is emitted
only when two independent derivations agree.** Everything the layer owns is deterministic;
the model does only structure, naming, and comprehension; and the honesty guarantee —
*escalate rather than lie* — holds regardless of model strength.

All numbers below were measured in-session on this M3 against the 20-problem GSM8K pilot slice
(`experiments/gsm8k-small-model-pilot`), using the local `cell80` release binary and Ollama
(`qwen2.5:3b` = weak, `gemma4:e4b` = strong).

## TL;DR — the capstone

`gemma4:e4b`, all 20 problems, structured cross-check (`full_crosscheck.py`):

| metric | value |
|---|---|
| accepted | 16 / 20 |
| **precision (accepted & correct)** | **16 / 16 = 100%** |
| **false positives (accepted & wrong)** | **0** |
| yield (accepted & correct / 20) | 80% |
| escalated | 4 (2 mechanical-fixable, 2 genuine comprehension) |

Recall scaled with model strength (`qwen2.5:3b` accepted 1/5 → `gemma4` 16/20) while **precision
stayed pinned at 100%**. The gate's *safety* is model-independent; only its *yield* moves.

## How the question changed (the arc)

1. **"Repair the broken JSON plans."** The pilot found small models can't hit the strict plan-IR
   JSON, and the *tool-calling* variant did *worse* than free-form. Repairing symptoms.
2. **"JSON is the wrong ask."** Every JSON-shape error (object-ops, int-ids, nested arrays,
   bare-literal operands, inline exprs) is an artifact of flattening a DAG into tuples. Ask for
   **arithmetic** instead and parse it with `ast`.
3. **"Ask for a whole algorithm, not just arithmetic."** The cell dialect (restricted Rust) already
   compiles `if`/`while`/comparison — the arithmetic plan was a special case. The general form is
   **the model writes an algorithm = a cell.**
4. **"Match function calls; reuse, don't regenerate."** The model names operations by intent; we
   *retrieve* the verified library cell. This is the cell80 thesis — remember, don't re-derive.
5. **"Compile the composition to one verified cell."** Drive rustz80's own name-resolution as a
   linker — zero compiler edits.
6. **"Accept only what independent derivations confirm."** Replace the model's unreliable self-
   judgment with an objective cross-check.

## What we built & measured

### 1. Extraction format: code ≫ JSON (`format_bakeoff.py`, `rust_bakeoff.py`)
Same model, same problems, only the *asked-for format* changes:

| format | qwen2.5:3b | gemma4:e4b |
|---|---|---|
| strict JSON plan + repair | 10% | 75% |
| arithmetic → `ast` → plan IR | 50% (→70% after precision fix) | 90% (→95%) |
| direct restricted-Rust → cell | 65% | 95% |

The weaker the model, the bigger the payoff — which is exactly cell80's regime (small models).
Arithmetic/Rust and JSON also fail on *different* problems; the code forms even lift comprehension,
because the model no longer reasons *and* hand-encodes a DAG.

### 2. Precision fix: defer-division (`equations_to_plan.py`, `verify_fix.py`)
The dominant *systematic* error was the model writing **correct** fractional reasoning
(`30/100 * x`, `2/3 * x`, `* 0.9`) that the integer cell truncates to zero. A deterministic AST
rewrite — flatten `*`/`/` chains to `(num, den)` and divide once at the end, plus decimal→fraction
(`0.9`→`9/10`) — fixed **4/8** captured failures with the model untouched (qwen 50%→70%). Safe:
overflow from the reorder hits the cell's checked arithmetic → escalate, never silent-wrong.

### 3. Function matching (`call_match.py`)
Fuzzy intent → verified cell: **7/8**. Char-3-gram search finds the *family*; **arity-matching
against the cell signature picks the right overload** — in 5/7 hits the text-top was the wrong
arity (`gcd3`/`argmax3`/`abs_diff_u32`/`min3`/`lcm3`) and arity-match rescued it to
`gcd`/`max`/`abs_diff`/`min`/`lcm`. The one miss (`hcf`, no shared trigrams with `gcd`) is where the
embedder / behavioural `route_by_example` take over.

### 4. The linker — cells calling cells (`compose_link.py`)
Drive rustz80's own resolution: `cell80 compile` the model source; on
`rustz80: unknown call target \`X\``, search the library, inline the resolved cell's source
(rename entry `fn run`→`fn X`), recompile; loop. **Zero compiler edits.** All demos compiled to a
*single* re-checkable cell (`max`/`min` even folded inline — composition ≈ zero bytes) and ran
correct. `cell80 compile` does *not* auto-link the general library (only builtins like `gcd`), so
the linker is genuinely needed.

### 5. The honesty gate — structured cross-check (`structured_consensus.py`, `full_crosscheck.py`)
Solve each problem two genuinely different ways at temp 0 — **inline arithmetic** and
**library-cell composition** — and accept only if the two derivations agree. This beat both
alternatives: model self-`DONE` was *unsafe* (rubber-stamped a wrong 10); temperature-diversity
consensus was *safe but zero-yield* (noise wrecks the weak model). Structured diversity is safe
**and** yields — see the capstone.

## Escalation analysis (the 4 gemma4 escalations, `escalation_analysis.py`)

| row | want | inline | composed | cause | fix |
|---|---|---|---|---|---|
| `row89` | 8000 | fail | fail | **u16 overflow** — both wrote `88000/11` (correct!), 88000 > 65535 | **width: u32** |
| `row93` | 4 | 4 ✓ | fail | **trailing-`let`** — composed ended `let jackson = amy-5` not a tail expr | **auto-fix** |
| `row86` | 44 | 44 ✓ | 48 | **comprehension** — inclusive vs exclusive year count (`34-23` vs `+1`) | *correctly escalates* |
| `row101` | 175 | 175 ✓ | 765 | **comprehension** — "10 more" vs "10× more" (`+10` vs `×11`) | *correctly escalates* |

**2 of 4 are mechanical (width, syntax)** — fixing them lifts yield 80%→90% at unchanged precision.
**2 of 4 are genuine comprehension divergences** — the gate *should* escalate; that's the honesty
property working, not a defect. (`row86`/`row101` also had `inline` correct, so a 3rd concurring
derivation would recover them via 2-of-3 majority.)

## Roadmap — future cells

- **`u32` variants of the core ops** — `min_u32`, `lcm_u32`, `is_gt_u32` to sit alongside the
  existing `gcd_u32`/`max_u32`/`abs_diff_u32`, so large-value problems (row89 class) can link a wide
  cell instead of overflowing. Pair with width-aware resolution (below).
- **Fraction / rational-scaling cells** — e.g. `scale_frac(x, num, den) = x*num/den` (defer-division
  baked in), `percent_of(x, p)`, `ratio_part(total, part, whole)`. Lets the model *call* a verified
  fractional op instead of hand-writing `x*9/10` and risking division-order truncation.
- **Aggregate cells** — `mean2`/`mean3`, `sum3`/`sum4`, `clamp` — common GSM8K shapes.
- **Comparison/choice family** — already strong (`max`/`min`/`is_gt`/`choose_best3`); extend to
  `argmax`/`argmin` name coverage so intent-names resolve more often.

## Roadmap — future code features

1. **Width-aware compilation** — default `fn run() -> u32`, or auto-widen when any literal exceeds
   `u16::MAX`, and prefer `_u32` cell overloads in resolution. Rescues row89 and the whole
   large-value class. *(Highest leverage.)*
2. **Defer-division as a rustz80 `syn` AST pass** — move the precision fix from the Python `ast`
   path into the compiler so it applies to direct-Rust too (`rustz80/src/lib.rs:108` parses to
   `syn::File`; the pass lives there).
3. **A dialect normalizer** — small deterministic auto-fixes on model source: trailing-`let` → tail
   expression (row93), strip statement macros, bind compound call-arguments to `let`, collapse
   redundant parens. Clears the mechanics weak models can't self-debug.
4. **3rd derivation + 2-of-3 majority** — add a third method (a distinct decomposition); accept on
   majority agreement. Recovers row86/row101-style cases where one path is right and one misreads,
   while keeping precision high.
5. **Escalating resolver** — `text (char-3-gram) → potion embedder → route_by_example (behavioural,
   run candidates on the args)` for names search can't match (`hcf`→`gcd`).
6. **Signature/type-aware resolution** — match arity *and* types (u16/u32, arg order), not arity
   alone; disambiguate same-shape siblings via `route_by_example`.
7. **Multi-function inlining hygiene** — rename inlined helper `fn`s to avoid collisions when
   composing several multi-function cells (v1 assumes single-function cells).

## Roadmap — productionization

1. **Move the pipeline into `cell80` proper** — a `cell80 compose <source>` subcommand and an MCP
   tool (`cell_compose` / a code-mode `cell_solve`) that runs parse → link (search-resolve calls) →
   precision pass → compile → run, with the cross-check gate. Reachable from CLI / cell80-py / MCP,
   not just this Python harness.
2. **rustz80 AST integration** — do call-resolution + the precision pass inside the compiler's own
   `syn` name-resolution, so "a cell that calls library cells" is a first-class compile, not a
   Python retry loop.
3. **Cross-check as a first-class solve mode** — N independent derivations → consensus → accept /
   escalate, wired into `solve`.
4. **Fact provenance** — each accepted composed cell emits a verified fact (artifact hash, args,
   result, cycles) into the `.facts` file, so composed answers become re-verifiable procedural
   memory (the video-1 "memory you can't lie to").
5. **Full-scale eval** — run the whole GSM8K (1319) through `cell-eval` for real precision / recall /
   yield, plus a CoT/PAL baseline, and the cross-model sweep (granite / qwen / gemma).
6. **Grow the library** — land the `u32` and fraction cells above; re-measure yield.

## File index (`experiments/planfix/`)

| file | what |
|---|---|
| `planfix.py` | original deterministic JSON→plan-IR normalizer (superseded, kept for the record) |
| `replay.py` | model-free replay of the granite corpus (baseline vs planfix: 15%→35%) |
| `equations_to_plan.py` | arithmetic (Python `ast`) → plan IR, **with defer-division precision fix** |
| `format_bakeoff.py` / `rust_bakeoff.py` | JSON vs arithmetic vs direct-Rust extraction bakeoffs |
| `verify_fix.py` | deterministic verification of the defer-division fix on captured outputs |
| `call_match.py` | fuzzy intent → cell via search + arity matching (7/8) |
| `compose_link.py` | **the linker** — cells calling cells, compiled to one verified cell |
| `compose_demo.py` | step-by-step interpreter version (cells calling cells, pre-compile) |
| `structured_consensus.py` | **the honesty gate** — inline vs composed cross-check |
| `full_crosscheck.py` | the capstone scorecard (gemma4, 20 problems) |
| `escalation_analysis.py` | capture + diagnosis of the 4 escalations |
| `consensus_compose.py` / `compose_roundtrip.py` / `retrieve_compose.py` | consensus & feedback variants explored en route |
| `probe_embedder.py` / `probe_search.py` | substrate probes (potion embedder weak; char-3-gram search strong) |
| `comprehension_beam.py` / `reframe.py` / `beam_demo.py` | interpretation-beam + model-judge reframe experiments |
