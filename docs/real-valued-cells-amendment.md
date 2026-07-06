# The F-waves — owned IEEE binary32, an amendment to the real-valued-cells policy

*Status: **draft for registration, 2026-07-07.** Amends `docs/real-valued-cells-spec.md`
Part 1 (the float policy) and supersedes its Wave 3 in one respect (§F2 below). Companion
to `docs/10-dialect-semantics.md` (gains an f32 tier), `docs/escalation-ladder.md`
(one code re-scoped, two added), and the canonicalization pass (one hard interaction,
§F0.6). Nothing here touches the M3 critical path; the PAL baseline still goes first.*

**One sentence:** the float ban narrows from "no floats" to **"no floats we don't own"**
— IEEE binary32 enters the dialect as owned integer softfloat kernels whose golden
reference for basic operations is rustc itself, transcendentals arrive later as owned,
ULP-bounded, demand-gated implementations, and libm stays banned permanently.

---

## Part 0 — the amendment, and why it is not a reversal

The banked decision (real-valued-cells spec §1.1) recorded the reason floats were
rejected: **rustc lowers transcendentals to platform libm, libm differs across hosts**,
and a cell whose reference behaviour is host-dependent breaks the differential oracle,
content addressing, and fact-file portability at once.

Read precisely, that rationale bans a *dependency*, not a number format. IEEE 754
splits in two:

- **Basic operations** — add, sub, mul, div, sqrt, comparisons, conversions, rounding
  — are bit-specified: correctly rounded, one legal result, identical on every
  conforming implementation. rustc's `f32` basic ops are IEEE on every supported
  target, **no libm involved**. For this tier, release-mode rustc *is* a portable
  golden reference, and `check!` works unmodified in spirit.
- **Transcendentals** — sin, cos, exp, log, pow — are *not* bit-specified. rustc calls
  platform libm; answers differ across hosts. This half of IEEE is where the banked
  rationale actually lives, and it stays banned in its libm form permanently.

So the amendment: implement binary32 ourselves, in integer Z80 code — sign/exponent/
mantissa unpacking, alignment, round-to-nearest-even, repacking. Integer arithmetic all
the way down, therefore: bit-exact across hosts, content-addressable, fact-file-portable,
and differentially testable. The substrate has prior art — the ZX Spectrum ROM shipped a
complete software floating-point calculator on this same core in 1982 — but the format is
**IEEE binary32, not Sinclair's five-byte format**, because the oracle needs rustc
agreement, and the cost prior from that era (fmul in the low thousands of T-states) says
float ops fit the µs envelope. Costs are measured and published, not assumed (H-F2).

**What stays banned, so this is surgical:**

| banned | reason |
|---|---|
| platform libm, in any form | the original rationale, now aimed at its true target |
| rounding modes other than RNE | per-op modes are a determinism hazard; no customer |
| flush-to-zero / DAZ | diverges from rustc (which handles subnormals); breaks the oracle |
| f64 | doubles kernel cost with no named customer; demand-gated like everything else |
| `mul_add`/fma | IEEE-specified (single rounding) so *eligible*, but 48-bit intermediate mantissa; deferred to demand as an F1 stretch, not banned |

**Escalation-code re-scope:** `0xFF02 needs_floats` currently means "floats at all →
host." Post-F0 it means "float capability not yet in dialect" — transcendentals before
F2 lands, f64, anything libm-shaped. The coverage map's `host_only` trig rows keep
routing to it until F2 changes their answer. Two codes are added at the *cell boundary*
(§F0.4): `float_overflow` and `float_domain`.

---

## Part 1 — the tier map, restated once

| tier | representation | exactness class | customer |
|---|---|---|---|
| fractions | `(num, den)` u32 | `exact` | GSM/word-problem rationals — **unchanged, still the campaign claim** |
| Q-format | Q8.8 / Q16.16 | `exact_at_scale` / `approximate` | agent-control reflexes, cheap bounded real |
| **f32** | IEEE binary32 in u32 | `correctly_rounded` (basic ops) / `approximate` (F2) | **physics, dynamic-range math** |
| host math server | anything | — | symbolic, matrices, distributions, unbounded |

New exactness-taxonomy value: **`correctly_rounded`** — deterministic and bit-specified,
inexact relative to ℝ by exactly one RNE rounding per operation. Distinct from
`approximate` (declared ULP bound, F2 transcendentals) and never spelled "exact."

One line enters the extraction-prompt conventions when F0 lands: *fractions for exact
rational reasoning; Q for bounded-range reflexes; f32 only when the problem is genuinely
real-valued with dynamic range.* Any drift of GSM extraction toward f32 is a regression
to catch in review — H-M2's "exact rationals by default" differentiator depends on it.

---

## Part 2 — the waves

### F0 — the kernel five + the f32 tier (the prerequisite wave)

