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

## `if`/`match` as values

`let x = if c { a } else { b };` and `match`-with-value-arms are accepted in `let`,
assignment, `return`, and tail position (with nesting and `else if` chains), lowering to
the statement form through the destination slot. A value-`if` needs an `else`; a
value-`match` needs a `_` arm; every branch must end with the value (no trailing `;`) —
each violation is its own instructive compile error.

## Arithmetic

All integer arithmetic is **wrapping** (mod 2^width) — the semantics of release-mode
rustc, which is what the differential oracle runs. `u8` ops mask to 8 bits, `u16` to 16,
`u32` to 32. `wrapping_add`/`wrapping_sub`/`wrapping_mul` are accepted and identical to
the bare operators (everything wraps). There is no overflow trap and no checked
arithmetic; a cell that must not wrap uses the library's saturating cells (`add_sat`,
`mul_sat`) or guards explicitly.

Shifts: a constant amount unrolls; a runtime amount shifts by the **low byte** of the
operand, and a count ≥ the width shifts out to `0` (matching rustc's behaviour for the
in-range counts the oracle tests; rustc panics on out-of-range constants in debug — the
dialect does not, it zeroes).

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

At most **3 parameters** (the `HL`/`DE`/`BC` register convention; no stack args), at most
3 tuple-return values (same registers). `u32` values do not cross call boundaries —
params, returns, and arguments are 16-bit; widen inside the callee (`as u32`) and narrow
before returning. A `u32` in any 16-bit position is a compile error, never a truncation.

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
