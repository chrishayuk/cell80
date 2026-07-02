# cell80 roadmap — fixes and features

**Ordering principle:** the product claim is *deterministic, auditable, exactly-metered
kernels for agents*. Everything that makes that claim false ships before anything that
makes it bigger. Each item has a definition of done (DoD); phases are sequential gates,
items within a phase are parallelizable. (The narrative companion to
[roadmap.md](roadmap.md), which tracks what's built; this file tracks what makes the
claim true.)

## The end state

The headline sentence changes from aspiration to fact. Today the honest claim is "a Z80
sandbox with a restricted-Rust compiler and a promising retrieval sketch." Post-roadmap
it's: **a verified-deterministic kernel substrate where an agent can discover, verify,
execute, and cache pure functions at microsecond cost with exact metering** — every word
load-bearing and defensible. Phase 0 makes "verified" true (no accepted program can
silently diverge from rustc on either target). Phase 2 makes "discover" true — the
difference between a demo and an ecosystem. Phase 3 makes it *safe to let agents author
and share cells*, where the emergent behaviour starts.

Three things become possible that aren't today:

1. **Closed-loop cell authorship.** An LLM writes a cell; the diagnostics are good enough
   for one-shot repair (measured by the repair-rate eval, not hoped); the admission gate
   rejects it unless it's retrievable under paraphrase and adversarial queries; the signed
   artifact enters a library that stays healthy as it grows. A self-extending tool
   ecosystem with quality control at ingest — MCP's answer to "how do tools scale" is
   currently "context windows get bigger."
2. **Verification as a primitive.** Memoization + determinism means any claim an agent
   makes that can be phrased as a cell run is checkable for effectively zero cost,
   forever. Scoring, constraint checking, invariant testing stop being things you trust
   the model about.
3. **The SOMA organ layer gets a production substrate.** Fast reflexes at kHz–MHz under a
   slow planner, T-state budgets as a real resource the planner allocates, `trapped_ops`
   making reward functions un-gameable.

Strategically this is capability via *trust*, orthogonal to the industry's capability via
scale: a small verified core the agent leans on so the expensive stochastic part does
less. A local model with a library of a million verified microsecond kernels punches far
above its parameter count — and the kernels cost bytes, not VRAM. It is the symbolic
complement to externalizing knowledge: this externalizes *procedures*.

**The proof-point demo:** an agent, given a novel task, searches the library, finds no
suitable cell, *writes one*, gets it admitted through the gate, and the next agent
retrieves and reuses it — the whole chain signed, metered, and replayable. One loop,
every phase at once, and neither the Wasm world nor the MCP world can run it.

---

## Phase 0 — Close the determinism contract ✓ (shipped)

The runtime's value proposition is "this number is trustworthy." These were the holes in
that sentence.

**0.1 Reject recursion at compile time. ✓**
Direct and mutual recursion compiled and returned silently wrong values when a slot was
read after a recursive call returned (`fact` via `let t = fact(n-1); n * t` → 1 instead
of 120), while tail-shaped recursion worked by accident (intermediates riding the
hardware stack). A three-colour DFS over the lowered call graph now rejects any cycle at
lowering, on both entry paths (`lower_program` and `compile_fn`), naming the cycle:
`recursion is not supported (Stage 1: static locals) — rewrite as a loop (cycle: a → b → a)`.
*DoD met:* the probe programs (slot-after-call factorial, tail-shaped, local-array
recursion, mutual recursion, single-fn self-recursion) are reject-tests in
`tests/diff/recursion.rs`, plus a DAG-still-compiles control; no cyclic call graph
reaches codegen.

**0.2 Differential-test the Cell target. ✓**
`compile_fn`/`compile_program` hardcoded `Target::Spectrum48`, so the rustc oracle never
exercised the `ED FE` trap path the cell VM actually ships. The harness
(`tests/diff/harness.rs`) now compiles every diff test for **both targets** — the RAM bus
services the full trap set (MUL16/DIVMOD16/MUL32/DIVMOD32/FILL16) — asserts the targets
agree with each other, and asserts both match rustc. `compile_fn_for`/
`compile_program_for` expose the target choice.
*DoD delta:* rather than a 2× CI matrix, both targets run **inside every test** (a
divergence fails the single test with the target named) — same coverage, no CI fork.