**Kernels**, joining the shared inline-foldable family (`mul_checked_u32`, `q_mul`):
`fadd`, `fsub`, `fmul`, `fdiv`, `fsqrt`. Binary32 lives in u32 (two u16 words on the
core); `fmul`'s 24×24 mantissa product is the word-split-partials machinery `q_mul_q16`
already uses, wearing an exponent; `fdiv`/`fsqrt` by integer restoring/Newton methods
with correct RNE via guard/round/sticky bits. **Subnormals handled exactly** (gradual
underflow, as rustc does), signed zeros preserved, Inf/NaN propagated per IEEE.

**Semantics inside a cell: bit-identical to rustc f32, including NaN/Inf propagation.**
No in-cell trapping on overflow or invalid — trapping mid-expression would diverge from
the golden reference. Comparisons follow Rust exactly, including `NaN != NaN` and, where
`min`/`max` cells are authored (F1), **Rust's `f32::min`/`f32::max` semantics (NaN
ignored, other operand returned) — not IEEE-2019 `minimum`/`maximum`**, because the
oracle's reference is rustc, and this footgun is worth one loud sentence.

**Dialect surface:** `f32` type on lets/params/returns; float literals convert at
compile time via correct decimal→binary32 rounding (RNE — same algorithm rustc uses, so
literal bits match the reference); arithmetic on f32 operands routes through the kernels
exactly as checked arithmetic routes through checked kernels. The plan unit system gains
a **representation tag** (`repr: int | q8 | q16 | f32`) orthogonal to dimension — the
renderer rejects mixed-repr ops without an explicit conversion (F1 cells), the same
type-flow discipline that already guards dimension and scale.

#### F0.4 — the boundary contract (escalate-not-lie, applied where it belongs)

IEEE's silent NaN propagation is precisely the silent-wrong-answer disease. The dialect
keeps IEEE semantics *inside* the cell (oracle fidelity) and applies the project's
discipline *at the boundary*:

- **Canonical NaN at the cell boundary and in the fact file** — one blessed quiet-NaN
  bit pattern (`0x7FC0_0000`), the same move WASM makes for the same reason (hardware
  NaN payloads differ across x86/ARM). The oracle's comparison treats NaN-class as
  equal-after-canonicalization; every stored fact is canonical.
- **`finite_result` manifest contract** — a cell that declares it halts typed at return
  if the result is non-finite: `float_overflow` (±Inf), `float_domain` (NaN). Declared
  per cell, default **on** for library cells (a physics cell returning Inf is an
  escalation, not an answer), opt-out-able for cells whose *job* is IEEE plumbing
  (`is_nan`, `is_finite`, classification predicates).

#### F0.5 — the oracle, extended not weakened

`check!` for f32 cells asserts **bit equality** (post-NaN-canonicalization) between both
targets and release rustc, on: an enumerated edge bank (±0, subnormal min/max, mantissa
LSB neighbours, exponent boundaries, ±Inf, NaN, values straddling rounding ties — the
Berkeley TestFloat case families, imported as a harness-side bank, never as a runtime
dependency) plus a seeded random bank. Bit equality, not tolerance — tolerance is how
float testing lies to itself, and correctly-rounded ops don't need it.

#### F0.6 — the canonicalization interaction (the one hard constraint)

