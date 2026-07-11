//! The Metal executor (macOS): compile an [`MslModule`]'s source with fast-math
//! **off**, then dispatch one thread per input triple and read back each
//! thread's `[r0, r1, r2, status]` quad. Buffers are `StorageModeShared`
//! (unified memory — the Apple-Silicon path docs 14 leans on for G1/G3).

use crate::codegen::{MslModule, IN_STRIDE, KERNEL_NAME, OUT_STRIDE};
use metal::{
    Buffer, CommandQueue, CompileOptions, ComputePipelineState, Device, MTLCommandBufferStatus,
    MTLResourceOptions, MTLSize,
};

/// A compiled-and-ready cell kernel: pipeline state, queue, and the const blob
/// resident on the device. Build once per cell, dispatch many batches.
pub struct GpuBatch {
    device: Device,
    queue: CommandQueue,
    pipeline: ComputePipelineState,
    consts: Buffer,
}

impl GpuBatch {
    /// Compile the module on the system Metal device. Fast-math is disabled —
    /// integer cells never depend on it, and E4's f32 bank will require it off.
    pub fn new(module: &MslModule) -> Result<Self, String> {
        let device = Device::system_default().ok_or_else(|| "msl: no Metal device".to_string())?;
        let opts = CompileOptions::new();
        opts.set_fast_math_enabled(false);
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
        })
    }

    /// Run one thread per input triple; each output quad is `[r0, r1, r2,
    /// status]` (status per [`crate::STATUS_OK`] and friends; a halt's code
    /// rides `r0`).
    pub fn run(&self, inputs: &[[u16; IN_STRIDE]]) -> Result<Vec<[u16; OUT_STRIDE]>, String> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }
        let n = inputs.len();
        let in_buf = self.device.new_buffer_with_data(
            inputs.as_ptr() as *const _,
            (n * IN_STRIDE * 2) as u64,
            MTLResourceOptions::StorageModeShared,
        );
        let out_buf = self.device.new_buffer(
            (n * OUT_STRIDE * 2) as u64,
            MTLResourceOptions::StorageModeShared,
        );
        let cmd = self.queue.new_command_buffer();
        let enc = cmd.new_compute_command_encoder();
        enc.set_compute_pipeline_state(&self.pipeline);
        enc.set_buffer(0, Some(&in_buf), 0);
        enc.set_buffer(1, Some(&out_buf), 0);
        enc.set_buffer(2, Some(&self.consts), 0);
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
        Ok(out.to_vec())
    }
}
