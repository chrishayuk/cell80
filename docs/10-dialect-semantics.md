# Dialect semantics — what an accepted program means

The spec-07 addendum the determinism contract rests on: exactly what the restricted-Rust
dialect *guarantees*, on both targets, and what the differential oracle does and doesn't
check. The rule of thumb: **an accepted program computes what rustc computes** — anything
the compiler can't make true it must reject, never approximate.

## Types

`u8`, `u16`, `i16`, `u32`, `bool`, and — since the F-wave amendment — `f32`
(owned softfloat, its own section below; explicit `f32` suffix required, no
implicit conversions). `i16` is two's complement in a single slot:
add/sub/mul and the bitwise ops share the unsigned bit patterns (wrapping), while
**comparisons order by sign**, **divide truncates toward zero** (the remainder takes the
dividend's sign — rustc semantics; `i16::MIN / -1` wraps to `i16::MIN`), and **`>>` is an
arithmetic shift** (the sign propagates). Casts between `i16` and `u16`/`u8` are
bit-preserving; `i16 as u32` (a sign extension in Rust) is rejected — take the bits
explicitly (`x as u16 as u32`). Negative literals need the suffix (`-5i16`), and unary
`-` needs a signed operand.

**Fractional values default to integer conventions, not floats**: exact rationals
(the fraction cells) and Q-format fixed point (a Q8.8 weight is a `u32`/`u16` with an
implied point — multiply then shift, `(a * w) >> 8`) stay the first choice; `f32` is
the explicit opt-in tier for genuinely real-valued, dynamic-range work (owned
softfloat — bit-exact and cross-target deterministic like everything else; see its
section). Unsuffixed decimal and `char` literals are rejected with instructive
errors. The implied point can be declared
structurally with a **`//! scale: N`** header (the fractional-bit count — `q_mul`/`q_div`
declare `8`), which rides in the manifest (`.cell` v7) so a host reads the cell's values
as `raw / 2^N` without inferring the convention from the summary.

## Const items and const data

Top-level `const` items are in the dialect. **Scalar consts** (`u16`/`u8`/`i16`/`bool`;
the initializer an int/bool literal, a negated `i16` literal, or an earlier scalar const)
substitute as literals at every use site — they occupy no image bytes. **Data consts** —
`[u8/u16/i16; N]` arrays, `&str`, struct literals (`Tile { rows: […] }`), and
`[Struct; N]` tables — are **byte-packed into the image after the code**, each at its own
symbol, little-endian in declaration order. Note this packed layout is *not* the 2-byte-
per-slot state layout: a `[u8; N]` const is `N` bytes, and a packed struct element's
stride is its packed size. Data consts have their own DCE — only consts referenced by a
kept function are laid.

Addressing: `&CONST` evaluates to the data's address; `&CONST[i]` addresses element `i`
at the packed stride (literal indices are bounds-checked at compile time); a scalar read
`CONST[i]` loads the element; and a `&[u8; N]` parameter receives such an address and
reads packed elements through the pointer — so a table helper is real Rust both ways,
diff-tested against rustc.

**String literals** are const data, not strings-the-type: a `"…"` literal is interned
(deduplicated by content), stored **length-prefixed** (a little-endian **u16** length,
capped at 1024 — the Phase S wire format, `docs/11-machine-text.md` §1), and
**evaluates to its address** — the length's low byte is `peek(s)`, high byte
`peek(s + 1)`, and byte `i` is `peek(s + 2 + i)`. There is still no `String`, no heap,
and no string operations; the literal is addressable bytes for the host or screen
routines to consume. (Pre-Phase-S the prefix was a single length byte — a consumer
routine written against that convention, e.g. an SDK text renderer, reads the length
low byte identically for strings under 256 bytes but must skip **2** prefix bytes,
not 1.)

**Byte-string literals** (`b"…"`) and `const B: &[u8; N] = b"…";` are the raw
sibling: packed bytes with **no** prefix (the `[u8; N]` type carries the length),
deduplicated, evaluating to their address. `b'a'` is a `u8` value literal.

**`&str` parameters** (Phase S §2.1) occupy one register — the address of a
length-prefixed buffer — and expose exactly four methods, each real Rust with
identical semantics (diff-tested via `check_str!`): `s.len()` (a 16-bit load at
`s`), `s.is_empty()`, `s.as_bytes()[i]` (a byte load at `s + 2 + i`, **no bounds
check** — guard with `i < s.len()`, the library idiom), and
`s.is_char_boundary(i)` (the exact `str::is_char_boundary` truth table, including
`i > len` ⇒ `false`; the byte read short-circuits behind `i < len`). Direct
indexing `s[i]`, slicing `&s[a..b]`, `chars()`, and any `String`-producing
operation stay out with steering diagnostics; strings are read-only — output
builds into `[u8; N]` state fields (§2.3 of the Phase S spec).

## `if`/`match` as values

`let x = if c { a } else { b };` and `match`-with-value-arms are accepted in `let`,
assignment, `return`, and tail position (with nesting and `else if` chains), lowering to
the statement form through the destination slot. A value-`if` needs an `else`; a
value-`match` needs a `_` arm; every branch must end with the value (no trailing `;`) —
each violation is its own instructive compile error.

`match` arms accept integer/byte literals, enum variants, **range patterns**
(`0..=9 =>`, exclusive `0..10 =>`, byte bounds `b'a'..=b'z' =>`) and **or-patterns**
(`1 | 2 =>`, ranges allowed inside the or-list) — everything lowers to the existing
if-chain over the scrutinee temp; a lone equality stays a direct comparison, compound
arms materialise a `0`/`1` test. Range bounds are non-negative literals, which keeps
the unsigned comparison exact for `i16` scrutinees. Bindings, tuple patterns, and
open-ended ranges stay out with steering diagnostics. (The `range_pattern` repair
class graduated to a feature with this — its rows left the repair dataset.)

## Arithmetic

All integer arithmetic is **wrapping** (mod 2^width) — the semantics of release-mode
rustc, which is what the differential oracle runs. `u8` ops mask to 8 bits, `u16` to 16,
`u32` to 32. `wrapping_add`/`wrapping_sub`/`wrapping_mul` are accepted and identical to
the bare operators (everything wraps). There is no overflow trap and no checked
arithmetic.

**`saturating_add`/`saturating_sub`/`saturating_mul`** are accepted for `u8`/`u16`
**and `u32`** — real Rust, oracle-checked — lowering to branch-free mask clamps
(`s | (0 - overflow)` for add, `d & (0 - in_range)` for sub, a widened product with
a high-part test for 8/16-bit mul; the u32 clamps ride the 32-bit compare below).
`u32 saturating_mul` needs no 64-bit product: with the wrapped product `p = a * b`,
overflow ⇔ `a != 0 && p / a != b` (the classic post-hoc check — the division is one
extra trap of honest cost, short-circuited behind the zero test). The clamp re-reads
its operands, so effectful operands (a call in operand position) are rejected with a
"bind it first" message. `i16` saturating clamps to a *signed* range the mask trick
doesn't express — rejected instructively. This removes the compiles-but-wraps class
in machine-authored cells: `a.saturating_add(b)` now means what it means in host
Rust.

**`u32` comparisons** are in, condition and value position alike: `if a < b`,
`while total < cap`, `(a == b) as u16` — unsigned (the dialect has no `i32`),
oracle-checked across the word seams. The lowering computes `l - r` through the
32-bit `SBC` chain and reads the final borrow as `l < r` (equality ORs the
difference's four bytes) — branch-free, no labels, no traps. In condition position
the `0`/`1` materialises and branches on `!= 0`, the same compound-condition shape
`&&`/`||` use. The word-split idiom (`hi != hi || lo < lo`) retires; the
`q_max`/Q-clamp family writes `if a < b { b } else { a }` as intended.

**`[u32; N]` arrays** work as locals and struct state fields: two slots per
element, element access through the wide load/store nodes at `base + i*4` —
reads, writes, `[v; N]`/`[e0, e1, …]` init, all oracle-checked (including the
sliding-window accumulate shape). A bare wide-array name or field is not a value
(index it); `[u32; N]` fields are not name-addressed by the cell layer (2N slots,
never mistaken for a scalar `u32`). Wide *elements* don't change the call rule:
`u32` still never crosses a call boundary.

Shifts: a constant amount unrolls; a runtime amount shifts by the **low byte** of the
operand, and a count ≥ the width shifts out to `0` (matching rustc's behaviour for the
in-range counts the oracle tests; rustc panics on out-of-range constants in debug — the
dialect does not, it zeroes).

**The std bit methods** are in for `u8`/`u16`, all rustc-identical and oracle-checked.
`count_ones`, `leading_zeros`, `trailing_zeros` call tiny appended kernels
(`__bits_*` — plain Z80 shift loops, identical bytes on both targets, honest cycles,
**no traps**); the u8 variants derive from the u16 kernels (`lz - 8`; `tz` of
`x | 0x100`). `rotate_left(k)`/`rotate_right(k)` desugar to `(x << k') | (x >> w-k')`
with `k' = k % width` — constant amounts unroll, runtime amounts (std's `u32`
narrows freely) mask and shift; `swap_bytes` is `(x << 8) | (x >> 8)` (u8: identity).
The desugars re-read operands, so effectful operands steer to a `let` binding. The
counting trio returns 16-bit values where std returns `u32` — every in-range use
agrees, and the value never exceeds 16. `u32`/`i16` variants reject instructively.

## Divide by zero

rustc panics; a cell cannot. The two targets answer differently, **by design**:

| target | `/ 0`, `% 0` | why |
|---|---|---|
| `Cell` (default policy `DivByZero::Halt`) | the run stops with `Halt::DivByZero` | a garbage quotient must not flow onward into scoring |
| `Cell` (opt-in `DivByZero::Saturate`) | `q = 0xFFFF` / `0xFFFF_FFFF`, `rem = dividend`, run continues | the legacy bounded-garbage behaviour |
| `Spectrum48` | saturates like the opt-in | real hardware has no trap surface to halt through |

The policy is part of the artifact (an image flag; absent = halt, so pre-policy images
load safe). The divergence is deliberate and documented here rather than papered over:
the authentic target physically cannot halt-on-trap. The differential oracle never
exercises `/ 0` (rustc would panic), so this corner is covered by the runner tests, not
`check!`.

## Evaluation order

Left-to-right, with one deliberate exception: the **right operand of `-`, `/`, `%`, and
16-bit `*` is evaluated first** (an operand-ordering artifact of the accumulator scheme).
This is observable only by side-effecting operands (`poke`, function calls with effects);
pure expressions — the overwhelming cell shape — cannot tell. Write side-effecting
operands as separate statements if order matters. `&&`/`||` short-circuit exactly as in
Rust. A `for` range's bounds are evaluated once, before the loop.

## Recursion: rejected

Stage 1 gives every function **static local slots** — there is no stack frame, so a
recursive call clobbers its caller's locals. Before the gate, the slot-after-call
factorial compiled and silently returned 1 instead of 120, while tail-shaped recursion
"worked" by riding the hardware stack. Any cycle in the call graph (direct or mutual) is
now a compile error naming the cycle:

```
recursion is not supported (Stage 1: static locals) — rewrite as a loop (cycle: fact → fact)
```

## Calls

At most **3 registers of parameters** (the `HL`/`DE`/`BC` convention; no stack args), at
most 3 tuple-return values (same registers).

**One u32 may cross a call boundary** (the Tier-2 convention — deliberately minimal so
the register contract stays intact):

- **At most one `u32` parameter, and it must be first**: it rides `HL:DE` (low word in
  `HL` — the same pair every u32 computes in), leaving `BC` for at most one more 16-bit
  parameter. The prologue is unchanged by construction: registers store to consecutive
  slots, and a wide slot pair *is* `[low, high]`.
- **A `u32` return** rides `HL:DE` likewise. A wide return can't be part of a tuple.
- Everything else stays out: a second `u32` param, a `u32` in a non-first position, a
  `u32` on a method (`self` holds `HL`), a wide argument to a 16-bit slot — each a
  steering compile error, never a truncation.

This is what makes shared kernels modular: `fn scale(acc: u32, k: u16) -> u32` is
callable from many sites without the inliner's help, so an accumulate/step family
stops hand-inlining its widen-multiply-shift.

**Two u32 params** are also in — the composition debt that forced every fraction
cell to inline its own Euclidean GCD loop is paid: `fn gcd_u32(a: u32, b: u32) ->
u32` is callable. The convention mirrors the house `__mul32` precedent (its left
operand has always travelled on the stack): the **first** u32 rides `HL:DE`, the
**second** rides the **stack** (caller pushes `hi` then `lo`, then `CALL`s), and
the **callee pops it** in its prologue (`POP ret / POP lo / POP hi / PUSH ret`) —
no caller cleanup, no SP-relative addressing, ~6 prologue instructions. An optional
third **u16** may follow in `BC` (it was idle). Allowed signatures grow to:
`(u32)`, `(u32, u16)`, `(u32, u32)`, `(u32, u32, u16)` — the u32s must be the
leading params. Still no recursion, so params land in static slots as ever.

**Composition costs zero bytes.** A wide kernel called from *one* site folds into
its caller (the inliner treats a `u32` param as its two slots, binds it with an
`Assign32`, and substitutes a pure wide arg like any scalar), so a shared kernel
called once is byte-identical to the loop it replaced — the standalone-cartridge
model bundles the prelude per cell, so this is what keeps `gcd_u32(self.n, self.d)`
from costing more than the hand-inlined loop. Two refinements make it *exact*:
*result-aliasing* lands the kernel's returned local straight on the caller's result
slot (no trailing copy — the reduced `x` becomes the `let g`), and an *effect-free
arg* (a `self.field` read) substitutes without a bind/copy whenever the kernel writes
no memory. A kernel called from *many* sites stays a real call (the convention above)
and is shared — a net byte win exactly when the same kernel is used more than once in
a cell. Measured: the ten fraction cells fold `gcd_u32` byte-for-byte; four unrelated
cells that call a prelude helper once got *smaller* (−21 bytes) from result-aliasing
alone. The bigger lever is `mul_checked_u32` (the wrapping-multiply + overflow-escalate
idiom, `docs/12`): factoring the 41 hand-inlined copies across 30 cells into one shared
kernel cut **−1683 bytes** — cells that check one product fold to neutral, cells that
check two or three (fraction add/sub/average) drop 116–261 bytes each — with behaviour
byte-identical on the differential battery. `add_checked_u32`/`sub_checked_u32` complete
the checked trio (the `wrapping_add`+`s < a` and guard-then-subtract idioms); those save
less — the add carry-chain is cheap next to the `mul`'s `div32` — but still fold 30-plus
copies into two kernels. All are **u32-only**: the same idiom on `u16` values keeps its
own (narrower) overflow boundary and is left inline.

The *math* residual stands: a Q16.16 `q_mul` needs a 64-bit intermediate the
substrate doesn't have, so that kernel remains word-split partials
(`ah·bh<<16 + ah·bl + al·bh + (al·bl)>>16`, each 16×16→32) however its operands
arrive — a **state cell** (named `u32` fields in, `u32` out) stays the recommended
pattern for kernels that are wide in *math*, not just in arity. Host note: a u32-param **entry** is drivable from the host by
passing the two words as `args = [low, high]` — the register convention makes the
split exact.

A 16-bit argument in a wide slot zero-extends (the value rustc would infer for an
in-range literal); a wide value in a 16-bit slot stays an error.

## The owned-softfloat family (F0)

The F-wave amendment (`docs/real-valued-cells-amendment.md`) adds IEEE binary32 to
the dialect as **owned** integer softfloat — the kernel five `fadd`/`fsub`/`fmul`/
`fdiv`/`fsqrt`, the comparison trio `feq`/`flt`/`fle`, and helpers `f32_shr_jam`/
`f32_pack` (`rustz80::F32_KERNELS`; the text lives rustz80-side so the differential
bank tests the same string cells compile). Semantics are bit-identical to rustc `f32`
basic ops — full subnormals, signed zeros, RNE only — verified by
`tests/diff/f32_ops.rs` on both targets over an edge bank and a seeded random bank,
with **bit equality** post NaN-canonicalization (every kernel-produced NaN is the
canonical `0x7FC0_0000`). The kernels never `halt()` — the boundary contract
(`finite_result`, codes `0xFF07`/`0xFF08`, docs 09) is the cell's job, not the
kernel's, because an in-kernel trap would diverge from the golden reference.

**The `f32` type** rides on top: `f32` params/lets/returns (bits in the wide `u32`
convention), `f32`-suffixed literals (compile-time decimal→binary32, RNE — the same
correctly-rounded parse rustc applies, so literal bits match the oracle), operators
`+ - * /` routing to the kernels, all six comparisons through the trio (Rust
semantics: NaN false everywhere ordered, `-0.0 == 0.0`; `!=` negates `feq`, `>`/`>=`
swap onto `flt`/`fle`), unary `-` and `.abs()` as pure sign-bit ops, `.sqrt()` as the
fifth kernel. The needed kernels **auto-append** at lowering (name-collision-safe
with the cell prelude's copies). Three deliberate boundaries: **unsuffixed decimals
are not f32** — `12.5` stays the canon pass's exact-decimal lane (the fraction
tier); **f32 never mixes with integers** — every cross (operators, comparisons,
bindings, call boundaries, casts, returns) is a clean compile error, no implicit
conversion existing at all until the F1 kernels (this repr discipline is the hard
gate model-composed float cells depend on); and **f32 struct fields are rejected**
until the state ABI gains `Ty::F32` (F1). Because float ops lower to *calls*, the
canon pass's algebraic rewrites cannot touch float chains even in principle — and a
guard enforces it anyway (`touches_f32` → structural-only), with H-F4 pinned in CI
(`canon.rs::f32_chains_never_reassociate`).

Measured cost (2026-07-07, single-kernel driver cell, baseline-subtracted; `fmul`'s
four u32 multiplies ride `ED FE` traps charged ~4 T each, so its honest cost pairs
`cycles` with `trapped_ops` — the others are pure shifts/adds, authentic cycles):

| kernel | T-states | traps | image bytes (kernel + helpers + driver) |
|---|---|---|---|
| `fadd` | 10,854 | 0 | 3,164 |
| `fsub` | 12,586 | 0 | 3,225 |
| `fmul` | 11,227 | 4 | 3,429 |
| `fdiv` | 36,644 | 0 | 3,332 |
| `fsqrt` | 53,219 | 0 | 2,254 |
| `ftrunc` | 2,775 | 0 | 540 |
| `ffloor` | 3,113 | 0 | 815 |
| `fround` | 3,380 | 0 | 687 |
| `fmin` | 6,409 | 0 | 1,235 |
| `int_to_f32` | 6,392¹ | 0 | 1,044 |
| `q16_to_f32` | 11,526¹ | 0 | 1,044 |
| `f32_to_int_trunc` | 14,895¹ | 0 | 903 |

¹ value-dependent: the conversion normalize/strip loops shift by 1 up to 31/23 steps.

**Banked negative** (2026-07-07): a barrel-decomposed `f32_shr_jam` (test-and-shift by
16/8/4/2/1) measured *worse* than the per-bit loop on the typical profile — fadd 12,406
vs 10,854 T-states and +636 B — because real alignments are small (same-magnitude adds
shift 0–2) and the `n > 31` early-out already caps the tail. The loop stays; the
prediction that the barrel was "the obvious cost lever" is recorded as measured-false.

**The F1 surface** (all oracle-banked like F0): conversions
`int_to_f32`/`q16_to_f32`/`f32_to_int_trunc`/`f32_to_q16` — *typed builtins*, the only
sanctioned int↔f32 crossings (`as` stays rejected); the `f32_to_*` pair halts typed
`0xFF08 float_domain` on NaN/out-of-range, deliberate boundary behaviour rather than
rustc's saturating cast (the family's one documented rustc divergence). The rounding
family `.floor()`/`.ceil()`/`.trunc()`/`.round()` (`round` = Rust's half-away-from-zero,
not RNE). `.min()`/`.max()` with Rust's "NaN is missing data" semantics plus two
deterministic pins where rustc itself is unspecified: `-0 < +0`, and a *signaling* NaN
is ignored like a quiet one (LLVM's minnum quiets sNaN on ARM but not via libm —
host-dependent inside rustc, so the oracle excludes those zones and the pins are CI
members). Pure-bits sugar with no kernel: `.abs()`, `.copysign(b)`, and the
classification trio `.is_nan()`/`.is_finite()`/`.is_subnormal()` (inline compares).
State cells carry `f32` fields (`Ty::F32`, docs 09). The sandboxed code cap moved
4096 → 8192 with the memory map (half the physical `0xB000` budget — a multi-kernel
f32 cell is ~6 KB of honest bytes until kernels go bank-resident).

H-F2's pre-registered prediction ("fadd/fmul low thousands") missed by ~3× — recorded,
not hidden. Pinned as regression ceilings in `cell80/tests/f32_kernels.rs`.

**Multi-kernel cells fit** (2026-07-07, same day the limit was found): the locals
scratch region relocates *above the code* whenever the code outgrows the classic
`0x9000` window — the same measured placement the frame loop has always used —
byte-identical for every program that fits the old window (goldens unchanged), and
a hard ceiling where the memory map actually is: `0xB000` (`STATE_BASE`) on the Cell
target, `0xF000` on Spectrum. That gives a cell ~12KB for code + locals — a
three-kernel `lerp` chain (fsub → fmul → fadd, ~8KB) compiles, runs both targets,
and stays bit-identical to rustc (`diff::f32_ops::f32_multi_kernel_chain`).

**The resident kernel bank** (opt-in, `//! kernel_bank: on`): the arithmetic five +
comparison trio + helpers compile once to an 11,156-byte bank at `BANK_ORG = 0xC000`
(its own locals at `0xB800`, disjoint from cell scratch); a banked cell's `CALL`s
resolve into it and its image carries only its own logic — `impulse_1d_f32` went
8,197 B → 337 B. The cartridge pins the bank image's SHA-256 (`.cell` v9;
different bank ⇒ hard load error), the cell image carries a bank flag (image v2),
and the runner places the bank outside touch-tracking so it survives per-run
resets. Non-bank kernels (conversions, rounding, min/max) still append per cell
and resolve their `f32_pack`/`flt` calls into the bank when banked.

