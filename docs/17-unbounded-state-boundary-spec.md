# 17 — Unbounded State Boundary Spec

Design document for the boundary between a fixed-extent cell and growable protected
state. Written for the Cellular Residual Computer programme's Turing-universality
claim (`chris-experiments-2/experiments/CRC_SUITE_FINDINGS.md`), but the boundary is
programme-independent. Nothing here is built; this is the spec to build against.

## 0. Two corrections to the plan this replaces

The plan was: ship Phase S3, add a bignum/tape cell family, and the universality claim
becomes mechanical. Both halves need correcting before anything is built.

**The array-persistence gap is already closed.** `.cell` v11 landed `Ty::Array(ArrayElem,
u16)` on 2026-07-11 with `StateCell::set_array`/`get_array` and
`CellHost::run_state_values`; `u16[N]`/`u32[N]` state fields round-trip by name today
(`docs/09-cell80-abi.md`, `cell80/src/report.rs`). Phase S3 is now *only* `bytes[N]` /
`str[N]` byte-I/O. The blocker named in
`experiments/sliding-window-state-cells-findings.md` was resolved by its own close-out.

**Shipping S3 would not buy unboundedness, and neither would a bignum cell family.**
Cell80 is a **64 KiB flat-RAM Z80**. Typed state is laid at `STATE_BASE = 0xB000`, the
resident bank's scratch at `BANK_SCRATCH = 0xB800`, the kernel bank at `BANK_ORG =
0xC000`. The entire typed state region is therefore **2 KiB with a kernel bank resident,
4 KiB without** — and `Ty::Array(elem, u16)` is a *fixed declared envelope* whose reads
return all `N` slots. No marshalling surface changes this. The ceiling is the address
space, not the I/O.

So unbounded state cannot live inside a cell, at any element width, under any ABI.

That is not a defeat. Pushing the tape host-side is the **Turing decomposition** — finite
control plus unbounded tape — and it fits the machine that already exists more honestly
than a bignum-in-a-cell would. The finite control is exactly what Cell80 is good at: a
fixed, deterministic, cycle-accounted transition function with a byte-exact touched-set.

## 1. Architecture

```text
protected activation state   (host-side, growable)        ← the tape
        │   windowed read, receipt-bound
        ▼
Cell80                       (64 KiB, fixed extent)       ← the finite control
        │   typed proposal + receipt (may carry an extent request)
        ▼
receipt validator            → commit, or exact no-write
        │
        └── the one commit path; growth is not an exception to it
