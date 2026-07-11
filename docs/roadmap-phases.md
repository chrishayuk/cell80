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

## Phase 1 — The compiler as an LLM-facing API ✓ (shipped)

The primary author is a model; error text is the repair interface.

**1.1 If/match as expressions. ✓**
`let x = if c { a } else { b };` (and value-`match`) lowers to the statement form
through the destination slot, in `let` / assignment / `return` / tail position, with
nesting, `else if` chains, statements-before-the-value in branches, and u32 arms. A
tail conditional with statement branches stays a statement (void fns legitimately end
with `if`). A value-`if` without `else`, a value-`match` without `_`, and a branch
ending in `;` are each their own instructive error.
*DoD met:* `tests/diff/conditionals.rs` — if-expr in all four positions, nested,
match-expr, wide arms, bool arms, three rejection modes — on both targets.

**1.2 Diagnostic rewrite pass. ✓**
Every syn `{:?}` dump in `lower/` is gone — 14 sites now name the construct in prose
(`describe_expr`/`describe_stmt`/`describe_lit`) and state the accepted rewrite where
one exists. `compile_fn` on multi-item input points at `compile_program`.
*DoD met:* a coverage test probes ten rejection shapes and asserts no diagnostic
contains a syn Debug marker; the repair dataset (1.3) re-asserts it on every class.

