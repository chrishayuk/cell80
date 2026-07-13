//! CPU emulation of the CUDA dialect — the pre-silicon validation executor.
//!
//! The emitted CUDA text is plain C++ once a small shim supplies the CUDA
//! vocabulary (`__device__`/`__global__`/`__noinline__` attributes, the
//! `blockIdx`/`threadIdx`/`blockDim` builtins as thread-loop variables, and
//! `__popc`/`__clz`/`__ffs` over the host compiler's builtins). Every
//! shift/divide corner in the emitted code is explicitly guarded (counts
//! clamped, `MIN/-1` selected out, zero-checks before every divide), so no
//! C++ undefined behavior is reachable and the host compiler is a fair
//! executor of the text's semantics.
//!
//! What this validates: the CUDA dialect's *semantics* — values, trap
//! statuses, IR-step counts, state bytes — against the same oracles the GPU
//! batteries use, before any NVIDIA hardware is rented (docs/16). What it
//! deliberately does NOT validate: NVRTC acceptance and NVIDIA codegen —
//! those remain the cloud gate's job, and no result here may be cited as
//! silicon verification.
//!
//! Mechanics: shim + module source + a file-I/O driver `main` are written to
//! a temp dir, compiled with the host C++ compiler (`$CXX`, else `c++`), and
//! run once per batch; the driver loops `blockIdx.x` over the grid serially,
//! so the sextet/state layouts are byte-identical to the real executors'.

use crate::codegen::{Dialect, GpuModule, IN_STRIDE, OUT_STRIDE};
use crate::interp::{bytecode, CellProgram};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The CUDA-vocabulary shim. Includes come first so the module's `min`
/// macro cannot mangle a system header. `__clz(0)` pins the CUDA-documented
/// 32 (the host builtin is UB on 0); `__ffs` is 1-based with `__ffs(0)==0`,
/// exactly NVIDIA's.
const SHIM: &str = r#"
#include <cstdio>
#include <cstdlib>
#include <cstring>
#define __global__
#define __noinline__ __attribute__((noinline))
#define __device__
struct EmuDim { unsigned x, y, z; };
static EmuDim blockIdx = {0, 0, 0}, threadIdx = {0, 0, 0}, blockDim = {1, 1, 1};
static inline int __popc(int x) { return __builtin_popcount((unsigned)x); }
static inline int __clz(int x) { return x == 0 ? 32 : __builtin_clz((unsigned)x); }
static inline int __ffs(int x) { return x == 0 ? 0 : __builtin_ffs(x); }
static void* emu_slurp(const char* p) {
    FILE* f = fopen(p, "rb");
    if (!f) { fprintf(stderr, "emu: open %s failed\n", p); exit(2); }
    fseek(f, 0, SEEK_END); long n = ftell(f); fseek(f, 0, SEEK_SET);
    void* b = malloc(n > 0 ? (size_t)n : 1);
    if (n > 0 && fread(b, 1, (size_t)n, f) != (size_t)n) { exit(2); }
    fclose(f); return b;
}
static void emu_spit(const char* p, const void* b, size_t n) {
    FILE* f = fopen(p, "wb");
    if (!f) { fprintf(stderr, "emu: create %s failed\n", p); exit(2); }
    if (n > 0 && fwrite(b, 1, n, f) != n) { exit(2); }
    fclose(f);
}
"#;

/// Driver for the compiled megakernel: one serial "thread" per grid index,
/// `blockDim = 1` so `tid == blockIdx.x` — the same routing the GPU does.
/// argv: inp cst stin n_inputs total stout_bytes out_path stout_path.
const MAIN_CELL: &str = r#"
int main(int argc, char** argv) {
    if (argc != 9) { fprintf(stderr, "emu: bad argv\n"); return 2; }
    const unsigned short* inp = (const unsigned short*)emu_slurp(argv[1]);
    const unsigned char* cst = (const unsigned char*)emu_slurp(argv[2]);
    const unsigned char* stin = (const unsigned char*)emu_slurp(argv[3]);
    unsigned n_inputs = (unsigned)strtoul(argv[4], 0, 10);
    unsigned total = (unsigned)strtoul(argv[5], 0, 10);
    unsigned stout_bytes = (unsigned)strtoul(argv[6], 0, 10);
    unsigned short* outp = (unsigned short*)calloc((size_t)total * 6u, 2u);
    unsigned char* stout = (unsigned char*)calloc(stout_bytes ? stout_bytes : 1u, 1u);
    for (unsigned tid = 0; tid < total; tid++) {
        blockIdx.x = tid; threadIdx.x = 0; blockDim.x = 1;
        cell_main(inp, outp, cst, n_inputs, stin, stout);
    }
    emu_spit(argv[7], outp, (size_t)total * 6u * 2u);
    emu_spit(argv[8], stout, stout_bytes ? stout_bytes : 1u);
    return 0;
}
"#;

