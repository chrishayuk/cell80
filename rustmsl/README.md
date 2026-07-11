# rustmsl

The Metal (MSL) backend of the cell family (Phase 6, WS-E): IR → MSL codegen
for **straight-line integer cells** (E1) plus a batch GPU executor on macOS.

One IR, one oracle. `rustmsl::compile` lowers the `cell80-core` typed IR to a
Metal compute kernel — one thread per input triple — with the reference
interpreter's semantics reproduced arm by arm: width masking, the
shift-by-≥-width corners, signed div/rem wrapping (`MIN/-1`), divide-by-zero
and `halt(code)` as per-thread traps. A GPU result that does not agree
bit-for-bit with the interpreter is a defect, never a "GPU difference".

The codegen is platform-independent text emission and builds everywhere; the
executor (`GpuBatch`) exists behind `cfg(target_os = "macos")` and compiles
kernels with fast-math off.

E1 scope (pre-registered): loop-free bodies (`if` allowed), integer widths
only. Loops (E2), the batch megakernel layouts (E3), and the f32 kernel bank
(E4) land per docs/14-model-native-cells-spec.md.
