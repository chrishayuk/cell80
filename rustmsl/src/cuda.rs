//! The CUDA executor (the `cuda` cargo feature): NVRTC-compile a
//! CUDA-dialect [`GpuModule`] with `--fmad=false`, launch the
//! `n_cells × n_inputs` grid, and read back each thread's
//! `[r0, r1, r2, status, steps_lo, steps_hi]` sextet — mirroring the Metal
//! executor's API and layouts exactly ([`crate::GpuBatch`]), so the
//! batteries treat the two as interchangeable. E1–E3 cells are integer-only,
//! so determinism never rides on fp flags; the fmad pin is laid now so E4's
//! f32 bank inherits it (docs 14, R8).
//!
//! Kernels compile `--gpu-architecture=compute_{cc}` for the device they
//! will run on — the gate always compiles on the box it runs on, so there
//! is no cross-SM story to get wrong.
//!
//! cudarc is bound `dynamic-loading` (dlopen of libcuda/libnvrtc at run
//! time), so this module *builds* on machines with no CUDA stack — on one,
//! constructing a [`CudaBatch`] fails with a typed error instead.

use crate::codegen::{Dialect, GpuModule, IN_STRIDE, KERNEL_NAME, OUT_STRIDE};
use crate::interp::{bytecode, CellProgram};
use cudarc::driver::{
    CudaContext, CudaFunction, CudaSlice, CudaStream, LaunchConfig, PushKernelArg,
};
use cudarc::nvrtc::{compile_ptx_with_opts, CompileOptions, Ptx};
use std::sync::Arc;

/// Launch width for the compiled megakernel. The emitted grid-tail guard
/// makes the round-up safe; 256 is a portable default for integer kernels.
const BLOCK: u32 = 256;

/// Device 0's identity for the gate ledger: name + compute capability.
/// (Driver and NVRTC versions are pinned by the box image and recorded via
/// `nvidia-smi` in the runbook — cudarc's safe layer exposes no query.)
pub fn toolchain_info() -> Result<String, String> {
    let ctx = CudaContext::new(0).map_err(|e| format!("cuda: no device 0: {e:?}"))?;
    let name = ctx
        .name()
        .map_err(|e| format!("cuda: device name: {e:?}"))?;
    let (major, minor) = ctx
        .compute_capability()
        .map_err(|e| format!("cuda: compute capability: {e:?}"))?;
    Ok(format!("{name} (compute_{major}{minor})"))
}

/// NVRTC-compile `src` for device 0's architecture, fmad off. The full
/// NVRTC log rides the error — the analogue of Metal's compile error string
/// the battery panics with alongside the source.
fn compile_for(ctx: &Arc<CudaContext>, src: &str, what: &str) -> Result<Ptx, String> {
    let (major, minor) = ctx
        .compute_capability()
        .map_err(|e| format!("cuda: compute capability: {e:?}"))?;
    let opts = CompileOptions {
        fmad: Some(false),
        options: vec![format!("--gpu-architecture=compute_{major}{minor}")],
        ..Default::default()
    };
    compile_ptx_with_opts(src, opts).map_err(|e| format!("cuda: {what} compile failed: {e:?}"))
}

/// Uniform error text for a driver call — `what` names the failed step.
fn fail<E: std::fmt::Debug>(what: &str, e: E) -> String {
    format!("cuda: {what} failed: {e:?}")
}

/// A compiled-and-ready module: context, stream, kernel, and the const blob
/// resident on the device. Build once, dispatch many batches — the CUDA
/// sibling of [`crate::GpuBatch`].
pub struct CudaBatch {
    stream: Arc<CudaStream>,
    func: CudaFunction,
    consts: CudaSlice<u8>,
    n_cells: usize,
    /// Total state bytes per input across every cell (Σ state_len) — the
    /// cell-major state buffers' per-input stride.
    state_stride: usize,
}

impl CudaBatch {
    /// NVRTC-compile the module on device 0. Refuses a non-CUDA module with
    /// a typed error (the MSL executor refuses symmetrically).
    pub fn new(module: &GpuModule) -> Result<Self, String> {
        if module.dialect != Dialect::Cuda {
            return Err(format!(
                "cuda: module is {:?} dialect — CudaBatch runs CUDA (use compile_cuda/compile_library_cuda)",
                module.dialect
            ));
        }
        let ctx = CudaContext::new(0).map_err(|e| format!("cuda: no device 0: {e:?}"))?;
        let ptx = compile_for(&ctx, &module.source, "kernel")?;
        let cu_module = ctx
            .load_module(ptx)
            .map_err(|e| format!("cuda: module load failed: {e:?}"))?;
        let func = cu_module
            .load_function(KERNEL_NAME)
            .map_err(|e| format!("cuda: missing kernel `{KERNEL_NAME}`: {e:?}"))?;
        let stream = ctx.default_stream();
        // A read-only const blob shared by every thread; keep ≥1 byte so the
        // kernel always has a live pointer (the Metal executor's rule).
        let blob: &[u8] = if module.consts.is_empty() {
            &[0u8]
        } else {
            &module.consts
        };
        let consts = stream
            .clone_htod(blob)
            .map_err(|e| format!("cuda: const upload failed: {e:?}"))?;
        Ok(CudaBatch {
            stream,
            func,
            consts,
            n_cells: module.cells.len(),
            state_stride: module.cells.iter().map(|c| c.state_len).sum(),
        })
    }