/// Driver for the interp kernel: one block per cell, probes across "lanes",
/// serially. argv: code table probes n_probes n_cells out_path.
const MAIN_INTERP: &str = r#"
int main(int argc, char** argv) {
    if (argc != 7) { fprintf(stderr, "emu: bad argv\n"); return 2; }
    const unsigned* code = (const unsigned*)emu_slurp(argv[1]);
    const unsigned* table = (const unsigned*)emu_slurp(argv[2]);
    const unsigned short* probes = (const unsigned short*)emu_slurp(argv[3]);
    unsigned n_probes = (unsigned)strtoul(argv[4], 0, 10);
    unsigned n_cells = (unsigned)strtoul(argv[5], 0, 10);
    unsigned short* out = (unsigned short*)calloc((size_t)n_cells * n_probes * 6u, 2u);
    unsigned tpb = n_probes < 1024u ? n_probes : 1024u; // the block-width cap, mirrored
    for (unsigned cell = 0; cell < n_cells; cell++) {
        for (unsigned p = 0; p < tpb; p++) {
            blockIdx.x = cell; threadIdx.x = p;
            interp(code, table, probes, out, n_probes);
        }
    }
    emu_spit(argv[6], out, (size_t)n_cells * n_probes * 6u * 2u);
    return 0;
}
"#;

static DIR_SEQ: AtomicUsize = AtomicUsize::new(0);

fn scratch_dir() -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join(format!(
        "rustmsl-cpu-emu-{}-{}",
        std::process::id(),
        DIR_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).map_err(|e| format!("emu: mkdir: {e}"))?;
    Ok(dir)
}

