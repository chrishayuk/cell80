//! The GPU library-dispatch backend (macOS/Metal). A fixed-size MSL kernel reads
//! each cell's IR from a data buffer, so kernel size is constant in the number of
//! cells — one threadgroup per cell, probes across lanes. The kernel source and
//! its `pack`-format bytecode both come from [`super::bytecode`]/[`super::source`]
//! — shared with `cuda.rs`'s `CudaInterpBatch` and `cpu_emu.rs`'s pre-silicon
//! validator, so an opcode-semantics fix lands in every dispatch backend at once.

use super::bytecode::{pack, CellProgram};
use super::source::interp_source_msl;
use crate::{IN_STRIDE, OUT_STRIDE};
use metal::{Buffer, CommandQueue, ComputePipelineState, Device, MTLResourceOptions, MTLSize};

/// A whole library compiled for GPU dispatch: the fixed interpreter kernel
/// plus the concatenated bytecode + per-cell offset table for a set of
/// [`CellProgram`]s. One threadgroup per cell, probes across lanes — kernel
/// size is constant in the number of cells (the point of this backend). The
/// sextet grid is cell-major: `cell * probes.len() + probe`.
pub struct InterpBatch {
    device: Device,
    queue: CommandQueue,
    pipeline: ComputePipelineState,
    code_buf: Buffer,
    table_buf: Buffer,
    n_cells: usize,
    max_tpg: usize,
}

impl InterpBatch {
    /// Build from linearized cells. Cells whose local count exceeds the
    /// kernel bound are skipped; the count of skipped cells is returned
    /// alongside the batch. `n_cells()` reflects the admitted cells.
    pub fn new(progs: &[CellProgram]) -> Result<(Self, usize), String> {
        let device = Device::system_default().ok_or_else(|| "msl: no Metal device".to_string())?;
        let (code, table, skipped) = pack(progs);
        let n_cells = table.len() / 3;
        let opts = metal::CompileOptions::new();
        opts.set_fast_math_enabled(false);
        opts.set_language_version(metal::MTLLanguageVersion::V3_1);
        let lib = device
            .new_library_with_source(&interp_source_msl(), &opts)
            .map_err(|e| format!("msl interp: kernel compile failed: {e}"))?;
        let func = lib
            .get_function("interp", None)
            .map_err(|e| format!("msl interp: missing kernel: {e}"))?;
        let pipeline = device
            .new_compute_pipeline_state_with_function(&func)
            .map_err(|e| format!("msl interp: pipeline creation failed: {e}"))?;
        let queue = device.new_command_queue();
        let max_tpg = pipeline.max_total_threads_per_threadgroup() as usize;
        // Metal wants ≥1 byte even for an empty library.
        let mk = |v: &[u32]| {
            let bytes: &[u32] = if v.is_empty() { &[0] } else { v };
            device.new_buffer_with_data(
                bytes.as_ptr() as *const _,
                (bytes.len() * 4) as u64,
                MTLResourceOptions::StorageModeShared,
            )
        };
        let code_buf = mk(&code);
        let table_buf = mk(&table);
        Ok((
            InterpBatch {
                device,
                queue,
                pipeline,
                code_buf,
                table_buf,
                n_cells,
                max_tpg,
            },
            skipped,
        ))
    }

    /// Admitted cell count (skipped cells excluded).
    pub fn n_cells(&self) -> usize {
        self.n_cells
    }

    /// Swap in a new program set, reusing the compiled kernel/pipeline — the
    /// hot path for a search loop (a fresh candidate population per
    /// generation), avoiding a kernel recompile. Returns cells skipped for
    /// exceeding the local-slot bound.
    pub fn reload(&mut self, progs: &[CellProgram]) -> usize {
        let (code, table, skipped) = pack(progs);
        let mk = |v: &[u32]| {
            let bytes: &[u32] = if v.is_empty() { &[0] } else { v };
            self.device.new_buffer_with_data(
                bytes.as_ptr() as *const _,
                (bytes.len() * 4) as u64,
                MTLResourceOptions::StorageModeShared,
            )
        };
        self.code_buf = mk(&code);
        self.table_buf = mk(&table);
        self.n_cells = table.len() / 3;
        skipped
    }

