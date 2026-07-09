# cells-pending/physics — emptied: the bank answered

The two cells that sat here (`impulse_1d_f32` at 8,197 B, `elastic_collision_1d_f32`
at 8,570 B — each just over the 8,192-byte sandbox cap because it carried all four
arithmetic kernels) moved into `cell80/cells/physics/` the day after they were
parked, **unchanged except for one header line** (`//! kernel_bank: on`): the
resident kernel bank landed, their images now call into `BANK_ORG` and carry only
their own logic — **337 B and 650 B**. The demand signal worked exactly as the
F-wave amendment registered it: measure the refusal, build the bank, move the
cells, never bend the cap.

This directory stays as the pattern's home: a cell blocked on a *policy* gate
parks here with its measurements, and the gate's answer — not an exception —
unblocks it.
