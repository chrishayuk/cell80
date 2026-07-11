//! The Metal executor (macOS): compile an [`MslModule`]'s source with fast-math
//! **off**, then dispatch the `n_cells × n_inputs` grid and read back each
//! thread's `[r0, r1, r2, status, steps_lo, steps_hi]` sextet. Buffers are
//! `StorageModeShared` (unified memory — the Apple-Silicon path docs 14 leans
//! on for G1/G3).

use crate::codegen::{MslModule, IN_STRIDE, KERNEL_NAME, OUT_STRIDE};
use metal::{
    Buffer, CommandQueue, CompileOptions, ComputePipelineState, Device, MTLCommandBufferStatus,
    MTLResourceOptions, MTLSize,
};

/// A compiled-and-ready module: pipeline state, queue, and the const blob
/// resident on the device. Build once, dispatch many batches.
pub struct GpuBatch {
    device: Device,
    queue: CommandQueue,
    pipeline: ComputePipelineState,
    consts: Buffer,
    n_cells: usize,
    /// Total state bytes per input across every cell (Σ state_len) — the
    /// cell-major state buffers' per-input stride.
    state_stride: usize,
}

impl GpuBatch {
    /// Compile the module on the system Metal device. Fast-math is disabled —
    /// integer cells never depend on it, and E4's f32 bank will require it off.
    pub fn new(module: &MslModule) -> Result<Self, String> {
        let device = Device::system_default().ok_or_else(|| "msl: no Metal device".to_string())?;
        let opts = CompileOptions::new();
        opts.set_fast_math_enabled(false);
        opts.set_language_version(metal::MTLLanguageVersion::V3_1);
        let library = device
            .new_library_with_source(&module.source, &opts)
            .map_err(|e| format!("msl: kernel compile failed: {e}"))?;
        let function = library
            .get_function(KERNEL_NAME, None)
            .map_err(|e| format!("msl: missing kernel `{KERNEL_NAME}`: {e}"))?;
        let pipeline = device
            .new_compute_pipeline_state_with_function(&function)
            .map_err(|e| format!("msl: pipeline creation failed: {e}"))?;
        let queue = device.new_command_queue();
        // A read-only const blob shared by every thread; Metal wants ≥1 byte.
        let blob: &[u8] = if module.consts.is_empty() {
            &[0u8]
        } else {
            &module.consts
        };
        let consts = device.new_buffer_with_data(
            blob.as_ptr() as *const _,
            blob.len() as u64,
            MTLResourceOptions::StorageModeShared,
        );
        Ok(GpuBatch {
            device,
            queue,
            pipeline,
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
            return Err("msl: this module has state cells — use run_with_state".into());
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
                "msl: state_in is {} bytes, want {} (state stride {} × {} inputs)",
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
        let in_buf = self.device.new_buffer_with_data(
            inputs.as_ptr() as *const _,
            (n_in * IN_STRIDE * 2) as u64,
            MTLResourceOptions::StorageModeShared,
        );
        let out_buf = self.device.new_buffer(
            (n * OUT_STRIDE * 2) as u64,
            MTLResourceOptions::StorageModeShared,
        );
        // Metal wants ≥1 byte even when no cell carries state.
        let stin_data: &[u8] = if state_in.is_empty() { &[0] } else { state_in };
        let stin_buf = self.device.new_buffer_with_data(
            stin_data.as_ptr() as *const _,
            stin_data.len() as u64,
            MTLResourceOptions::StorageModeShared,
        );
        let stout_buf = self.device.new_buffer(
            state_bytes.max(1) as u64,
            MTLResourceOptions::StorageModeShared,
        );
        let cmd = self.queue.new_command_buffer();
        let enc = cmd.new_compute_command_encoder();
        enc.set_compute_pipeline_state(&self.pipeline);
        enc.set_buffer(0, Some(&in_buf), 0);
        enc.set_buffer(1, Some(&out_buf), 0);
        enc.set_buffer(2, Some(&self.consts), 0);
        let n_inputs = n_in as u32;
        enc.set_bytes(
            3,
            std::mem::size_of::<u32>() as u64,
            &n_inputs as *const u32 as *const _,
        );
        enc.set_buffer(4, Some(&stin_buf), 0);
        enc.set_buffer(5, Some(&stout_buf), 0);
        // Non-uniform threadgroups (dispatch_threads) — every Apple-Silicon
        // family supports them, so the grid is exactly `n` with no tail guard.
        let width = self.pipeline.thread_execution_width().max(1);
        enc.dispatch_threads(MTLSize::new(n as u64, 1, 1), MTLSize::new(width, 1, 1));
        enc.end_encoding();
        cmd.commit();
        cmd.wait_until_completed();
        if cmd.status() != MTLCommandBufferStatus::Completed {
            return Err(format!("msl: command buffer failed: {:?}", cmd.status()));
        }
        let out = unsafe {
            std::slice::from_raw_parts(out_buf.contents() as *const [u16; OUT_STRIDE], n)
        };
        let state_out = if state_bytes > 0 {
            unsafe { std::slice::from_raw_parts(stout_buf.contents() as *const u8, state_bytes) }
                .to_vec()
        } else {
            Vec::new()
        };
        Ok((out.to_vec(), state_out))
    }
}