/// Compile shim + kernel source + driver into an executable with the host
/// C++ compiler. `-O1`: heavy cells run ~10⁸ ticks, `-O0` is too slow to
/// execute and `-O2` too slow to compile at megakernel size.
fn build_exe(dir: &Path, source: &str, driver: &str) -> Result<PathBuf, String> {
    let src = dir.join("kernel.cpp");
    let exe = dir.join("kernel");
    std::fs::write(&src, format!("{SHIM}\n{source}\n{driver}"))
        .map_err(|e| format!("emu: write source: {e}"))?;
    let cxx = std::env::var("CXX").unwrap_or_else(|_| "c++".into());
    let out = Command::new(&cxx)
        .args(["-std=c++17", "-O1", "-w", "-o"])
        .arg(&exe)
        .arg(&src)
        .output()
        .map_err(|e| format!("emu: spawn {cxx}: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "emu: host C++ compile failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(exe)
}

fn le_bytes(words: &[u16]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_le_bytes()).collect()
}

fn words_of(bytes: &[u8]) -> Vec<u16> {
    bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect()
}

fn sextets_of(bytes: &[u8], n: usize) -> Result<Vec<[u16; OUT_STRIDE]>, String> {
    let words = words_of(bytes);
    if words.len() != n * OUT_STRIDE {
        return Err(format!(
            "emu: readback is {} words, want {}",
            words.len(),
            n * OUT_STRIDE
        ));
    }
    Ok(words
        .chunks_exact(OUT_STRIDE)
        .map(|c| <[u16; OUT_STRIDE]>::try_from(c).unwrap())
        .collect())
}

/// Run a CUDA-dialect [`GpuModule`] on the CPU — the emulation twin of
/// `CudaBatch::run`, same layout, same refusals.
pub fn run(
    module: &GpuModule,
    inputs: &[[u16; IN_STRIDE]],
) -> Result<Vec<[u16; OUT_STRIDE]>, String> {
    let state_stride: usize = module.cells.iter().map(|c| c.state_len).sum();
    if state_stride > 0 {
        return Err("emu: this module has state cells — use run_with_state".into());
    }
    Ok(dispatch(module, inputs, &[])?.0)
}

/// The emulation twin of `CudaBatch::run_with_state`.
pub fn run_with_state(
    module: &GpuModule,
    inputs: &[[u16; IN_STRIDE]],
    state_in: &[u8],
) -> Result<(Vec<[u16; OUT_STRIDE]>, Vec<u8>), String> {
    let state_stride: usize = module.cells.iter().map(|c| c.state_len).sum();
    if state_in.len() != state_stride * inputs.len() {
        return Err(format!(
            "emu: state_in is {} bytes, want {} (state stride {} × {} inputs)",
            state_in.len(),
            state_stride * inputs.len(),
            state_stride,
            inputs.len()
        ));
    }
    dispatch(module, inputs, state_in)
}

fn dispatch(
    module: &GpuModule,
    inputs: &[[u16; IN_STRIDE]],
    state_in: &[u8],
) -> Result<(Vec<[u16; OUT_STRIDE]>, Vec<u8>), String> {
    if module.dialect != Dialect::Cuda {
        return Err(format!(
            "emu: module is {:?} dialect — the CPU emulator runs the CUDA text",
            module.dialect
        ));
    }
    if inputs.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let n_in = inputs.len();
    let total = module.cells.len() * n_in;
    let state_stride: usize = module.cells.iter().map(|c| c.state_len).sum();
    let state_bytes = state_stride * n_in;

    let dir = scratch_dir()?;
    let result = (|| {
        let exe = build_exe(&dir, &module.source, MAIN_CELL)?;
        let flat_in: Vec<u16> = inputs.iter().flat_map(|t| t.iter().copied()).collect();
        let write = |name: &str, bytes: &[u8]| -> Result<PathBuf, String> {
            let p = dir.join(name);
            let data: &[u8] = if bytes.is_empty() { &[0] } else { bytes };
            std::fs::write(&p, data).map_err(|e| format!("emu: write {name}: {e}"))?;
            Ok(p)
        };
        let inp = write("inp.bin", &le_bytes(&flat_in))?;
        let cst = write("cst.bin", &module.consts)?;
        let stin = write("stin.bin", state_in)?;
        let out_path = dir.join("out.bin");
        let stout_path = dir.join("stout.bin");
        let status = Command::new(&exe)
            .args([
                inp.as_os_str(),
                cst.as_os_str(),
                stin.as_os_str(),
                n_in.to_string().as_ref(),
                total.to_string().as_ref(),
                state_bytes.to_string().as_ref(),
                out_path.as_os_str(),
                stout_path.as_os_str(),
            ])
            .status()
            .map_err(|e| format!("emu: spawn kernel exe: {e}"))?;
        if !status.success() {
            return Err(format!("emu: kernel exe failed: {status}"));
        }
        let out = std::fs::read(&out_path).map_err(|e| format!("emu: read out: {e}"))?;
        let sextets = sextets_of(&out, total)?;
        let state_out = if state_bytes > 0 {
            std::fs::read(&stout_path).map_err(|e| format!("emu: read stout: {e}"))?
        } else {
            Vec::new()
        };
        Ok((sextets, state_out))
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

/// Run linearized cells through the CUDA interp kernel's text on the CPU —
/// the emulation twin of `CudaInterpBatch`. Returns the cell-major sextets
/// and the count of cells skipped for exceeding the local-slot bound.
pub fn run_interp(
    progs: &[CellProgram],
    probes: &[[u16; IN_STRIDE]],
) -> Result<(Vec<[u16; OUT_STRIDE]>, usize), String> {
    let (code, table, skipped) = bytecode::pack(progs);
    let n_cells = table.len() / 3;
    if n_cells == 0 || probes.is_empty() {
        return Ok((Vec::new(), skipped));
    }
    let dir = scratch_dir()?;
    let result = (|| {
        let exe = build_exe(&dir, &crate::interp::interp_source_cuda(), MAIN_INTERP)?;
        let u32_bytes = |v: &[u32]| -> Vec<u8> { v.iter().flat_map(|w| w.to_le_bytes()).collect() };
        let write = |name: &str, bytes: &[u8]| -> Result<PathBuf, String> {
            let p = dir.join(name);
            let data: &[u8] = if bytes.is_empty() { &[0] } else { bytes };
            std::fs::write(&p, data).map_err(|e| format!("emu: write {name}: {e}"))?;
            Ok(p)
        };
        let code_p = write("code.bin", &u32_bytes(&code))?;
        let table_p = write("table.bin", &u32_bytes(&table))?;
        let flat: Vec<u16> = probes.iter().flat_map(|p| p.iter().copied()).collect();
        let probes_p = write("probes.bin", &le_bytes(&flat))?;
        let out_path = dir.join("out.bin");
        let status = Command::new(&exe)
            .args([
                code_p.as_os_str(),
                table_p.as_os_str(),
                probes_p.as_os_str(),
                probes.len().to_string().as_ref(),
                n_cells.to_string().as_ref(),
                out_path.as_os_str(),
            ])
            .status()
            .map_err(|e| format!("emu: spawn interp exe: {e}"))?;
        if !status.success() {
            return Err(format!("emu: interp exe failed: {status}"));
        }
        let out = std::fs::read(&out_path).map_err(|e| format!("emu: read out: {e}"))?;
        Ok((sextets_of(&out, n_cells * probes.len())?, skipped))
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}
