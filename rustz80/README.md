# `rustz80` — a restricted Rust → Z80 compiler

Write a game in a **subset of Rust that is also real Rust**, and compile it to Z80
machine code that boots on a real ZX Spectrum — no C, no external toolchain. The
same `.rs` runs two ways:

- under **`rustc`** (`cargo run`) — host execution, fast iteration, a real debugger;
- through **`rustz80`** — Z80 you can package as a `.tap` and boot on the ROM.

The two are kept honest by **differential testing**: every feature is run both ways
and the results must match (see [`tests/`](./tests)). Design rationale lives in
[spec 07](../docs/07-rust-z80-compiler-spec.md).

Not an LLVM backend and not real `core`: a `syn` frontend → a small typed IR → naive
Z80 codegen (`HL` accumulator, `DE` secondary, a fixed RAM "register file"), plus a
hand-written mul/div micro-runtime.

`rustz80` is **generic** — it knows nothing about games or any SDK. The Spectrum game layer
(`impl Game`, the dialect prelude, the symbol map, the `speccy-compile` CLI, and the
emulator to boot the `.tap`) lives in
[`chuk-speccy`](https://github.com/chrishayuk/chuk-speccy) (its SDK's `compile` feature),
built on this crate's generic API (`lower_program` with a caller-supplied `PreludeConfig`,
`codegen_loop`, `to_tap`). chuk-speccy depends on cell80 for that.

## Quick start

```bash
# Generate a bootable Spectrum tape via the library API:
#   let tap: Vec<u8> = rustz80::compile_to_tap(src, "main", "GAME")?;

# To RUN compiled programs headless on the deterministic cell micro-VM, use the
# `cell80` crate (built on this compiler): cargo run -p cell80 --bin cell80 -- run prog.rs
```

