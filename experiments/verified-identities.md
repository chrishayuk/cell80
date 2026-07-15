# Verified identities: proved, cost-model-independent, adjudicated separately

A composition can be **exact** — full-domain-proved equal to a target's authored
behaviour — without being a **win** on any given body: costs are per-ISA, identities
aren't. This file exists so that discovery finding a real algorithmic identity and
discovery finding a cost win stay two separate claims, never conflated, and so the
expensive half (blind discovery + full-domain proof) is never re-done just because a
verdict changes.

**Entry format:** the identity, how it was found, the proof (both algebraic and
machine-checked), and the current cost verdict per body — updated in place as bodies
are added or cost models are repriced, with the *identity*'s own entry never touched.

---

## `next_pow2`

**Identity:** for all `x: u16`,

```
next_pow2(x) == snap_up(mask_xor(x, is_zero(x)), highest_set_bit(x))
```

where (`cell80/cells/{predicates/is_zero,bit-mask/mask_xor,bit-mask/highest_set_bit,
bounds/snap_up}.rs`, `cell80/cells/number-theory/next_pow2.rs`):
- `next_pow2(x)`: smallest power of two `≥ x` (`0` if it would exceed `65535`;
  `next_pow2(0) = 1`), computed by the reference as a doubling `while` loop.
- `is_zero(x) = (x == 0) as u16`; `mask_xor(a,b) = a^b`; `highest_set_bit(x)`
  isolates the value of `x`'s top set bit via smear-then-subtract (`0` when
  `x == 0`); `snap_up(x, step) = x` if `step == 0 || x == 0`, else rounds `x` up to
  the nearest multiple of `step`.

**Discovered:** blind, by GA+CEGIS composition search over the library's free-fn
vocabulary (`cell80/examples/gpu_fanout_gate.rs`, the C0 fan-out-gate re-run,
2026-07-14 — `experiments/cell-fanout-gate-findings.md` §5). Not hand-derived, not
suggested by any prior hit; the search found it as a byproduct of chasing a cheaper
`next_pow2` under the experiment's IR-step cost model.

**Proof.**
- *Algebraic* (`cell-fanout-gate-findings.md` §5, full case analysis): `x=0` hits
  `snap_up`'s `step==0` early-return with the pre-patched value `1`, matching
  `next_pow2(0)=1`; `x` already a power of two passes through `snap_up(x,x)`
  unchanged; `2^k < x < 2^(k+1)` gives `snap_up(x, 2^k) = ceil(x/2^k)*2^k =
  2^(k+1)`, the correct next power of two; the documented overflow case
  (`x=65535`) reaches `0` via genuine unsigned `u16` wraparound inside `snap_up`'s
  own multiply, matching the reference's own overflow convention by the same
  mechanism, not by coincidence.
- *Machine-checked, twice, independently*: full 65,536-input sweep under the IR-step
  interpreter (the search's own verifier) — 0 mismatches. Full 65,536-input sweep on
  the real Z80 body (`cell80/examples/spotcheck_next_pow2_z80.rs`) — 0 mismatches.

**Cost verdict, by body:**

| body | reference mean | composed mean | ratio | verdict |
|---|---|---|---|---|
| Z80 (raw runner cycles) | 4,716.4 T-states | 1,313.0 T-states | 3.59× | composed *appears* cheaper |
| Z80 (P-repriced, `P=5,412`) | 4,716.4 T-states | 12,136.8 T-states | 0.389× | **reference cheaper — composed LOSES** |

**Why the Z80 verdict is what it is.** The composed candidate's `snap_up` divides
then multiplies; `cell80::Runner` charges every `/`/`*` a flat 4-T-state host trap
(`cell80/src/runner.rs:127`) regardless of the fact that a real Z80 has no MUL/DIV
instruction — the same underpricing `cell-cost-discovery` measured and named `P`
(≈5,412 T-states, the true cost a genuine software routine would carry). The
reference's doubling loop contains no `/`/`*` at all, so repricing only ever adds
cost to the composed side. A per-stage breakdown
(`cell80/examples/spotcheck_next_pow2_breakdown.rs`) confirms the traps themselves
cost only 8 raw T-states (2 × 4) — the win was never really "the traps are free,"
it was "the model doesn't know what they'd really cost," and repricing closes that
gap directly. **The Z80 verdict is a body-specific fact, not evidence against the
identity.**

**Live question, not yet answered here:** would this identity pay on a body with
real hardware multiply/divide (removing the trap-repricing penalty entirely) or a
barrel shifter (removing `highest_set_bit`'s multi-bit-shift cost, its own largest
raw contributor at ~676 T-states with zero traps)? `cell80`'s multi-target
direction (RV32 and others) makes this a concrete, checkable question, not a
hypothetical — re-run the same hand-composed-source technique
(`spotcheck_next_pow2_z80.rs`'s pattern) against whichever body's runner exists
next, and update this entry's table with a new row. The identity itself needs no
re-proving.

**Status:** proved; **not adopted** as a library win on any body checked so far.