```

The learned controller sits above this, proposing typed operations. It is not shown
because it has no privileged relationship to growth — see §2.3.

## 2. Overflow and growth are different events

The single most important decision in this document: **do not unify them.** They look
alike (both arise when a value will not fit) and they must behave oppositely.

### 2.1 Arithmetic overflow within a fixed extent stays a no-write fault

Unchanged from CRC finding 2. A `u16` limb that overflows inside the cell is a fault, the
proposal is rejected, and the complete protected state is byte-exactly unchanged. This is
the invariant that has held across every CRC experiment and it is not being weakened.

### 2.2 Tape growth is a host-side extent change, never a cell operation

The cell never resizes. It cannot: its address space is fixed at compile time. Growth
changes the extent of the host-side protected region, and the cell only ever sees a
window of it.

### 2.3 The proposer is the deterministic transition function, not the learned controller

Growth must be explicitly proposed and receipt-bound — but **Cell80 proposes it, not the
controller.**

The reason is that universality must not inherit the controller's error rate. Minsky's
counters grow as a *consequence* of `INC`, not as a separately-authored instruction. If
extent changes require the learned controller to ask for them, then the machine computes
unbounded functions only when a component currently sitting at 0.600 on unseen wording
correctly asks — which is not a counter machine, it is a finite-state machine with an
unreliable escape hatch.

Concretely:

1. The controller proposes a typed operation — `INC r`, and nothing about extents.
2. The cell computes the transition over the current window and returns, as part of its
   **deterministic** result, whether the window was exhausted (`carry_out` past the top
   limb for `INC`, `borrow_out` for `DEC`).
3. The host's **exact** layer, not its learned one, turns that signal into an extent
   request on the receipt.
4. The validator checks the receipt and commits, or refuses and writes nothing.

Growth stays auditable and typed, travels the same commit path as everything else, and
depends on no learned component. The controller cannot cause growth, cannot prevent it,
and cannot observe it except through committed state.

**Gate:** a test asserting that no controller-authored proposal can carry an extent
change, and that a forged one is refused by the validator.

### 2.4 Growth and write are one receipt

If extend and write commit separately, a sequence that extends successfully and then
faults on the write leaves the tape longer with the value unincremented. Nothing is
corrupt and the state is well-formed — but it is *not the pre-state*, so CRC finding 2's
wording ("failure leaves the complete protected machine state byte-exactly unchanged")
becomes false at precisely the moment the design was trying to preserve it.

**One receipt, one commit.** An extent change and the write it enables are a single
atomic unit. There is no observable intermediate state in which the tape has grown but
the operation has not completed.

**Gate:** a torn-grow test — force the write to fault after the extent change is staged,
and assert the pre-state digest (§3) is unchanged, extent included.

## 3. Receipt schema

The pre-state digest must cover **extent as well as contents**. Otherwise a
grow-then-fail is indistinguishable at the digest level from a no-op, and §2.4's gate
cannot be written.

```text
receipt {
  pre_digest    : H(extent ‖ contents)     -- extent is inside the hash, not beside it
  op            : typed proposal (controller-authored)
  extent_delta  : 0 | +k                   -- cell-authored; 0 for every non-growing op
  post_digest   : H(extent' ‖ contents')
}
```

`extent_delta` is a distinct field rather than an implicit consequence of `post_digest`
so that growth is greppable in a trace. The maximum extent a program reached becomes a
derived quantity of its receipt log rather than an implicit property of a run — which is
what makes "this trace used unbounded state" auditable rather than asserted.

Shrinking is out of scope. `extent_delta` is non-negative; reclamation, if it is ever
wanted, is a separate operation with its own receipt and its own argument.

## 4. Representation

A counter is a little-endian limb vector host-side, and a `u16[K]` window cell-side —
the v11 array lane, already shipped, no new ABI surface required.

Two consequences worth designing for rather than discovering:

- `INC`/`DEC` are O(1) amortised over the window; carry propagation past the top limb is
  what raises `carry_out` and drives §2.3.
- **Zero-testing is the expensive predicate.** Naively it is O(limbs), and for a counter
  machine "is this counter zero" is the *only* predicate the program ever asks. Cache a
  zero flag in the protected state and maintain it as part of the same receipt. Without
  this, the branch cost grows with the counter's magnitude, which is both slow and a
  side channel on the state's size.

## 5. Settle the Minsky encoding question in the same pass

Two-counter machines are Turing-universal, but with a caveat that a reviewer will find:
Schroeppel (1972) showed a 2CM cannot compute *n* ↦ 2ⁿ, so the universality result holds
for **Gödel-encoded** input and output, not for direct representations.

Two ways to close it:

- **State the encoding convention explicitly** in the claim. Cheapest, and honest.
- **Use three counters.** Removes the caveat entirely, and in this design it costs
  address space rather than architecture.

Recommend the third counter. The resize machinery above is per-tape and nothing in it is
per-counter, so a third costs one more limb vector and one more zero flag. Deciding this
before the spec hardens is the point — retrofitting a counter after the receipt schema is
frozen is the expensive version.

## 6. What Phase S3 is still for

Not this. S3 is `bytes[N]` / `str[N]` byte-I/O — host packing, `str_out` promotion, graph
`str` edges (`docs/09-cell80-abi.md`, `docs/11-machine-text.md` §3). It is worth building
on its own merits and the tape window will ride the same wire pattern that v11
established (code 6 plus element sub-code, following v6's code-3/4 precedent).

But it is **not on the critical path for the universality claim**, and decoupling the two
is the main scheduling consequence of this document. Neither one blocks the other.

## 7. Scope and honest limits

- The claim this enables is Turing-universal **given unbounded host memory**. That is the
  standard idealisation and it should be written down rather than implied — the machine
  is universal in the same sense every physical implementation is.
- The finite control remains genuinely finite at 64 KiB, which is the correct shape. What
  changed is only where the tape lives.
- CRC finding 2's boundary is controller-independent and stays so. Nothing above gives a
  learned component a new capability; §2.3 specifically removes one it would otherwise
  have acquired by accident.
- None of this touches the wording or role-head problems, which are separate and
  currently the binding constraint on getting *any* program compiled
  (`CRC_SUITE_FINDINGS.md` §12). An exact unbounded machine that nothing can be compiled
  onto is still worth building, but the sequencing should be a deliberate choice rather
  than an assumption.

## 8. Build gates

Before this is considered done:

1. Every existing no-write test passes unchanged.
2. Torn-grow test (§2.4): extent staged, write faults, pre-state digest unchanged.
3. Controller-cannot-grow test (§2.3): forged extent-carrying proposal is refused.
4. Extent appears in the pre-state and post-state digests, verified by a test that a
   grow-only mutation changes the digest.
5. Zero-flag consistency (§4): maintained across every committed op, checked against a
   recomputed zero-test over the full limb vector.
6. A recurrence test in the shape of CRC-10's 6,035-transition run, but crossing at least
   one extent boundary, asserting byte-exactness throughout.