Samples live in [`samples/`](./samples). The `speccy-compile` CLI and the emulator that
boots the `.tap` are in [chuk-speccy](https://github.com/chrishayuk/chuk-speccy).

## The dialect

Supported today (all differential-tested):

| Feature | Notes |
|---|---|
| Types | `u16` (default) and `u8` (wraps at 256). `as u8` truncates, `as u16`/`as usize` widen. `u32` (two slots, computed in `HL:DE`) for `^ & \|` + constant shifts + `as u16`/`as u8` — enough for a 32-bit xorshift RNG. |
| Arithmetic | `+ - * / %`, `wrapping_add/sub/mul`. `*`/`/`/`%` use the appended micro-runtime — *except by a constant*: `× k` is shift-and-add, `/ 2ⁿ` / `% 2ⁿ` are shift/mask, and literal-only ops const-fold (no runtime call). (16-bit; `u32` arithmetic beyond bitwise/shift is not done yet.) |
| Bitwise | `\|` `&` `^`, and `<<` / `>>` by a **constant** amount (`u16` and `u32`) or a **runtime** amount (`u16`; a counted shift loop — a count ≥ 16 shifts out to `0`). |
| Booleans | Comparisons (`< <= > >= == !=`) work as **conditions** *and* as **values** — `(a < b) as u16` materialises `1`/`0`. Short-circuit `&&` / `\|\|` on bool operands. So a predicate is a one-liner: `fn run(a: u16, b: u16) -> u16 { (a <= b) as u16 }`. |
| Control flow | `if`/`else if`/`else`, `while`, `for` over integer ranges (`a..b` / `a..=b`, `for _ in`), `loop` / `break` / `continue`, early `return`. |
| Arrays | `let a = [0u16; N];` (a single block fill — `LDIR`, or an `ED FE` trap in Cell mode) / `[e0, e1, …]`; `a[i]`, `a[i] = v`. Index with `i as usize`. `[u8; N]` are byte-packed-per-slot. Arrays of structs `let a = [Cell { … }; N]` — element field access `a[i].x` (read/write) + whole-element assign `a[i] = Cell { … }`. |
| Structs | `struct P { x: u16, y: u16 }` + literals + `p.x` read/write. Scalar, `[u16; N]`, tuple (`pos: (u16, u16)`, `p.pos.0`), and array-of-structs (`cells: [Cell; N]`, `p.cells[i].x`, `p.cells[i] = Cell { … }`) fields. |
| Enums + match | `enum Dir { Up = 1, … }` (explicit discriminants or `0,1,2,…`); `match` on integers/variants with `_`. Plus `bool` (`true`/`false`). |
| Functions + methods | Free fns and `impl T { fn m(&mut self, …) }` — up to 3 args in `HL`/`DE`/`BC`, result in `HL`; `self.field` through the receiver. |
| Generics | Generic *free functions* (`fn max<T: Ord>(…)`, `fn buf<const N: usize>()`), monomorphized per call — a type argument (turbofish or inferred) sets the instance's width, a const argument (turbofish) sizes arrays and substitutes as a value. Generic *structs* + methods (`struct Pair<T>`): type args erased to 16-bit. **Const-generic structs** (`struct Buf<const N: usize> { data: [u16; N], … }`) are monomorphized per `N` — a per-instance layout + methods (`Buf$8::push`), `N` inferred at the struct literal from the array field's length. The field may itself be an array of structs — **`Entities<Cell, const N> { data: [Cell; N], … }`**, the fixed-capacity entity pool. |
| Tuples | Multiple return values: `fn divmod(…) -> (u16, u16)` (in `HL`/`DE`/`BC`) destructured with `let (q, r) = …` — a tuple literal or a call. |
| Raw I/O | `poke(addr, val)` / `peek(addr)` (memory) and `inport(port)` (I/O ports, e.g. the keyboard at `0xFE`). |
| Cell80 | `halt(code)` — stop the cell early with a status code (`ED FE` host trap; surfaces as `Halt::Halted(code)` in the report). A no-op on real hardware, so it's harmless in a Spectrum build. |

Out of scope (use `rustc`-only host code, or wait for later stages): recursion
(needs stack frames — Stage 4), references / `&mut` params, `>3` params, slices,
`String`/`Vec`/`alloc`, floats, traits, `u32` *arithmetic* (`+ - * /`) and `u32`
params/returns (bitwise/shift `u32` works), `u32` *variable* shift amounts (`u16`
variable shifts work), closures, nested
struct *fields*. Anything unsupported is a **clear compile error** — that error is the
"this is host-only" budget detector.

## A whole program

```rust
// The canonical ZX screen-address math + a pixel plotter, in the dialect.
fn addr_of(x: u16, y: u16) -> u16 {
    16384u16 + (y / 64u16) * 2048u16 + (y % 8u16) * 256u16
        + ((y / 8u16) % 8u16) * 32u16 + x / 8u16
}
fn mask_of(x: u16) -> u16 {
    let masks = [128u8, 64u8, 32u8, 16u8, 8u8, 4u8, 2u8, 1u8];
    masks[(x % 8u16) as usize] as u16
}
fn main() {
    let a = addr_of(0u16, 0u16);
    poke(a, peek(a) | mask_of(0u16)); // light the top-left pixel
}
```

`samples/snake.rs` is a complete game (body in arrays, `match` steering, draw via
`poke`/`peek`) — the worked example end to end.

## Examples — run the language

Runnable demos in [`examples/`](./examples) each compile a dialect program (in
[`samples/showcase/`](./samples/showcase)), run it on the real `z80` CPU, print the
result, and check it against the same algorithm in plain rustc:

```bash
cargo run -p rustz80 --example sorting        # insertion sort  (arrays, break, for)
cargo run -p rustz80 --example sieve          # primes < 100    (byte arrays, nested loops)
cargo run -p rustz80 --example rpn_vm         # a bytecode VM   (arrays + match dispatch)
cargo run -p rustz80 --example state_machine  # vending machine (struct + enum + methods)
cargo run -p rustz80 --example rng            # 16-bit LCG      (wrapping_mul, ^)
cargo run -p rustz80 --example numerics       # gcd / isqrt / fib (while, return, loop)
cargo run -p rustz80 --example generics       # one generic source → 6 monomorphic instances
cargo run -p rustz80 --example const_generics # const-param array sizes (triangle$4, triangle$8)
cargo run -p rustz80 --example stack          # const-generic fixed-cap stack (Stack$4, Stack$8)
cargo run -p rustz80 --example points         # array of structs [Cell; N], a[i].x access
cargo run -p rustz80 --example pool           # fixed-cap entity pool (struct field [Cell; N])
cargo run -p rustz80 --example entities       # Entities<Cell, const N> — two instances ($4, $8)
cargo run -p rustz80 --example rng32          # 32-bit xorshift RNG (u32 in the HL:DE pair)
cargo run -p rustz80 --example structs        # generic struct + methods + a tuple field
cargo run -p rustz80 --example tuples         # multiple return values (HL/DE/BC)
cargo run -p rustz80 --example report         # per-function code-size report (instances + runtime)
cargo run -p rustz80 --example bitmap         # draw to screen RAM, printed as ASCII art
```

The `bitmap` demo prints what it drew straight from the framebuffer:

```
########
##......
#.#.....
#..#....
#...#...
#....#..
#.....#.
#......#
```

`tests/examples.rs` locks every showcase result, so a codegen regression fails
`cargo test` even without running the demos.

## Running compiled programs → the `cell80` crate

`rustz80` compiles; it doesn't run. To execute compiled programs **headless on a
deterministic, sandboxed cell micro-VM** — `.cell` cartridges, a warm `CellHost`, typed-state
I/O, host-routed `CellGraph` composition, and the `cell80` CLI — use the
[`cell80`](../cell80) crate, which is built on this compiler's public API
(`compile_file` / `struct_layout` / `entry_signature` / `Signature` / `Target` / `ORG`).

## The dial: one `impl Game`, two compilers

(The commands below run from the [chuk-speccy](https://github.com/chrishayuk/chuk-speccy)
repo, whose SDK wraps this compiler.) Write an ordinary `speccy-sdk` `Game` and the
*same file* compiles **both** ways:

- **`rustc`** (host): a normal `impl Game for T { fn update(&mut self, …) }` — debug it.
- **`rustz80`**: `speccy-compile` detects the `impl Game`, routes `frame.*`/`input.*`
  to a **dialect prelude** (`Frame::pixel`/`clear` → screen pokes), lays the game
  state out as a zero-initialised global, and generates a frame loop
  (`EI; HALT; DI; CALL update` — interrupts on only for the 50 Hz sync, off during
  `update`). The output boots on the real ROM.

```bash
cargo run -p chuk-speccy-sdk --features compile --bin speccy-compile -- speccy-sdk/samples/bounce.rs -o bounce.tap
cargo run --release --bin speccy-gui -- testroms/48.rom bounce.tap
```

`samples/bounce.rs` (self-playing) and `samples/move.rs` (**playable** — cursor keys
or QAOP move a blob) are exactly this; `tests/dial.rs` compiles each under rustc
*and* rustz80 and boots them, proving the dial. The pure prelude covers
`Frame::clear`/`pixel` and **real `Input::held`** (keyboard read via the `inport`
intrinsic, mapped like the SDK). Games stay in the dialect subset (fixed state, no
`Vec`/`String`).

```bash
cargo run -p chuk-speccy-sdk --features compile --bin speccy-compile -- speccy-sdk/samples/move.rs -o move.tap
cargo run --release --bin speccy-gui -- testroms/48.rom move.tap   # then press 5/6/7/8 or Q/A/O/P
```

## How it works

- **Frontend** (`lower/`): `syn::parse_str` → accepted subset → typed IR (`ir.rs`).
  Unsupported nodes become errors. Split by concern: `vars` (the register file),
  `layout` (struct/enum layout + parse helpers), `prelude` (handle routing),
  `generics` (monomorphization), `expr`, and `stmt`; `mod.rs` owns the `Ctx` and the
  function-level orchestration.
- **Codegen** (`codegen/`: `asm` · `runtime` · `expr` · `stmt`): IR → Z80. Locals (incl.
  params) live in a per-function scratch region; expressions evaluate via `HL` + the stack;
  `*`/`/`/`%` `CALL` an appended `__mul16`/`__divmod16` (Spectrum) or trap (Cell).
- **Library API**: `compile_program(src) -> Program { code, symbols }`,
  `compile_fn(src) -> Vec<u8>`, `to_tap(code, org, entry, name)`,
  `compile_to_tap(src, entry, name)`. Code is laid out from `ORG = 0x8000`.
- **Tape boot**: `compile_to_tap` emits a `DI; CALL entry; EI; RET` trampoline at
  `ORG` and a BASIC autoloader (`CLEAR; LOAD "" CODE; RANDOMIZE USR`). The `DI` is
  load-bearing: the ROM's interrupt routine clobbers `BC`/`DE` (keyboard scan),
  which the codegen keeps live — so games run with interrupts off.

## Tests

```bash
cargo test -p rustz80   # differential oracle + .tap structure
cargo test -p cell80    # the cell micro-VM suite (its own crate)
```

- `tests/diff.rs` — the oracle: each `check!` runs one Rust block under `rustc` and
  through `rustz80` on a flat-RAM Z80 and asserts they agree; plus multi-`fn` programs
  for generics, tuples, structs/methods, and control flow.
- `tests/snake.rs` — the whole dialect at once: a Snake checked against a Rust replica
  (state checksum + screen bitmap).
- `tests/examples.rs` — locks each `samples/showcase/` program's result (the demos in
  `examples/` run the same sources against a rustc oracle).
- `tests/coverage.rs` — the error/rejection arms, prelude routing, the frame-loop
  generator, and array-struct fields through `self` — the paths the above don't reach.
- `tests/tap.rs` — `.tap` block structure (offline). The *boot on a real Spectrum* test
  lives in [chuk-speccy](https://github.com/chrishayuk/chuk-speccy). The cell micro-VM suite
  (cartridge, host, index, graph, determinism + reset fuzzer) lives in the
  [`cell80`](../cell80) crate.

Coverage (`cargo llvm-cov -p rustz80 -- --include-ignored`): **~97% of lines**, every source
file ≥ 90%.
