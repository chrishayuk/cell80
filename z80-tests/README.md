# z80-tests — correctness harness for the `z80` core

Three layers, fast to slow. The unit/smoke layers run on a plain `cargo test`; the two
data-driven correctness layers fetch their inputs on demand (git-ignored) and **skip
cleanly when the data is absent**, so CI is green without setup and thorough with it.

## 1. Decode smoke — `tests/opcode_sweep.rs`
Drives every base + CB/ED/DD/FD/DDCB/FDCB opcode through `step()` once and asserts it
neither panics nor hangs. No data needed. Proves every decode arm is reachable and safe
(not that results are correct — that's layer 2).

## 2. SingleStepTests — `tests/single_step.rs` (the headline layer)
Per-opcode vectors from <https://github.com/SingleStepTests/z80>: ~1000 cases each, every
one an initial CPU+RAM state, the expected final state after **one** instruction, and the
cycle-by-cycle bus activity. The harness sets up the state, runs one `step()`, and diffs
**every** register, RAM cell, port write, and the total T-state count.

```sh
z80-tests/sst/fetch.sh          # a representative subset (~70 files, all instruction classes)
z80-tests/sst/fetch.sh --all    # the entire instruction set (~1.8k files, ~1.5 GB)
cargo test -p z80-tests --release single_step -- --nocapture
```

Knobs: `SST_DIR` overrides the data dir · `SST_MAX_CASES=N` caps cases per opcode ·
`SST_NO_CYCLES=1` checks state only. On failure it prints a per-field and per-opcode tally
plus the first 30 diffs.

## 3. ZEXDOC / ZEXALL — the `zex` test (coarse acceptance)
The classic CP/M exerciser ROM, run on `run_zex`; it CRCs instruction-result sequences
against known-good values. `#[ignore]`d because it's billions of T-states.

```sh
z80-tests/zex/fetch.sh          # zexdoc.com   (--all also gets zexall.com)
cargo test -p z80-tests --release --lib zex -- --ignored --nocapture
```

ZEXDOC (documented flags) runs in ~30 s; ZEXALL (all undocumented flags too) is much
longer — point `ZEX_ROM` at `zexall.com`.

## Bugs this harness caught
Building layer 2 found **six** real core bugs, since fixed (`z80/src/decode.rs`, `cpu.rs`):
- **EI/IFF** — `EI` must set IFF1/IFF2 *immediately* (only interrupt *acceptance* is delayed
  one instruction); the core was delaying the IFF set, which mis-handled `EI; LD A,I`.
- **LDIR/LDDR**, **CPIR/CPDR** — on a repeated iteration the undocumented XY flags come from
  the high byte of PC, not from `A±(HL)` (Patrik Rak).
- **INIR/INDR/OTIR/OTDR** — the repeat fixup for `wz`, XY (from PCh) and the H/P flags.
- **DD/FD-prefixed SCF/CCF** — an ineffective index prefix is its own M1 cycle that re-latches
  Q, so the following SCF/CCF reads `q_prev` as 0 (changes the XY-flag quirk).
- **LD (IX+d),n timing** — 19 T-states, not 22: this two-operand form reads d then n with a
  2 T internal delay, not `ptr_addr`'s 5 T (which is for the `r,(IX+d)` read/write forms).

After the fixes: SingleStepTests **1 530 000 / 1 530 000** across the full instruction set —
base + CB/ED/DD/FD/DDCB/FDCB, including cycle counts (run `fetch.sh --all`) — and ZEXDOC
reports no CRC mismatch. The committed subset (`fetch.sh`, ~86 files) covers every class.
