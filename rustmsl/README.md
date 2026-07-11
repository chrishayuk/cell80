# rustmsl

The Metal (MSL) backend of the cell family (Phase 6, WS-E): IR → MSL codegen
for integer cells — straight-line (E1) and looping/branching (E2) — plus a
batch GPU executor and the library×probe-set megakernel (E3) on macOS.

One IR, one oracle. `rustmsl::compile` lowers the `cell80-core` typed IR to a
Metal compute kernel — one thread per (cell, input) pair — with the reference
interpreter's semantics reproduced arm by arm: width masking, the
shift-by-≥-width corners, signed div/rem wrapping (`MIN/-1`), divide-by-zero
and `halt(code)` as per-thread traps. Loops carry the interpreter's fuel
discipline tick for tick, so each thread reports its **IR-step count** (the
canonical family cost, docs 14 Q2) and batteries assert step parity alongside
value parity; a runaway loop is a counted trap, never a hung dispatch. A GPU
result that does not agree bit-for-bit with the interpreter is a defect, never
a "GPU difference".

`rustmsl::compile_library` fuses many cells into one translation unit: the
whole library runs against a probe set in a single dispatch — retrieval by
execution's substrate (WS-F).

Cell functions and the div/rem helpers are pinned `noinline`: the batteries
caught a real Apple Metal compiler bug (an integer divide feeding a branch
that guards stores through a `thread`-reference parameter inverts the branch
in non-inlined functions), so the shipped configuration is exactly the
battery-validated one instead of an inliner heuristic's.

The codegen is platform-independent text emission and builds everywhere; the
executor (`GpuBatch`) exists behind `cfg(target_os = "macos")` and compiles
kernels with fast-math off.

Measured (M3 Max, end-to-end, metering on): 3.7×10⁸ evals/s one-cell peak;
245 library cells × 10⁶ inputs bit-exact on values, status, and steps. Still
owed per docs/14-model-native-cells-spec.md: the library-launch fixed cost,
typed-state readback (state cells), the f32 kernel bank (E4), CUDA.
