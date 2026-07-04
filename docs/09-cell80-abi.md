# Cell80 ABI — v2

The frozen contract every `cell80` consumer (CLI, MCP server, tool index, future
`.cell` cartridge) relies on. **`ABI_VERSION = 2`** (`cell80::ABI_VERSION`, also the
`"abi"` field of the JSON report). Bump only on a breaking change to anything below.

**v2 (the 32-bit lane).** Two additions over v1: the `ED FE` traps `0x12` (MUL32) and
`0x13` (DIVMOD32), and **`u32` state fields** — two consecutive little-endian slots (low
word first), drivable and readable *by name* at their full width (the `.cell` format v4
carries a `Ty` per state field). Everything else is unchanged. The bump exists because the
trap addition is only *additive on a v2 host*: a v1 host treats unknown trap ids as no-ops,
so a cartridge compiled with `u32` mul/div would compute garbage there — the artifact must
declare which host it needs.

The cell is a **flat-RAM Z80** — no ROM, no ULA, no I/O ports by default, no syscalls —
provided by the `cell80` crate. Determinism is the whole point: same program + same inputs ⇒
identical result, identical cycle count, identical touched-set (asserted by
`runner_reuse_is_deterministic`).

## Memory map

| region | address | purpose |
|---|---|---|
| trampoline | `0x7000` | argument loader + `CALL entry` + `HALT` (written per run) |
| code (`ORG`) | `0x8000` | the compiled program; **const data** (data `const` items, interned string literals) is byte-packed immediately after the code, each blob at its own symbol |
| scratch / locals | `0x9000` | the "virtual register file": local `i` at `0x9000 + i*2` |
| typed state (convention) | `0xB000` (`STATE_BASE`) | where `StateCell` lays a state struct |
| stack | `0xFFF0` (`SP_TOP`), grows down | |

64 KiB flat. A program may read/write anywhere it has the capability for; the runner zeroes
only the bytes a run *touched* before the next run.

## Calling convention

- **Entry**: an exported `fn` (default resolution: `run`, then `main`; or an explicit name,
  e.g. `State::run`).
- **Arguments**: up to three `u16`, passed in `HL`, `DE`, `BC` (in that order). A method
  entry (`State::run(&mut self)`) takes the state base in `HL`.
- **Return**: the result is in **`HL`**; a `-> (u16, u16, u16)` tuple fills `HL`, `DE`, `BC`.
  The report exposes all three as `regs[0..3]` (`result == regs[0]`).

## Typed I/O

State lives as a `struct` in memory (by convention at `STATE_BASE = 0xB000`).
`rustz80::struct_layout(src, "State")` returns each field's slot `offset` and `slots`; a
scalar field is one 2-byte slot at `base + offset*2` (`u8` in the low byte). A `[u8; N]`
field is **byte-packed** (Phase S): `N` raw bytes at `base + offset*2` in `ceil(N/2)`
slots, flagged by `FieldLayout::bytes` — not name-addressed until the ABI-v3 `bytes[N]`
manifest type. Callers:

- **inputs**: write typed values before the run (`Runner::run_with_inputs`, CLI
  `--set addr:ty=val`), applied after the reset and cleaned before the next run;
- **outputs**: read typed values from post-run memory (`Runner::read_named` / `peek_u8/16/32`,
  CLI `--read name@addr:ty`);
- **by name**: `StateCell::bind(src, "State", entry)` does the name↔address mapping —
  `set("x", v)` → `run` → `get("score")`.

Field-state through this map is **differentially verified against rustc**
(`struct_field_state_matches_host`), not just against expected literals.

## Cycle budget & the `cycles` caveat

Each run is bounded by a T-state `budget` (the deterministic liveness guard); exceeding it
halts with `cycle_budget`. The reported **`cycles` is a deterministic *relative* cost
metric, not authentic Z80 time**: in Cell mode `*`/`/`/`%` and `[v; N]` fills are `ED FE`
host traps serviced natively and charged a flat ~4 T-states each, *not* their
software-routine cost. So `cycles` is correct for liveness and replay, but **must not** be
used as a hardware-fidelity figure or an RL reward — that would reward pushing work into
traps that read as "free." (The authentic `Spectrum48` target keeps the real software
routines; only the `Cell` target traps.)

**`trapped_ops`** is the honest companion: the count of cost-bearing traps (`mul`/`div`/fill)
a run executed. A reward function should pair it with `cycles` — weight or refuse trap-heavy
programs — rather than treat low `cycles` as cheap. (A program's compute footprint is then
roughly `cycles + trapped_ops × <software-routine cost>`.)

**This *exposes* the unfaithfulness; it does not close it.** Reporting two numbers means a
consumer that wants a single scalar cost still has to choose the `<software-routine cost>`
weight — and the honest weight differs by what you're modelling: the real-Z80 cost of a
software `mul`/`div` (tens of T-states) versus the host-trap cost (≈4). cell80 deliberately
leaves that choice to the cost model rather than baking one in; if you collapse to a scalar
(e.g. for an RL reward), pick the weight that matches your fidelity target and state it.

The memory footprint is bounded
separately by `max_touched` (the write-budget); there is no separate cap to add. `halt`
(`0x30`) is the explicit-stop trap and is *not* counted in `trapped_ops`; an unknown trap id
is a no-op (also uncounted).

## Halt status