**0.3 Div-by-zero becomes a halt, not a value. ✓**
`/0` and `%0` yielded `0xFFFF` that flowed onward into scoring. `Halt::DivByZero` added;
behaviour is a `CellConfig` policy — `DivByZero::Halt` default, `Saturate` opt-in for
legacy — carried in the image (flag bit 4; absent = halt, so old images load safe). Both
divide traps (16-bit `0x11` and wide `0x13`) honour it; the fast batch path falls back to
the authentic interpreter on a div-zero input so cycles/halt stay exact. Spectrum keeps
saturation (no trap surface to halt through) — the divergence is documented, not hidden.
*DoD met:* `x / 0` reports `halt: div_by_zero`; the policy is tested both ways plus the
image round-trip; the dialect spec defines both targets' behaviour.

**0.4 Write the dialect semantics down. ✓**
[docs/10-dialect-semantics.md](10-dialect-semantics.md): wrapping (release-Rust)
arithmetic, the div-by-zero policy table, evaluation order (including the
right-operand-first exception for `- / % *16`), the no-recursion rule, the 3-arg call
limit, the u32-doesn't-cross-calls rule, and exactly what `check!` guarantees
(release-mode oracle, both targets). README links it from the differential-testing claim
and carries the non-goals.

---

## Phase 1 — The compiler as an LLM-facing API (1–2 weeks)

The primary author is a model; error text is the repair interface.

**1.1 If/match as expressions.**
`if a { 1 } else { 2 }` is the single most idiomatic shape LLMs emit and it currently
dies with a raw syn Debug dump. Lower both arms to an assignment into a temp slot. Same
for `match` arms yielding values.
*DoD:* diff tests for if-expr, nested if-expr, match-expr; the old error path is
unreachable for these shapes.

**1.2 Diagnostic rewrite pass.**
Replace `unsupported statement expression: Expr::Lit {...}`-class messages with
instructive text: what's unsupported, the accepted rewrite, one example. Cover the top
~10 rejection sites in `lower/` (`compile_fn` on multi-fn input should point at
`compile_program`).
*DoD:* no user-facing error contains a `{:?}` of a syn node.

**1.3 Repair-rate eval in cell-eval.**
New metric: given a rejected cell + the diagnostic, one-shot LLM repair success rate per
diagnostic class. Makes 1.2 measurable and catches regressions.
*DoD:* `cell-eval` reports repair@1 per error class; baseline recorded before/after 1.2.

**1.4 Signed `i16`.**
Rewards, deltas, and coordinates go negative. Two's-complement add/sub/compare are
near-free; signed mul/div via trap on Cell, runtime on Spectrum. Document the fixed-point
idiom for fractional values in the same commit.
*DoD:* diff tests across the sign boundary (−1, i16::MIN, mixed-sign mul/div); README
numerics table updated.

---

## Phase 2 — Retrieval is the product (2–6 weeks)

An agent that finds the wrong capsule in 0.25 µs loses to one that finds the right
function in 100 ms. This phase gates the "millions of cells" claim.

**2.1 Confidence-gated escalation path.**
Tier 1: lexical (tf-idf, current). Tier 2: embedding rerank over manifest descriptions.
Tier 3: behavioural disambiguation — run the top-k candidates on precomputed
discriminating probe inputs and match output fingerprints. Cells are microsecond-cheap
and deterministic; *executing the candidates* is a retrieval signal no other tool
ecosystem can afford. This is cell80's native trick — lean into it.
*DoD:* paraphrase ≥ 0.85, adversarial ≥ 0.75 on the current eval set at ≤ 10 ms p99
(tiers 1–2) / ≤ 50 ms (tier 3).