    /// Run every cell in the module against every input triple — one thread
    /// per (cell, input), cell-major: the sextet for `(cell, input)` sits at
    /// `cell * inputs.len() + input`. A halt's code rides `r0`; steps decode
    /// via [`crate::steps_of`]. For modules with state cells use
    /// [`run_with_state`](Self::run_with_state).
    pub fn run(&self, inputs: &[[u16; IN_STRIDE]]) -> Result<Vec<[u16; OUT_STRIDE]>, String> {
        if self.state_stride > 0 {
            return Err("cuda: this module has state cells — use run_with_state".into());
        }
        Ok(self.dispatch(inputs, &[])?.0)
    }

    /// [`run`](Self::run) with per-thread state: `state_in` holds each
    /// thread's initial state block, cell-major like the outputs (cell 0's
    /// `n_inputs` blocks of its `state_len` bytes, then cell 1's, …). Returns
    /// the sextets and every thread's final state in the same layout.
    pub fn run_with_state(
        &self,
        inputs: &[[u16; IN_STRIDE]],
        state_in: &[u8],
    ) -> Result<(Vec<[u16; OUT_STRIDE]>, Vec<u8>), String> {
        if state_in.len() != self.state_stride * inputs.len() {
            return Err(format!(
                "cuda: state_in is {} bytes, want {} (state stride {} × {} inputs)",
                state_in.len(),
                self.state_stride * inputs.len(),
                self.state_stride,
                inputs.len()
            ));
        }
        self.dispatch(inputs, state_in)
    }

    fn dispatch(
        &self,
        inputs: &[[u16; IN_STRIDE]],
        state_in: &[u8],
    ) -> Result<(Vec<[u16; OUT_STRIDE]>, Vec<u8>), String> {
        if inputs.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }
        let n_in = inputs.len();
        let n = self.n_cells * n_in;
        let state_bytes = self.state_stride * n_in;
        let flat_in: Vec<u16> = inputs.iter().flat_map(|t| t.iter().copied()).collect();
        let in_buf = self
            .stream
            .clone_htod(&flat_in)
            .map_err(|e| fail("input upload", e))?;
        let mut out_buf: CudaSlice<u16> = self
            .stream
            .alloc_zeros(n * OUT_STRIDE)
            .map_err(|e| fail("output alloc", e))?;
        // ≥1 byte even when no cell carries state (live pointers, as Metal).
        let stin_data: &[u8] = if state_in.is_empty() { &[0] } else { state_in };
        let stin_buf = self
            .stream
            .clone_htod(stin_data)
            .map_err(|e| fail("state upload", e))?;
        let mut stout_buf: CudaSlice<u8> = self
            .stream
            .alloc_zeros(state_bytes.max(1))
            .map_err(|e| fail("state alloc", e))?;

        let n_inputs = n_in as u32;
        let cfg = LaunchConfig {
            grid_dim: ((n as u32).div_ceil(BLOCK), 1, 1),
            block_dim: (BLOCK, 1, 1),
            shared_mem_bytes: 0,
        };
        let mut launch = self.stream.launch_builder(&self.func);
        launch.arg(&in_buf);
        launch.arg(&mut out_buf);
        launch.arg(&self.consts);
        launch.arg(&n_inputs);
        launch.arg(&stin_buf);
        launch.arg(&mut stout_buf);
        // Safety: the kernel signature is emitted by this crate's codegen and
        // matches the six args above in order and type.
        unsafe { launch.launch(cfg) }.map_err(|e| fail("launch", e))?;

        let flat_out = self
            .stream
            .clone_dtoh(&out_buf)
            .map_err(|e| fail("readback", e))?;
        let out: Vec<[u16; OUT_STRIDE]> = flat_out
            .chunks_exact(OUT_STRIDE)
            .map(|c| <[u16; OUT_STRIDE]>::try_from(c).unwrap())
            .collect();
        let state_out = if state_bytes > 0 {
            self.stream
                .clone_dtoh(&stout_buf)
                .map_err(|e| fail("state readback", e))?
        } else {
            Vec::new()
        };
        Ok((out, state_out))
    }
}