    /// Run every admitted cell against every probe. Returns the sextets
    /// `[r0, r1, r2, status, steps_lo, steps_hi]`, cell-major over the full
    /// probe set.
    ///
    /// Chunked to `max_tpg` (the pipeline's max threads per threadgroup): the
    /// kernel assigns one thread per probe within a threadgroup and has no
    /// internal loop over probes, so a single dispatch with more probes than
    /// `max_tpg` silently leaves every probe beyond that count's threads
    /// unwritten — the output buffer then reads back as a false
    /// `status=0, r0=0` ("succeeded, value 0") instead of the real result.
    /// Found live (docs: `cell-fanout-gate-preregistration.md`'s InterpBatch
    /// amendment): a full 65,536-probe domain sweep against a kernel whose
    /// `max_tpg` was lower left the upper probes zeroed, not erroring —
    /// exactly the silent-wrong-answer shape this codebase's own discipline
    /// (`msl_battery.rs`'s "no silent caps") exists to catch, except this
    /// path (the bytecode interpreter, not the codegen `GpuBatch`) had no
    /// battery covering it. Multiple dispatches into fresh per-chunk buffers,
    /// merged into the full-sized result — no kernel change needed.
    pub fn run(&self, probes: &[[u16; IN_STRIDE]]) -> Vec<[u16; OUT_STRIDE]> {
        if self.n_cells == 0 || probes.is_empty() {
            return Vec::new();
        }
        let total = probes.len();
        if total <= self.max_tpg {
            return self.run_chunk(probes);
        }
        let mut out = vec![[0u16; OUT_STRIDE]; self.n_cells * total];
        let mut offset = 0usize;
        while offset < total {
            let chunk_len = (total - offset).min(self.max_tpg);
            let chunk_out = self.run_chunk(&probes[offset..offset + chunk_len]);
            for cell in 0..self.n_cells {
                let src = cell * chunk_len;
                let dst = cell * total + offset;
                out[dst..dst + chunk_len].copy_from_slice(&chunk_out[src..src + chunk_len]);
            }
            offset += chunk_len;
        }
        out
    }

    /// One dispatch, `probes.len()` assumed `<= max_tpg` (the caller, [`run`],
    /// enforces this by chunking). Cell-major over just this chunk.
    fn run_chunk(&self, probes: &[[u16; IN_STRIDE]]) -> Vec<[u16; OUT_STRIDE]> {
        let flat: Vec<u16> = probes.iter().flat_map(|p| p.iter().copied()).collect();
        let probe_buf = self.device.new_buffer_with_data(
            flat.as_ptr() as *const _,
            (flat.len() * 2) as u64,
            MTLResourceOptions::StorageModeShared,
        );
        let out_buf = self.device.new_buffer(
            (self.n_cells * probes.len() * OUT_STRIDE * 2) as u64,
            MTLResourceOptions::StorageModeShared,
        );
        let n_probes = probes.len() as u32;
        let tpg = probes.len().min(self.max_tpg); // probes across lanes
        let cmd = self.queue.new_command_buffer();
        let enc = cmd.new_compute_command_encoder();
        enc.set_compute_pipeline_state(&self.pipeline);
        enc.set_buffer(0, Some(&self.code_buf), 0);
        enc.set_buffer(1, Some(&self.table_buf), 0);
        enc.set_buffer(2, Some(&probe_buf), 0);
        enc.set_buffer(3, Some(&out_buf), 0);
        enc.set_bytes(4, 4, &n_probes as *const u32 as *const _);
        enc.dispatch_thread_groups(
            MTLSize::new(self.n_cells as u64, 1, 1),
            MTLSize::new(tpg as u64, 1, 1),
        );
        enc.end_encoding();
        cmd.commit();
        cmd.wait_until_completed();
        let n = self.n_cells * probes.len();
        unsafe { std::slice::from_raw_parts(out_buf.contents() as *const [u16; OUT_STRIDE], n) }
            .to_vec()
    }
}