**2.2 Per-cell admission gate.**
A cell enters the library only if it survives its own paraphrase + adversarial query set
(eval-driven growth as definition of done, applied at ingest).
*DoD:* `cell80 index` refuses (with a report) cells whose queries collide with an
existing cell's fingerprint.

**2.3 Scale the eval library.**
100 cells can't surface collision behavior. Grow to ~1,000 (generated + curated), track
P@1/adoption/composition as the library grows — the retrieval-quality-vs-scale curve is
the headline chart for the vision section.
*DoD:* eval runs at 1K cells; curve published in README.

---

## Phase 3 — Trust and the agent loop (parallel with Phase 2)

**3.1 Content-addressed, signed cells.**
`source_hash` covers source; extend the `.cell` format so the artifact hash covers the
emitted image + manifest, and add optional signing. A registry of executable artifacts
invites poisoning; answer it before anyone asks.
*DoD:* `cell80 run` verifies image hash by default; `--no-verify` exists for dev; format
version bumped.

**3.2 Escalation contract (cell → host).**
Make the boundary explicit and machine-readable: a manifest field declaring what the cell
*can't* do and a structured "escalate" halt so an orchestrator knows a request exceeded
the kernel class (needs strings / floats / I/O) versus failed. Positions cell80 honestly
as the deterministic organ layer with a defined hand-off, not a universal runtime.
*DoD:* `Halt::Escalate(reason)` in the taxonomy; cell80-mcp surfaces it as a typed result.

**3.3 Memoization cache.**
Determinism makes `(image_hash, args) → (result, t_states, trapped_ops)` cacheable
forever. Trivial to build, and it turns repeated scoring/verification calls into hash
lookups — the economic argument for the runtime in one feature.
*DoD:* opt-in cache in `Runner`; benchmark shows cached-call cost; hit-rate counter in
`Report`.

---

## Phase 4 — Codegen stage 2 (after 0–1, before feature growth resumes)

**4.1 The `Ins` layer.**
Codegen emits raw bytes (`a.byte(0x21)`) — there is nothing for a peephole to rewrite,
and the retrofit cost grows with every line added to `codegen/`. Introduce an instruction
enum between codegen and encoding *now*, behaviorally neutral, validated by
byte-identical output on the full test corpus.
*DoD:* all codegen goes through `Ins`; emitted images byte-identical to pre-refactor for
the whole suite.

**4.2 Peephole pass.**
Redundant load elimination (`LD (addr),HL; LD HL,(addr)`), push/pop pairs around trivial
RHS, dead `mask_to_width`. Measure on the size report — the win is bytes and T-states,
both metered exactly, so the pass proves itself.
*DoD:* corpus-wide size/T-state deltas published; diff suite green on both targets.

**4.3 u32 completion + array elements.**
Finish the half-open u32 surface (array elements are currently rejected), keep u64/floats
gated behind capability flags — resist the drift toward "worse Wasm."
*DoD:* u32 arrays diff-tested; capability flags reject u64/float syntax with
Phase-1-quality diagnostics.

---

## Explicit non-goals (in the README)

- **Strings, floats-by-default, I/O, network** — that's the escalation path (3.2), not
  the roadmap. The moment the ISA chases general applicability, the differentiation vs
  Wasm evaporates.
- **JIT / speed chasing** — Wasm wins warm compute; the moat is exact metering,
  auditability, byte-scale artifacts, and determinism. Protect the moat, don't race the
  loser's race.

The failure mode that would dissolve everything is feature drift toward a worse Wasm; the
implicit bet is that a narrow thing that's *actually true* beats a broad thing that's
approximately true, in a field currently drowning in the latter.

## Sequence summary

Phase 0 was a single "close the determinism contract" commit-set and gates everything.
Phase 1 makes the compiler usable by its actual author. Phase 2 is the product bet and
the long pole. Phase 3 rides alongside. Phase 4 lands before the next dialect expansion,
or the retrofit price keeps compounding.