/// The bytecode-interpreter backend on CUDA — the sibling of
/// [`crate::interp::InterpBatch`]: the fixed `interp` kernel plus the
/// concatenated bytecode + per-cell offset table. One block per cell, probes
/// across lanes; kernel size is constant in the number of cells. The sextet
/// grid is cell-major: `cell * probes.len() + probe`.
///
/// Like the Metal version's threadgroup cap, probes beyond the 1024-thread
/// block limit never execute — the shared cap is documented, not fixed,
/// on both backends.
pub struct CudaInterpBatch {
    stream: Arc<CudaStream>,
    func: CudaFunction,
    code_buf: CudaSlice<u32>,
    table_buf: CudaSlice<u32>,
    n_cells: usize,
}

impl CudaInterpBatch {
    /// Build from linearized cells. Cells whose local count exceeds the
    /// kernel bound are skipped; the count of skipped cells is returned
    /// alongside the batch. `n_cells()` reflects the admitted cells.
    pub fn new(progs: &[CellProgram]) -> Result<(Self, usize), String> {
        let ctx = CudaContext::new(0).map_err(|e| format!("cuda: no device 0: {e:?}"))?;
        let ptx = compile_for(&ctx, &crate::interp::interp_source_cuda(), "interp kernel")?;
        let cu_module = ctx
            .load_module(ptx)
            .map_err(|e| format!("cuda interp: module load failed: {e:?}"))?;
        let func = cu_module
            .load_function("interp")
            .map_err(|e| format!("cuda interp: missing kernel: {e:?}"))?;
        let stream = ctx.default_stream();
        let (code, table, skipped) = bytecode::pack(progs);
        let (code_buf, table_buf) = Self::upload(&stream, &code, &table)?;
        Ok((
            CudaInterpBatch {
                stream,
                func,
                code_buf,
                table_buf,
                n_cells: table.len() / 3,
            },
            skipped,
        ))
    }

    fn upload(
        stream: &Arc<CudaStream>,
        code: &[u32],
        table: &[u32],
    ) -> Result<(CudaSlice<u32>, CudaSlice<u32>), String> {
        // ≥1 element even for an empty library (live pointers, as Metal).
        let mk = |v: &[u32]| {
            let data: &[u32] = if v.is_empty() { &[0] } else { v };
            stream
                .clone_htod(data)
                .map_err(|e| format!("cuda interp: upload failed: {e:?}"))
        };
        Ok((mk(code)?, mk(table)?))
    }

    /// Admitted cell count (skipped cells excluded).
    pub fn n_cells(&self) -> usize {
        self.n_cells
    }

    /// Swap in a new program set, reusing the compiled kernel — the hot path
    /// for a search loop, avoiding an NVRTC recompile. Returns cells skipped
    /// for exceeding the local-slot bound.
    pub fn reload(&mut self, progs: &[CellProgram]) -> Result<usize, String> {
        let (code, table, skipped) = bytecode::pack(progs);
        let (code_buf, table_buf) = Self::upload(&self.stream, &code, &table)?;
        self.code_buf = code_buf;
        self.table_buf = table_buf;
        self.n_cells = table.len() / 3;
        Ok(skipped)
    }

    /// Run every admitted cell against every probe in one dispatch. Returns
    /// the sextets `[r0, r1, r2, status, steps_lo, steps_hi]`, cell-major.
    pub fn run(&self, probes: &[[u16; IN_STRIDE]]) -> Result<Vec<[u16; OUT_STRIDE]>, String> {
        if self.n_cells == 0 || probes.is_empty() {
            return Ok(Vec::new());
        }
        let flat: Vec<u16> = probes.iter().flat_map(|p| p.iter().copied()).collect();
        let probe_buf = self
            .stream
            .clone_htod(&flat)
            .map_err(|e| fail("interp probe upload", e))?;
        let n = self.n_cells * probes.len();
        let mut out_buf: CudaSlice<u16> = self
            .stream
            .alloc_zeros(n * OUT_STRIDE)
            .map_err(|e| fail("output alloc", e))?;
        let n_probes = probes.len() as u32;
        let tpb = probes.len().min(1024) as u32; // probes across lanes
        let cfg = LaunchConfig {
            grid_dim: (self.n_cells as u32, 1, 1),
            block_dim: (tpb, 1, 1),
            shared_mem_bytes: 0,
        };
        let mut launch = self.stream.launch_builder(&self.func);
        launch.arg(&self.code_buf);
        launch.arg(&self.table_buf);
        launch.arg(&probe_buf);
        launch.arg(&mut out_buf);
        launch.arg(&n_probes);
        // Safety: the fixed interp kernel's signature is these five args.
        unsafe { launch.launch(cfg) }.map_err(|e| fail("launch", e))?;
        let flat_out = self
            .stream
            .clone_dtoh(&out_buf)
            .map_err(|e| fail("readback", e))?;
        Ok(flat_out
            .chunks_exact(OUT_STRIDE)
            .map(|c| <[u16; OUT_STRIDE]>::try_from(c).unwrap())
            .collect())
    }
}