One honest limit remains, F1 work: the variable-distance shifts inside
`f32_shr_jam`/alignment are shift-by-1 loops because u32 shifts take literal amounts
only — the main T-state lever. The enabling codegen win is already in: *constant* u32
shifts decompose word/byte-first (`<< 31` is ~15 bytes, not 248), which also shrank
every Q-format cell that shifts.

## What `check!` actually guarantees

Every differential test compiles its block for **both targets** — `Spectrum48` (software
`__mul16`/`__divmod16`/`__mul32`/`__divmod32` routines) and `Cell` (the `ED FE` trap path
the cell VM ships) — runs both on the emulator core (itself validated by 1,530,000
SingleStepTests vectors + ZEXDOC), asserts the targets agree with each other, and asserts
both equal the same source compiled by **release-mode rustc** on the host. So the oracle
guarantees agreement with *release* Rust semantics (wrapping, no overflow panics) on the
inputs tested; it cannot speak to `/ 0` (rustc panics) or out-of-subset programs (they
don't compile). Rejection tests pin the reject-don't-approximate rule.

## Out of the dialect (by design, not omission)

Strings as a *type* (string literals compile as addressable const data — see above — but
there is no `String`, no slices, no string ops), `f64` and platform libm (libm
permanently, per the F-wave amendment; `f32` is in, owned, above), unsuffixed decimal
literals (the canon pass's exact-decimal lane, not a dialect value),
`u64`, heap allocation, closures, traits, recursion, I/O. These are the escalation path —
a cell that needs them isn't a cell, and the honest answer is a typed hand-off to the
host, not a bigger ISA. See the roadmap's non-goals.

**Floats specifically are rejected permanently, not deferred — the reason is the oracle.**
`check!`'s guarantee is that both compile targets agree with each other *and* with
release-mode rustc on the host; IEEE basic arithmetic is bit-specified, but the
transcendentals (`sin`/`cos`/`exp`/`log`/real `pow`) are not — rustc lowers them to
platform libm, and libm results differ across hosts. A cell whose reference behaviour is
"whatever this machine's libm returned" breaks the differential oracle, weakens content
addressing (same source, same inputs, host-dependent facts), and degrades the fact file
from "memory you can't lie to" into "memory that was true on the machine that wrote it."
Real-valued computation still has a home in the dialect — as unit-tagged, error-bounded,
demand-gated Q-format fixed point (`q_mul`/`q_div`/`q_sqrt`/`q_lerp`/`q_sigmoid` today);
see `docs/real-valued-cells-spec.md` for the full policy (scale joins the unit system,
an accuracy contract for approximate cells, and the exactness taxonomy that keeps Q from
free-riding on the fractions' "exact" claim).

That hand-off is a language-level idiom, not new syntax: `halt(code)` with a code in
the **escalation band** (`0xFF00`–`0xFFFF`) reports as `halt: "escalate"` with a named
reason (`0xFF01` = `needs_strings`, `0xFF02` = `needs_floats`, `0xFF03` = `needs_io`,
`0xFF06` = `out_of_domain`, …) instead of `halted` — the orchestrator routes the
request up a rung rather than treating it as a failure. A cell's *static* boundary is
declared in its manifest (`//! limits:` header → the `.cell` `limits` field). Codes and
the full table live in [09-cell80-abi.md](09-cell80-abi.md).
