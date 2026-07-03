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
there are no floats and there will be none (see the non-goals).

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

Strings, floats, `u64`, heap allocation, closures, traits, recursion, I/O. These are the
escalation path — a cell that needs them isn't a cell, and the honest answer is a typed
hand-off to the host, not a bigger ISA. See the roadmap's non-goals.