**1.3 Repair-rate eval. ✓**
`cell-eval repair`: 20 rejected cells across 10 diagnostic classes, each with intended
behavior as I/O examples; the model gets the broken source + the compiler error, one
shot, no tools — the repair counts only if it compiles **and reproduces the examples**.
Steering is held fixed and deliberately thin: the signal being measured is the error
text. Offline tests pin the loop (a known-good fix counts, a compiles-but-wrong fix
doesn't, every dataset row genuinely rejects).
*Baselines (post-1.2):* granite4.1:3b **repair@1 = 0.60**, gemma-4-26B **0.90** — the
instructive-rewrite classes (`if_no_else`, `match_no_wildcard`, `range_pattern`,
`string_literal`, `float_literal`) repair at 1.00 on both; `recursion`/`closure` sit at
0.00 on the 3B but 0.5/1.0 on the 26B — confirming those misses are a model-capability
floor (one-shot restructuring), not a diagnostic gap. `cell-eval/baselines/repair-*.json` carries
both runs.

**1.4 Signed `i16`. ✓**
`Width::SWord`: add/sub/mul/bitwise share the unsigned bit patterns (wrapping);
comparisons order by sign (S ⊕ V on both the value and branch paths); divide truncates
toward zero via `__sdivmod16` (strip signs → the unsigned core, software on Spectrum /
the DIVMOD16 trap on Cell so `/ 0` still honours the halt policy → reapply signs);
`>>` is an arithmetic shift. Literals (`-5i16` through `i16::MIN`), bit-preserving
casts (`i16 as u32` rejected with the `as u16 as u32` rewrite), params, arrays, and
struct fields. The fixed-point idiom is documented alongside.
*DoD met:* `tests/diff/signed.rs` runs the sign boundary (−1, `i16::MIN`, `MIN / -1`,
mixed-sign div/rem, the V=1 compare case) against rustc on both targets; README carries
the numerics table.

---

## Phase 2 — Retrieval is the product (2–6 weeks)

An agent that finds the wrong capsule in 0.25 µs loses to one that finds the right
function in 100 ms. This phase gates the "millions of cells" claim. The governing
design is [escalation-ladder.md](escalation-ladder.md): one cost-ordered ladder,
cheapest adequate mechanism wins, **calibration is a first-class deliverable**.

**2.1 Confidence-gated escalation path — first slice ✓ (the calibrated margin gate).**
Tier 1: lexical (tf-idf, `search_scored` keeps the cosine). Tier 2: static-embedding
rerank (`model2vec` potion-retrieval-32M, µs on CPU) over a **blended score**
(`0.25·tfidf + 0.75·embed` — swept; the blend dominates either signal alone: the
embedding lifts adversarial 0.31 → 0.50 while the lexical term holds direct at 0.97).
**The margin gate** answers iff `top1 − top2 ≥ θ`, else **escalates** — and θ is
calibrated, not chosen: `cell-eval tiers` sweeps the curve with the **adversarial
split as the calibration set** and picks the smallest θ whose adversarial
precision-on-answered clears the 0.75 floor. Operating point on the seed library:
**θ = 0.14** — direct answers 72% of queries at **1.00** precision, paraphrase 23%
at 0.83, adversarial answers only 15% at 0.75 and escalates the rest. The full
curve + report are checked into `cell-eval/baselines/tier-calibration.json`; re-runs catch
drift as the library grows.
*Embedder bake-off (recorded in `cell-eval/baselines/embed-bakeoff.json`):* seven
candidates through the same gate and floor. **nomic-embed-text** is the recommended
tier-2 default where an Ollama endpoint exists — best answered-coverage per
millisecond (0.91 / 0.47 / 0.46 across the splits at ~31 ms/query) and the
most-supported model in the ecosystem; **qwen3-embedding:0.6b** is the recorded
quality ceiling (ungated paraphrase 0.66, coverage 0.91 / 0.51 / 0.50, ~119 ms);
**granite-embedding** measured below both (paraphrase coverage 0.34) despite the
stack preference — the curve, not the vendor, picks. potion stays the µs offline
floor and the code default (runs anywhere, no server). Retrieval prefixes were
tested on the top three and don't change the ordering. θ is calibrated per
embedder (`OPERATING_POINTS`).

*Tier 3 — measured, and half of it banked as a negative.* The machinery exists
(`cell_eval.tier3`): discriminating-probe tables executed over a candidate set in µs
(min/max separate in one row), and `match_examples` — rung 1 scoped to the top-k, a
pure-execution filter for requests that carry I/O examples. The A/B over the
escalated residue (nomic θ = 0.05; `baselines/tier3-*.json`) answered the open
question the honest way: **raw probe tables attached to text-only escalations do not
help** — gemma-4-26B resolves the pickable residue at 1.00 from manifests alone
(probes neutral), and granite-3B at 0.85–1.00 manifests-only with probes *hurting*
(−0.11…−0.38: raw numeric tables distract a small model that can't map a text query
to expected outputs). So the ladder's text-only escalation path is "hand the top-k
manifests to the smallest adequate brain", which is nearly free; probes earn their
keep where an expected behaviour exists to match — example-carrying requests
(rung 1) and register-time discriminating-probe metadata for the admission gate.
A natural-language rendering of behavioural *differences* (rather than raw tables)
is the one follow-up worth an A/B before closing the door entirely.

*Still open in 2.1:* the ungated paraphrase target (needs better manifest text
and/or the admission gate more than a bigger model — the residual misses are
same-shape siblings). The original DoD's blanket "paraphrase ≥ 0.85 ungated" is
superseded by the ladder framing: the deliverable is *calibrated honesty per split*
— what the cheap tiers answer must be right; what they can't answer must escalate,
not guess. The **cell-potion** static-embedder experiment is specced (kill-gated)
in [cell-potion-training-spec.md](cell-potion-training-spec.md) — a *training*, not
a distillation, justified by the reflex-tier latency budget no served model meets.

**2.2 Per-cell admission gate. ✓**
`cell80 index <dir> --gate <retrieval.jsonl>` (`cell80/src/admission.rs`) admits a cell only
if (a) it's behaviourally distinct from every already-admitted cell —
`Fingerprint::agreement` (`cell80/src/fingerprint.rs`) against the already-admitted set, an
exact match refusing as a duplicate-in-metadata candidate — and (b) it carries at least one
retrieval-dataset row of its own. A refusal's report additionally names which of the
candidate's own queries also rank the duplicate #1 — the literal query-collision evidence
the DoD names, attached to the fingerprint finding rather than gated on independently (an
independent text-collision gate would refuse most legitimate new confusable-family members,
since plain lexical paraphrase P@1 is already ≈0.45 on the accepted 98-cell library).
Fingerprint comparison originally excluded state cells and 3-argument free fns (the
two-register probe bank collapsed every arity-3 cell to the same degenerate constant).
**Both exemptions were lifted 2026-07-05**: the probe bank supplies all three convention
registers (plus three rows aimed squarely at `clamp`/`min3`/`between` shapes), state
cells are driven through their named scalar fields (`field i ← probe[i % 3]`,
declaration order — identical-layout copy-paste duplicates fingerprint identically),
and comparison is guarded to the same *shape* (value-vs-value at equal arity,
state-vs-state at equal field count — cross-shape agreement is coincidence about the
assignment pattern, not evidence about behaviour). Behavioural routing gained the
structured sibling the same day: `route_by_field_examples` / MCP
`{fields:{...}, out}` / CLI `x1:3,y1:4=11` — named-field examples for the
`Struct::run` cells (and compiled plans) that register probes can't drive.
*DoD met:* running the gate over the real (then-100-cell) library (`cell80/tests/cell.rs`)
surfaced four genuine, previously-unknown behavioural duplicates — `is_gt`≡`argmin2`,
`is_lt`≡`argmax2`, `safe_div`≡`quantize`, `wrap`≡`safe_mod`, each the identical formula under
a different name for *every* `u16` input, not just the probe bank. All four were folded into
aliases (`argmin2`/`argmax2`/`quantize`/`wrap` removed, their vocabulary merged into
`is_gt`/`is_lt`/`safe_div`/`safe_mod`'s tags; see `docs/library-growth.md`), leaving a
96-cell library with one demonstrated false positive inherent to a 10-probe bank
(`snap_down`/`round_to_multiple` agree on every default probe but diverge at e.g. `x=8,
step=5`), documented in `admission.rs` as the honest scope of an `agreement == 1.0` finding:
strong evidence a maintainer should review, not proof.

**2.3 Scale the eval library.**
114 cells can't surface collision behavior. Grow to ~1,000 (generated + curated), track
P@1/adoption/composition as the library grows — the retrieval-quality-vs-scale curve is
the headline chart for the vision section.
*DoD:* eval runs at 1K cells; curve published in README.

---

## Phase 3 — Trust and the agent loop ✓ (shipped)

**3.1 Content-addressed, signed cells. ✓**
The `.cell` format (v5) carries a SHA-256 **artifact hash** over the serialized
manifest + image — the whole tool: metadata, entry, capability policy, code — and
`Cartridge::from_bytes` verifies it by default, so every loader (`exec` / `inspect` /
`index` / `serve` / the MCP `.cell` path) refuses a tampered artifact with a named
error. Optional ed25519 signing (`cell80 keygen` / `cell80 sign`) embeds a
`(verifying key, signature)` block over the hash, verified whenever present; signing
wraps the address without changing it. Pre-v5 artifacts load grandfathered.
*DoD met:* verification is on by default at the one choke point all loaders share;
`--no-verify` / `from_bytes_unverified` is the dev path; format bumped to v5. Tests:
round-trip, image + manifest tamper, forgery, v4 back-compat.

**3.2 Escalation contract (cell → host). ✓**
Two halves, both machine-readable. Dynamic: `halt(code)` in the reserved band
`0xFF00`–`0xFFFF` decodes as `Halt::Escalate` with a named reason (`needs_strings` /
`needs_floats` / `needs_io` / `needs_network` / `needs_wider_math` / `out_of_domain`)
— the band rides the existing halt trap, so it cost zero compiler surface. Static:
the manifest's `limits` field (authored via `//! limits:` in a library source header)
declares what the cell *can't* do, so a router can avoid the round-trip entirely.
*DoD met:* `Halt::Escalate` in the taxonomy; cell80-mcp returns `halt: "escalate"` +
`escalate` reason as data, documented as a typed hand-off distinct from
`{"error": ...}`; the band table lives in the ABI doc.

**3.3 Memoization cache. ✓**
`Runner::enable_cache()` (and `CellHost::set_cache` per load): determinism + the
per-run memory reset make `(entry, args)` fully determine the outcome, so `run_fast`
consults a hash map first. Only budget-independent outcomes are stored, and a hit
must fit strictly inside the caller's budget — a cached answer is byte-for-byte what
the live run would produce. The rich `run()` path stays uncached (it reports post-run
memory, which a memoized result can't faithfully fake).
*DoD met:* opt-in on `Runner`; `Report` carries `cache: {hits, lookups}`; the bench
table shows a cached call at ~46 ns regardless of workload (`add_loop` 2.2 ms live →
47,000×).

---

## Phase 4 — Codegen stage 2 (after 0–1, before feature growth resumes)

**4.1 The `Ins` layer. ✓ shipped.**
Codegen emits raw bytes (`a.byte(0x21)`) — there is nothing for a peephole to rewrite,
and the retrofit cost grows with every line added to `codegen/`. Introduce an instruction
enum between codegen and encoding *now*, behaviorally neutral, validated by
byte-identical output on the full test corpus.
*DoD:* all codegen goes through `Ins`; emitted images byte-identical to pre-refactor for
the whole suite. **Done:** symbolic operands (labels / call targets / locals *slots* —
scratch resolves at encode, so the frame loop measures once instead of emitting twice);
the hand-assembled runtime rides as `Ins::Blob`. Byte-identity proven against the
committed golden (`cell80/tests/codegen_golden.rs`: 100 stdlib cells + showcase samples
on both targets + `codegen_loop` + the u32/signed runtimes). Snapshotting the golden
also surfaced and fixed a real determinism hole: monomorphized methods were laid out in
`HashMap` order — different image per process for the same source.

**4.2 Peephole pass. ✓ shipped.**
Redundant load elimination (`LD (addr),HL; LD HL,(addr)`), push/pop pairs around trivial
RHS, dead `mask_to_width`. Measure on the size report — the win is bytes and T-states,
both metered exactly, so the pass proves itself.
*DoD:* corpus-wide size/T-state deltas published; diff suite green on both targets.
**Done:** eight rules over the `Ins` stream (seven adjacent-window + R8's window-spanning
leaf pair), label-fenced by construction,
run to fixpoint in `seal()`. Measured sites across the 100 stdlib cells (counted, not
assumed): leaf-operand pair 150, store-then-reload 30, 2-arg call tail 26, literal-add 15,
cleanups 4, dead push/pop 2 — the predicted "add is everywhere" ranking holds. Prize:
**−994 bytes (−4.3 %)** across 111 of 117 golden images. **R7 (`INC HL` strength-reduction)
added later:** `LD DE,1/2; ADD HL,DE` → `INC HL`(`; INC HL`), sized at 67 corpus sites then
measured **−497 bytes across 76 golden programs, none grew** (the flag-drop is safe — arithmetic
flags are dead; conditions recompute via `SBC`). Each rule ships a behavioural
diff case (`tests/diff/peephole.rs`) *and* a fired-proof shape assertion
(`tests/peephole_shape.rs`); the regenerated golden carries the reviewed byte deltas.
(Also fixed en route: the `is_prime` cell diverged for `n > 65025` — `d*d` wraps in u16;
bounded to `d < 256`, the `factor_count` idiom, complete for the whole u16 domain.)

**4.3 u32 completion + array elements. ✓ shipped (2026-07-04, with Cond32).**
The half-open u32 surface closed: **comparisons** (condition + value position, the
SBC-borrow chain), **saturating_add/sub**, and **array elements** — `[u32; N]`
locals (`declare_wide_array`, two slots per element) and struct fields
(`FieldDef::wide_len`, elements at `field + i*4` through the existing
`Deref32`/`Store32` nodes — no new codegen). All diff-tested, including the
sliding-wide-window shape the running-statistics pack wants. Bare
wide-array names/fields reject with "index it"; `FieldLayout::dword` deliberately
does *not* fire for wide arrays, so the cell layer never misreads `[u32; 2]` as a
scalar. u64/floats stay out (the remaining DoD half: capability-flagged rejection
diagnostics).

---

## Phase 5 — Multi-target: the cell-family (spec: [13-multi-target-spec.md](13-multi-target-spec.md))

The pre-registered "ISA attachment" clause (non-goals, below) coming due: the cell
contract — content-addressed, capability-manifested, cycle-accounted, differentially
verified — made portable across ISAs, with Z80 as backend zero of a family (RV32I/Hazard3
primary, Thumb-1/M0+ secondary; deployment driver: the SOMA reflex organ on the antweight
bot). Accepted 2026-07-10 after a repo-grounded review of the v0.1 draft; the spec carries
the revisions (WS-A re-scoped as a contract rewrite, the reference IR interpreter as a
first-class deliverable, signed re-specced as *widening* — i16 already shipped, the
cell-layer workstream WS-E, evaluation order pinned left-to-right, "no hardware floats").

**5.1 WS-A — cell-core + target descriptor. ✓ shipped (2026-07-10/11).** Descriptor
introduced, Z80 ported onto it (Spectrum48/Cell as descriptor instances), the
reference IR interpreter (A4, a standing battery oracle), evaluation order
canonicalized (A2a — the one sanctioned golden break: 4/347 programs, −8 bytes),
`SignExtend` + the width contract (A2b), i32 through the IR (A3,
interpreter-accepted; Z80 gates signed-32 instructively), and the `cell80-core`
extraction (A5 — dependency-free; rustz80 consumes behind unchanged paths; CI
publishes `cell80-z80 → cell80-core → rustz80 → cell80`).
*DoD held:* suite green throughout; golden byte-identical except A2a's pre-registered
break, landed as its own reviewed commit.

**5.2 WS-B — RV32 backend + executor (critical path). B1–B3 shipped (2026-07-11).**
`rustrv32`: the RV32 `Ins` sibling + exact encoder (encoding goldens), the
cycle-accounted RV32IM executor (RISC-V-exact M semantics, Hazard3-truth misalignment
faults, provisional cycle table pinned by test), codegen over the shared IR (family
2-byte slot ABI; native signed-32), and the battery legs: every diff program runs on
rustc + 2×Z80 + interpreter + RV32, with the RV32 data window byte-identical to the
interpreter's image. Per-file coverage ≥90%. Demo:
`cargo run -p rustz80 --example cell_family`.
*Since landed:* the **determinism-fuzz battery** (seeded random width-lattice
programs across the five-system matrix + the exact-cycle/window fingerprint) and the
**emission adversary** — GNU gas independently re-encodes every instruction shape,
CI-required on linux (teeth proven by deliberate corruption). *Owed for full 5.2:*
the Sail/spike *execution* adversary, the RV32 peephole suite, and B4 — RP2350
bring-up with the `mcycle` co-sign (the M2 demo's silicon half; the software half
runs today).

**5.3 WS-C — robo dialect.** Q-format (Q8.8 gate — live machinery today; Q16.16
stretch), the `bounded` WCET sub-dialect (static cycle bounds — new machinery), sensor
typing on the escalation-band pattern.
*DoD:* three MVP robo cells compile `bounded`, static WCET ≤ 50µs each @150MHz in
manifests, 1kHz HIL loop from synthetic sensor streams.

**5.4 WS-D + WS-E — Thumb-1 sibling; cell-layer multi-target. E1–E3(slice 1)
shipped (2026-07-11).** `.cell` v10 carries the **target id** (a host refuses a machine body
it can't run — the kernel-bank-pin posture) and the **family hash** (SHA-256 over
canonical source; sibling-target bodies share it — the formal "one cell, many
bodies"). The per-target state-address question **resolved for free**: the family
slot ABI + the shared window map make `state_addrs` window-relative and hence
target-portable unchanged. **E3 slice 1**: the first loadable RV32 cartridge —
the per-target `Body` enum (Z80 serialization byte-identical), `compile_rv32` on
the shared pipeline (canon, caps, prelude, DCE, manifest, family hash), the E1
gate host-parameterised (typed refusals at each runner's boundary, naming both
sides), `Rv32Runner` as `Runner`'s sibling, the sibling pair proven end to end.
*Owed:* E3's remainder — `CellHost` dispatch by body, rv32 typed-state I/O by
name, kernel-bank parity — then Thumb-1 (RP2040) forcing descriptor honesty per
the §2.1a ladder.
*DoD:* three-ISA demo — one family hash, three per-target artifact hashes, three cycle
certificates.

---

## Phase 6 — Model-native cells (spec: [14-model-native-cells-spec.md](14-model-native-cells-spec.md), draft v0.1)

The model provides judgment; the cells provide guarantees; the interface between
them gets fast enough to sit inside the model's own thought. Five workstreams over
the seams Phase 5 built: a **GPU target family** (WS-E — Metal first as the dev
spike, CUDA sequenced immediately before it's needed for training rewards, WGSL as
the browser demo body; one IR, the interpreter as oracle, bit-exactness gated per
target and between targets), **retrieval by execution** (WS-F — the fingerprint
probe set becomes the routing key space; the falsifiable F2 gate — probe-equipped
paraphrase P@1 ≥ 0.80 vs the 0.389 text baseline — is deliberately load-bearing:
the cheapest place to kill the thesis before training spend. **F2 PASSED
2026-07-11: 0.859 at 653 cells** — behavioural examples fused into the primary
search path (`CellHost::search_with_examples`, behaviour ranks / text breaks
ties), example sidecar generated at 98.5% coverage, zero per-query regressions;
checkpoint 21 in `cell-eval/baselines/README.md`, hard CI floor in
`cell-eval/tests/test_retrieval_examples.py`), **native wiring**
(WS-G — decode-time calls under constrained hash-vocabulary decoding, verified
numeric spans, and the Lazarus-pattern residual-stream injection), **trained
invocation** (WS-H — Toolformer-style self-labelling against an exact oracle, SFT
with hash-bound tool identities, then RLVR with cell reward organs and a
bitwise-reproducible training run owed), and the **circuit prosthetic** (WS-I —
ablate Gemma's localised arithmetic circuit, transplant a cell in its place;
either outcome is a finding). A structural note promoted from its design work:
**IR steps become the canonical family cost** (Q2), with T-states/cycles/wall-time
as per-target descriptor refinements. Notation: this spec's WS-E..I are scoped to
doc 14 (doc 13's WS-A..E are Phase 5's).

**WS-E opened 2026-07-11 — E1, E2, and E3 on Metal shipped same-day** (ledger
in doc 14 §6): `rustmsl`, the MSL sibling backend over the shared IR seam.
E1: straight-line integer cells, the R1 corner battery (shift-by-≥-width,
signed-shift saturation, `MIN/-1` div wrapping, short-circuit-hidden traps).
E2: loops/branches with the interpreter's fuel discipline mirrored tick for
tick — **IR-step parity is asserted alongside value parity** (Q2 made
operational), and a runaway loop is the same counted trap on both substrates.
E3: both batch layouts on one kernel shape — one-cell × N-inputs and the
**library × probe-set megakernel** (245 cells × 16 probes in one dispatch,
zero disagreements). Measured on M3 Max with metering on: **3.7×10⁸ evals/s**
one-cell peak (the ≥10⁸ target clears 3.7×); library launches are ~140–180 ms
flat (fixed fused-kernel cost — owed); the divergence probe confirms
WCET-friendly ≈ SIMT-friendly (wall time tracks the worst warp lane). The
batteries also **caught a real Apple Metal compiler bug** — integer divide
feeding a branch guarding stores through a thread-reference param inverts the
branch in non-inlined functions — bisected to a 10-line repro and dodged
structurally (noinline div helpers; cell functions pinned noinline so the
shipped configuration is the validated one). 245 integer value cells clean;
455 state cells await typed-state readback, f32 is E4. *Owed next:* the
library-launch fixed cost, `Body::Msl` + family-hash attestation (E6),
typed-state readback, CUDA before H3.

---

## Explicit non-goals (in the README)

- **Strings, floats-by-default, I/O, network** — that's the escalation path (3.2), not
  the roadmap. The moment the ISA chases general applicability, the differentiation vs
  Wasm evaporates.
- **JIT / speed chasing** — Wasm wins warm compute; the moat is exact metering,
  auditability, byte-scale artifacts, and determinism. Protect the moat, don't race the
  loser's race.
- **ISA attachment.** The Z80 is an implementation detail *behind* the `.cell` ABI, not
  the product: the contract is determinism + exact metering + capability gating, and the
  conformance suites (1.53M SingleStepTests vectors, ZEXDOC) are why the Z80 currently
  earns its seat. Codegen emits symbolic `Ins`, encoded once at the end — so a second
  backend target, if the contract ever needs one, is an encoder swap behind a stable
  artifact format, not a rewrite. The format survives the chip. *(This clause came due:
  Phase 5 / [13-multi-target-spec.md](13-multi-target-spec.md). The review found
  "encoder swap" was optimistic — the IR's contract bakes in 16-bit/HL:DE:BC/2-byte-slot
  assumptions, hence WS-A's descriptor rewrite — but the artifact-format half held.)*

The failure mode that would dissolve everything is feature drift toward a worse Wasm; the
implicit bet is that a narrow thing that's *actually true* beats a broad thing that's
approximately true, in a field currently drowning in the latter.

## Sequence summary

Phase 0 was a single "close the determinism contract" commit-set and gates everything.
Phase 1 makes the compiler usable by its actual author. Phase 2 is the product bet and
the long pole — 2.1 (the calibrated ladder + the cell-potion floor) and 2.2 (the
admission gate) are shipped; 2.3 (the scale curve) is the remaining open item, gated
behind 2.2 by design since growing to 1K cells before ingest gating would manufacture
the collision problem — now that the gate exists, 2.3 can proceed.
Phase 3 shipped alongside, as designed. Phase 4's Ins layer + peephole landed before
the next dialect expansion, as required. Phase 6 (model-native cells, doc 14) is
drafted with its own falsification ladder — the F2 routing gate precedes any
training spend, and passed 2026-07-11 (probe-equipped paraphrase 0.859 vs the
0.80 bar; checkpoint 21); WS-E opened the same day and ran E1→E3 on Metal
(`rustmsl`: 245 integer value cells bit-exact vs the interpreter on values
*and* IR steps, the library×probes megakernel, 3.7×10⁸ evals/s measured).
Phase 5 (multi-target) opened 2026-07-10;
WS-A shipped whole in two days, WS-B's compiler is live in the battery behind a
fuzz battery and a CI-required independent emission adversary, and WS-E's identity
half (v10 target id + family hash) is in the artifact — the remaining critical path
to the public M2 demo is hardware (the RP2350 `mcycle` co-sign), with WS-E3's
per-body host as the remaining product gap.