| `halt` | meaning |
|---|---|
| `returned` | the entry returned cleanly |
| `halted` (+ `halt_code`) | the program called `halt(code)` (an `ED FE` trap) |
| `escalate` (+ `escalate`, `escalate_code`) | the program called `halt(code)` with a code in the **escalation band** — see below |
| `cycle_budget` | the T-state budget was reached first |
| `memory_limit` | the `max_touched` ceiling was reached |
| `div_by_zero` | a `/ 0` / `% 0` reached a divide trap under the default `DivByZero::Halt` policy (the `Saturate` opt-in keeps the legacy bounded-garbage value instead) |

### The escalation band (`0xFF00`–`0xFFFF`)

A `halt(code)` in this band is not an outcome or a failure but a **structured
hand-off** (roadmap 3.2): the cell declares that the request exceeded the kernel
class, so an orchestrator should route it up the escalation ladder rather than retry
or report an error. The band rides the ordinary `halt` trap — any cell can escalate
today, no new compiler surface. Named reasons:

| code | `escalate` |
|---|---|
| `0xFF00` | `unspecified` |
| `0xFF01` | `needs_strings` |
| `0xFF02` | `needs_floats` |
| `0xFF03` | `needs_io` |
| `0xFF04` | `needs_network` |
| `0xFF05` | `needs_wider_math` |
| `0xFF06` | `out_of_domain` |

Unnamed codes in the band decode as `custom`; the raw code always rides along as
`escalate_code`. The static half of the contract is the manifest's `limits` field
(`.cell` v5, `//! limits:` in a library source header): what the cell *can't* do,
declared so a router can avoid the escalation round-trip entirely.

## Capability model

`CellConfig` gates the raw intrinsics and caps resources; **`default()` = `sandboxed()`**:

| field | `sandboxed()` (default) | `permissive()` |
|---|---|---|
| `allow_raw_memory` (`poke`/`peek`) | `false` | `true` |
| `allow_ports` (`inport`) | `false` | `true` |
| `max_code_bytes` | `Some(4096)` | `None` |
| `max_touched` | `Some(4096)` | `None` |

A program using a denied intrinsic fails to compile under that policy. The policy travels
with a compiled `CellProgram` (and its serialized image).

## Report JSON

`Report::to_json()` emits, in order (`abi` carries the current `ABI_VERSION` — 2):

```json
{"abi":2,"entry":"run","entry_addr":32768,"result":42,"regs":[42,0,0],
 "cycles":67,"trapped_ops":0,"budget":2000000,"halt":"returned","code_bytes":47,"functions":1,
 "symbols":{"run":32768},"memory_touched":[[36864,36867]],"reads":{}}
```

- `abi` — schema version (this document).
- `entry` / `entry_addr` — the function run, and its address.
- `result` / `regs` — `HL`, and `[HL, DE, BC]`.
- `cycles` / `budget` — see the caveat above.
- `trapped_ops` — count of cost-bearing traps (the honest companion to `cycles`).
- `halt` — one of the statuses above; `halt_code` is present only for `halted`;
  `escalate` + `escalate_code` only for an escalation.
- `cache` — `{hits, lookups}` of the runner's memoization cache; present only when
  the cache is enabled (`Runner::enable_cache` / `CellHost::set_cache`).
- `code_bytes` / `functions` — compiled size and function count.
- `symbols` — name → address, sorted by address.
- `memory_touched` — contiguous written ranges, `[start, end_inclusive]`.
- `reads` — named typed values requested via `--read` (else empty).

## Image & cartridge format

`CellProgram::to_bytes()` / `from_bytes()` serialize a compact, self-contained **image**
(magic `CZ80`: version, code, symbols, policy) with no `syn`. A **`.cell` cartridge** (magic
`CELL`, format **v5**) wraps that image with its `Manifest` — id, summary, tags, entry,
source hash, compiler + ABI version, the typed I/O **signature** (`params` / `ret` / `state`),
`state_addrs`: each scalar state field's byte address **and width** (`Ty`, one byte:
0 = u16, 1 = u32, 2 = u8) at `STATE_BASE` (so a host or a peer cell in a graph drives the
cell **by field name without the source** — a `u32` field at its full width), and the
`limits` list (the escalation contract's static half, above). `from_bytes`
still reads v4 (no `limits`, no content addressing), v3 (addresses without widths →
fields read as `u16`), and v2 (no `state_addrs`) cartridges. This named, versioned,
manifest-bearing artifact is the object the CLI, a tool index, the MCP server, and a
`CellGraph` pass around. (Note: the `.cell` *file* format version — v5 — is distinct
from the runtime `ABI_VERSION` above, now 2.)

### Content addressing & signing (v5)

After the manifest, a v5 cartridge carries its **artifact hash** — SHA-256 over the
serialized manifest + the image, i.e. the whole tool: metadata, entry, capability
policy, code — and an optional **ed25519 signature block** (`(verifying key,
signature)` over that hash). `Cartridge::from_bytes` **verifies both by default** and
refuses a mismatch; every loader (`exec`, `inspect`, `index`, `serve`, the MCP
library's `.cell` path) inherits the check. `--no-verify` (CLI) /
`from_bytes_unverified` is the dev escape hatch. Pre-v5 cartridges carry no hash and
load as before (grandfathered — recompile to pin them). `cell80 keygen` mints a
signing key; `cell80 sign <file.cell> --key <key>` embeds the signature in place.
Signing does not change the artifact hash: the signature *wraps* the address, so a
signed and an unsigned serialization of the same tool share it.