**Float arithmetic is not associative, and the canon pass must know it.** Reordering
*independent* ops in the dataflow DAG is safe (results don't interact); **algebraic
rewrites of a single chain are not**: defer-division, mul-chain reassociation, and
exact constant folding all change f32 bit results. Rule, stated once and enforced in
`canon.rs`: **algebraic rewrites apply to integer/fraction/Q tiers only; f32 chains
canonicalize structurally** (alpha-rename, slot order, dead-let removal) **and fold
constants only in source evaluation order with RNE** — the fold must produce the bits
the runtime would have. A canon test asserts a deliberately reassociation-sensitive
f32 chain survives the pass bit-identically. This constraint is why F0 cannot be "just
add a type": it touches the pass that owns hashing.

**F0 deliverables:** kernel five + dialect surface + repr tags + boundary contract +
oracle banks + canon guard + **the measured cost table** (T-states and bytes per kernel,
single-site fold deltas) published in the dialect doc the way the checked-kernel
factoring published −1683 B.

### F1 — conversions, rounding, bit-ops (small, mostly trivial)

`int_to_f32` / `f32_to_int_trunc` (typed `float_domain` on out-of-range/NaN, matching
the boundary contract rather than Rust's saturating cast — divergence from rustc noted
in the manifest, it is boundary behaviour not arithmetic), `q16_to_f32` / `f32_to_q16`,
`floor` / `ceil` / `trunc` / `round` (bit-specified, oracle applies), `abs` / `neg` /
`copysign` (pure bit ops), `is_nan` / `is_finite` / `is_subnormal` (classification,
`finite_result` off), `fmin` / `fmax` (Rust semantics, per the F0 sentence). Stretch,
demand-gated: `fma` (IEEE-specified, oracle-eligible, 48-bit intermediate — priced when
a customer names it).

### F2 — owned transcendentals (demand-gated; supersedes Wave 3's representation choice)

The real-valued-cells spec's Wave 3 registered CORDIC trig at Q16 behind an escalation
counter. **The gate survives unchanged; the representation question is now open** — one
trig decision, not two competing packs: when the counter crosses threshold, the customer
that fired it chooses the tier (physics demand → f32; agent-reflex demand → Q16). No
trig is authored twice.

If f32 wins: `sin_f32` / `cos_f32` / `atan2_f32` / `exp_f32` / `log_f32` as **owned**
implementations — payne-hanek-lite range reduction + minimax polynomial over the F0
kernels. Contract structure from the Q-spec, one honest narrowing stated loudly:

- **Determinism oracle:** both targets agree bit-exactly — unchanged.
- **rustc check: explicitly N/A** — rustc's answer here *is* libm's answer, which is
  the thing being escaped. The manifest says so per cell.
- **Accuracy oracle:** declared ULP bound (`//! accuracy: <= 1 ulp over [domain]`),
  verified harness-side against MPFR ground truth. CORE-MATH and RLIBM serve as
  reference implementations and range-reduction prior art (they prove ≤0.5 ULP f32
  correct rounding is achievable — a target to price, not a requirement to assume).

### F3 — the physics pack (the customer that justifies the family)

Named, demand-sourced from `chuk-mcp-physics` (Rapier) and SOMA rather than a taxonomy:
`verlet_step`, `impulse_1d`, `drag_force`, `spring_damper_step`, `kinetic_energy`,
`elastic_collision_1d`, `lerp_f32`, `clamp_f32`. The interop claim that makes this a
demo class and not just a pack: **Rapier is f32-native and ships its own
`enhanced-determinism` feature for cross-platform lockstep** — cell-side owned f32 means
a cell can *verify a Rapier world's arithmetic bit-for-bit*, putting the physics MCP and
the cells on one shared deterministic-f32 contract. Verified-physics-step cells checking
a simulation is a genuinely new demonstration for the channel and for SOMA's fast clock.
Each cell pays admission (retrieval rows, `finite_result` on, accuracy contracts where
any approximation enters).

---

## Part 3 — registered hypotheses and kill criteria

- **H-F1 (the oracle holds):** every F0/F1 op is bit-identical to release rustc f32
  across the edge and random banks, post-NaN-canonicalization, permanently — a CI gate.
  *Kill:* any reproducible mismatch that is not a rustc bug is a kernel bug; no f32
  cell is admitted while one is open. There is no "close enough" band.
- **H-F2 (the cost envelope):** kernel T-state costs keep single-op f32 cells inside
  the µs dispatch envelope; predictions registered before measurement — fadd/fmul low
  thousands of T-states, fdiv/fsqrt higher. *Kill:* fdiv/fsqrt blow the envelope →
  they become escalation-priced ops with their cost stated in the manifest, and
  division-heavy cells route to fractions or host; the pack narrows, stated openly.
- **H-F3 (kernels fold):** single-site kernel inlining is byte-neutral, ≥2 sites net
  positive, like the checked family. *Kill:* the wide-slot fold gap from H-Q2's clause
  applies here identically — fix the inliner before growing F1.
- **H-F4 (canon safety):** the reassociation guard holds — no algebraic rewrite ever
  fires on an f32 chain; the sensitive-chain canon test is a permanent CI member.
  *Kill:* a guard breach is a hashing-correctness bug and blocks release, because it
  silently forks content addresses from runtime behaviour.
- **H-F5 (demand, inherited):** the transcendental gate = Wave 3's counter, unchanged.
  *Kill:* counter silent → F2 stays unauthored, F3 ships restricted to
  polynomial-expressible physics (which is most of the named pack), and the negative
  is banked.

## Honest limits

f32 is correctly rounded, not exact, and this spec never lets the two blur — the exact
tier remains fractions, and the taxonomy word for this tier is `correctly_rounded`
precisely so nobody shortens it to "exact" in a README. The oracle narrows at F2 and
says so per cell: owned transcendentals are checked against MPFR and each other, not
against rustc, because escaping libm was the whole point. Softfloat costs real cycles —
if the measured table says fdiv is expensive, the honest response is pricing it, not
hiding it, and H-F2's kill clause exists because the µs pitch matters more than pack
breadth. The canon guard makes f32 hashing *sound* but coarser than integer hashing
(structurally-identical float chains that differ only algebraically will not
precipitate together — correctly, because their bits differ). And the physics pack's
bit-for-bit Rapier claim holds only while both sides keep their determinism promises;
it gets tested against a real Rapier trace before it is ever said in a video.
