# cells-pending/physics — authored, verified shape, blocked on kernel bytes

These cells are **complete and correct by construction** (typed f32 through the
owned softfloat kernels, finite-gated outputs) but **refused by the sandbox code
cap**: each pulls all four arithmetic kernels and lands just over the 8192-byte
policy (measured 2026-07-08: `impulse_1d_f32` **8,197 B** — five bytes over —
and `elastic_collision_1d_f32` **8,570 B**, after bit-exact restructuring).

They are the **bank-resident-kernels demand signal** the F-wave amendment
registered: a cell carrying its own ~8 KB of kernel bytes is honest but heavy,
and the moment two cells in one pack breach the cap, the fix is the kernel bank
(load the family once, hash the bank version into the artifact context), not a
bigger cap. When the bank lands, these move into `cell80/cells/physics/`
unchanged and pay admission like everyone else.

They are deliberately *outside* the discovered library tree so the index, golden,
and admission gates stay truthful about what actually ships.
