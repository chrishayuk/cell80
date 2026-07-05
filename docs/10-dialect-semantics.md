# Dialect semantics — what an accepted program means

The spec-07 addendum the determinism contract rests on: exactly what the restricted-Rust
dialect *guarantees*, on both targets, and what the differential oracle does and doesn't
check. The rule of thumb: **an accepted program computes what rustc computes** — anything
the compiler can't make true it must reject, never approximate.

## Types

`u8`, `u16`, `i16`, `u32`, `bool`. `i16` is two's complement in a single slot:
add/sub/mul and the bitwise ops share the unsigned bit patterns (wrapping), while
**comparisons order by sign**, **divide truncates toward zero** (the remainder takes the
dividend's sign — rustc semantics; `i16::MIN / -1` wraps to `i16::MIN`), and **`>>` is an
arithmetic shift** (the sign propagates). Casts between `i16` and `u16`/`u8` are
bit-preserving; `i16 as u32` (a sign extension in Rust) is rejected — take the bits
explicitly (`x as u16 as u32`). Negative literals need the suffix (`-5i16`), and unary
`-` needs a signed operand.

**Fractional values are a fixed-point convention on integers, not a type**: a Q8.8
weight is a `u32` (or `u16` for small ranges) with an implied point — multiply then
shift (`(a * w) >> 8`). This keeps every cell bit-exact and cross-target deterministic;
there are no floats and there will be none (see the non-goals). Float and `char`
literals are rejected with instructive errors.

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
byte-identical on the differential battery.

The *math* residual stands: a Q16.16 `q_mul` needs a 64-bit intermediate the
substrate doesn't have, so that kernel remains word-split partials
(`ah·bh<<16 + ah·bl + al·bh + (al·bl)>>16`, each 16×16→32) however its operands
arrive — a **state cell** (named `u32` fields in, `u32` out) stays the recommended
pattern for kernels that are wide in *math*, not just in arity. Host note: a u32-param **entry** is drivable from the host by
passing the two words as `args = [low, high]` — the register convention makes the
split exact.

A 16-bit argument in a wide slot zero-extends (the value rustc would infer for an
in-range literal); a wide value in a 16-bit slot stays an error.

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
there is no `String`, no slices, no string ops), floats, `u64`, heap allocation, closures,
traits, recursion, I/O. These are the escalation path — a cell that needs them isn't a
cell, and the honest answer is a typed hand-off to the host, not a bigger ISA. See the
roadmap's non-goals.

That hand-off is a language-level idiom, not new syntax: `halt(code)` with a code in
the **escalation band** (`0xFF00`–`0xFFFF`) reports as `halt: "escalate"` with a named
reason (`0xFF01` = `needs_strings`, `0xFF02` = `needs_floats`, `0xFF03` = `needs_io`,
`0xFF06` = `out_of_domain`, …) instead of `halted` — the orchestrator routes the
request up a rung rather than treating it as a failure. A cell's *static* boundary is
declared in its manifest (`//! limits:` header → the `.cell` `limits` field). Codes and
the full table live in [09-cell80-abi.md](09-cell80-abi.md).
