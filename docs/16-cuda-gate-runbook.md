# 16 — The CUDA gate runbook (WS-E: CUDA E1–E3 + interp backend)

The one-off cloud session that graduates the CUDA backend from
"golden-locked codegen, built but never run on silicon" to "gated, bit-exact
against the reference interpreter" — the same standard the Metal backend
cleared (741/746, docs 14 ledger 2026-07-11). The gate itself is
pre-registered in docs/14 §6 (2026-07-13 entry) **before** this session
runs; nothing below may weaken it.

Everything CUDA-side is opt-in behind the `cuda` cargo feature (cudarc
0.19.8, dynamic-loading, driver API pinned `cuda-12060`), so the box needs
an NVIDIA driver compatible with CUDA 12.6 (driver ≥ 555 is safe) — the
toolkit itself is not required to *build*, only `libcuda`/`libnvrtc` to
*run*; a stock CUDA 12.6 image provides both.

## 1. Box

- Lambda or RunPod, any Ampere-or-newer card (A10 / RTX 4090 / A100 — the
  battery is small; the cheapest current-gen card is fine).
- Image: a CUDA **12.6** image (matches the pinned cudarc driver-API
  feature). Record, verbatim, into the session notes for the closing ledger
  entry:
  - `nvidia-smi` (driver version, GPU name),
  - `nvcc --version` if present, else the image's advertised CUDA version
    (this pins the NVRTC version — libnvrtc ships with the toolkit),
  - the repo commit hash under test,
  - `rustc --version` (toolchain per workspace `rust-version = 1.85`).

## 1b. Pre-flight (no GPU needed — run on any dev box first)

Already-green as of the pre-registration commit, and re-runnable any time
the dialect changes before the session:

```sh
cargo test -p rustmsl --release --test cuda_text_semantics
cargo test -p cell80 --release --test cuda_cpu_emu_battery -- --ignored --nocapture
cargo check --workspace --all-targets --target x86_64-unknown-linux-gnu
```

These run the emitted CUDA text through the host C++ compiler
(`rustmsl::cpu_emu`) against the interpreter oracle — corners, full library,
megakernel, interp kernel — so the box session starts from "semantics
validated; only NVRTC acceptance and NVIDIA codegen left to prove". A CUDA
failure on the box therefore points at the toolchain/hardware seam, not the
dialect's semantics.

## 2. Setup

```sh
git clone <repo> && cd cell80
git checkout <pinned commit>
cargo check -p rustmsl --features cuda        # builds without the toolkit; sanity
```

## 3. Semantics seams first (rustmsl's own batteries)

```sh
cargo test -p rustmsl --features cuda --release
```

- `tests/corners.rs` — the R1/E2/typed-state corner battery, the exact text
  the Metal side runs per push, dispatched onto `CudaBatch`.
- `tests/interp_parity.rs` — the interp kernel (`CudaInterpBatch`) vs the
  portable reference VM.
- `tests/codegen_snapshot.rs` — the golden lock, re-confirmed on the box.
- `e1_e2_battery_cuda` prints `toolchain_info()` (device + compute
  capability) — capture it.

## 4. CI-speed battery (library sweep)

```sh
cargo test -p cell80 --features cuda --test cuda_battery --release -- --nocapture
```

512 inputs/value cell + 256 (input, state) pairs/state cell + the fused
megakernel. Transcript hits make this GPU-run + digest compare — minutes.
Floors: ≥ 230 value cells and ≥ 300 state cells compiled, zero
disagreements. The megakernel test is also where an NVRTC quirk at fused
scale would surface — the analogue of the Metal branch-inversion find.

## 5. The pre-registered 10⁶ gates

```sh
cargo test -p cell80 --features cuda --test cuda_battery --release -- --ignored --nocapture
```

`gate_one_million_cuda` + `state_gate_one_million_cuda`: 10⁶ random inputs
(and state blocks) per admitted cell, values + trap status + IR-step counts
(+ final state bytes) bit-exact. Transcript hits keep the interpreter off
the critical path; a miss grades live across the box's cores. **Do not run
with `UPDATE_GOLDEN=1`** — blessing is a macOS/Metal activity; the CUDA
battery never writes the book (enforced in code, stated here for the
operator).

**Transcript-coverage note (as of 2026-07-13):** the book fully covers the
value cells at the 10⁶ schedule (a handful of entries are stale — cells
modified since blessing — and will grade live), but only 116 of ~539 state
cells have `@st1000000` entries; the library grew past the original
blessing run. A local re-blessing (`UPDATE_GOLDEN=1`, the msl_battery 10⁶
gates on a Mac) was started 2026-07-13 and parked after ~8 CPU-hours in the
stale-value heavy tail — it saves per cell and can resume any time. Budget
accordingly: either finish the blessing on a Mac before the session (the
CUDA 10⁶ gates then cost GPU dispatch + digest compares), or accept that
the box live-grades ~420 state cells at 10⁶ pairs on its own cores (hours
of rented CPU). The gate's verdict is identical either way — only the wall
clock and the bill differ.

## 6. Throughput figure

docs/14 (E3) wants the CUDA number **measured on the first available card,
never extrapolated**. There is no CUDA twin of `library_launch_cost.rs`
yet; record the cheap proxy — wall-clock the CI-speed battery's megakernel
test and note cells × probes / dispatch time — and file the proper
`library_launch_cost` port as owed if the number matters downstream.

## 7. On any mismatch

The battery names the cell and prints the first disagreeing inputs.
Minimize on the box (the offending cell + input through `CudaBatch` in a
scratch test), fix in the **CUDA dialect arms / cuda.rs only** (Metal's
emission is golden-locked and stays untouched), regenerate the CUDA goldens
(`UPDATE_GOLDEN=1 cargo test -p rustmsl --test codegen_snapshot`), re-run
from step 3. Gates are never weakened; a cell that cannot be made bit-exact
is a filed defect excluded by name with a reason (the `STATE_OOW_DEFECTS`
pattern), counted in the ledger.

## 8. Close out

Append the results ledger entry to docs/14 §6: pass counts (value / state /
megakernel / corners / interp parity), every toolchain pin from §1, the
throughput proxy, and any defects filed. Commit from the box or copy the
diff off it before releasing the instance.
