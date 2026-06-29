# Synthesis (experimental second mode)

cell80's main pitch is **retrievable deterministic micro-tools**: `search → inspect → run`,
and static composition via `cell_compose` / `CellGraph`. Most use is exactly that — the LLM
reads the described steps, retrieves the cells, wires them in order, runs them. That workload
**folds** (no search needed), so there is no learned/search organ in the main path.

Synthesis is a deliberately separate, experimental mode for the one regime that *doesn't*
fold.

## What it is — the inverse of `CellGraph`

- `CellGraph`: **graph → output** — you author a chain and run it.
- Synthesis: **output → graph** — you give input→output **examples** and it *discovers* a
  short chain of cells that reproduces them.

The **verifier is the engine** in both directions: a candidate chain is accepted only when it
reproduces *every* example exactly (run it, check the numbers).

## When it earns its keep

Only on **outcome-specified synthesis over lossy ops**:

- **outcome-specified** — the prompt gives a target/examples, not the steps. If the steps are
  described ("do A then B"), it's a pipeline and folds — use `cell_compose` instead.
- **lossy / non-metric ops** — AND/OR/mask/rotate/pack/checksum, where "get closer in value"
  is deceptive, so distance-greedy fails and real backtracking search is required. On smooth
  arithmetic, greedy has a gradient and it folds — don't use this.

Real instances: bit-hack/mask synthesis, superoptimization, FlashFill-style string transforms,
crypto/CTF transform chains, constraint shaping.

## API (`cell80::synth`)

```rust
use cell80::{synthesize, Op};

let ops = vec![ Op::from_cell("xor_00ff", &xor_cell, 0x00FF), /* … */ ];
let examples = [(0x1234u16, 0x34edu16), (0x00ff, 0xffff)]; // input -> output
if let Some(plan) = synthesize(&examples, &ops, /*max_depth*/ 5, /*budget*/ 200_000) {
    println!("{:?}", plan.steps); // e.g. ["xor_ff00", "swap"] — a chain that satisfies all examples
}
```

- `Op::from_cell(name, cartridge, arg)` — an op = a cell applied to the running value
  (transition precomputed by the VM; applying it *is* executing the cell).
- `synthesize(examples, ops, max_depth, budget) -> Option<Plan>` — A* over the verifier with a
  **hand Hamming heuristic** (heuristic-first). `Plan { steps, tested, depth }`.
- `synthesize_with(.., h)` — pluggable heuristic seam. See the gate below.

Runnable demo: `cargo run --release --example synth_demo -p cell80`.

## Heuristic-first; the learned value net is *gated*, not assumed

The hand Hamming heuristic ships by default. A *learned* value net `v(s,t) ≈ dist` can be
plugged via `synthesize_with` — but it must **earn in** by beating the hand heuristic at equal
budget. In the clean-room kill gate (`examples/composition_eval.rs`) it only **tied** the hand
heuristic, so it is not baked into the default. (The original SOMA b3bits result has it winning
decisively — banked in `../docs/roadmap.md` — but that code is gone and was not reproduced
here, so it stays a gate, not a given.)

## Placement

Synthesis lives in **cell80** — it's search over a verifiable op-space, which is cell80's
substrate. It is **not** a SOMA concern (SOMA is the multi-rate runtime; synthesis has nothing
to do with timescale separation). SOMA may one day *schedule* cell80 synthesis as a slow organ
under a real-time deadline — a call, not ownership.
