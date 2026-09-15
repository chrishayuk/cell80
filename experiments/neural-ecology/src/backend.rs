use crate::{
    digest,
    model::{reference_delta, reference_node, Node},
    Result,
};
use cell80_core::{Interp, Target};
use std::{fs, path::Path, process::Command};

pub const NODE_SOURCE: &str = include_str!("../cells/node.rs");
pub const EDGE_SOURCE: &str = include_str!("../cells/plasticity.rs");
type BatchFn = unsafe extern "C" fn(*const u16, *mut u16, *const u8, u32, *const u8, *mut u8);

// The generated CUDA-dialect code is executed as C++ on the CPU, using the same
// vocabulary shim as rustmsl::cpu_emu. This is not NVIDIA hardware execution.
const SHIM: &str = r#"
#include <cstdint>
#include <cstring>
#define __global__
#define __device__
#define __noinline__ __attribute__((noinline))
struct EmuDim { unsigned x,y,z; };
static EmuDim blockIdx={0,0,0},threadIdx={0,0,0},blockDim={1,1,1};
static inline int __popc(int x) { return __builtin_popcount((unsigned)x); }
static inline int __clz(int x) { return x==0 ? 32 : __builtin_clz((unsigned)x); }
static inline int __ffs(int x) { return x==0 ? 0 : __builtin_ffs(x); }
"#;
const WRAPPER: &str = r#"
extern "C" void ne0_batch(const unsigned short* inp, unsigned short* out,
 const unsigned char* cst, unsigned n, const unsigned char* stin, unsigned char* stout) {
 for (unsigned i=0;i<n;i++) {
   blockIdx.x=i; threadIdx.x=0; blockDim.x=1;
   cell_main(inp,out,cst,n,stin,stout);
 }
}
"#;

pub struct Native {
    _library: libloading::Library,
    call: BatchFn,
    consts: Vec<u8>,
}
impl Native {
    fn new(module: &rustmsl::GpuModule, dir: &Path, name: &str) -> Result<Self> {
        fs::create_dir_all(dir)?;
        let source = format!("{SHIM}\n{}\n{WRAPPER}", module.source);
        let key = digest(source.as_bytes());
        let cpp = dir.join(format!("{name}-{key}.cpp"));
        let lib = dir.join(format!("{name}-{key}.dylib"));
        if !lib.exists() || !cpp.exists() || fs::read_to_string(&cpp)? != source {
            fs::write(&cpp, &source)?;
            let temporary = lib.with_extension(format!("{}.tmp.dylib", std::process::id()));
            let output = Command::new("c++")
                .args([
                    "-std=c++17",
                    "-O3",
                    "-fno-fast-math",
                    "-shared",
                    "-fPIC",
                    "-o",
                ])
                .arg(&temporary)
                .arg(&cpp)
                .output()?;
            if !output.status.success() {
                return Err(format!(
                    "native cell compile: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
            fs::rename(temporary, &lib)?;
        }
        // SAFETY: locally generated, compiled code; fixed wrapper signature;
        // Library is retained for the entire lifetime of the copied function.
        let library = unsafe { libloading::Library::new(fs::canonicalize(&lib)?) }?;
        let call = unsafe { *library.get::<BatchFn>(b"ne0_batch\0")? };
        let consts = if module.consts.is_empty() {
            vec![0]
        } else {
            module.consts.clone()
        };
        Ok(Self {
            _library: library,
            call,
            consts,
        })
    }
    fn run(&self, inputs: &[[u16; 3]], states: &[u8], stride: usize) -> (Vec<[u16; 6]>, Vec<u8>) {
        assert_eq!(states.len(), stride * inputs.len());
        assert!(inputs.len() <= u32::MAX as usize);
        let mut out = vec![[0u16; 6]; inputs.len()];
        let mut st = vec![0u8; states.len().max(1)];
        let zero = [0u8];
        let stin = if states.is_empty() { &zero[..] } else { states };
        // SAFETY: all slices meet the generated single-cell kernel's exact ABI;
        // execution is sequential (the shim's synthetic grid globals are private).
        unsafe {
            (self.call)(
                inputs.as_ptr().cast(),
                out.as_mut_ptr().cast(),
                self.consts.as_ptr(),
                inputs.len() as u32,
                stin.as_ptr(),
                st.as_mut_ptr(),
            )
        };
        st.truncate(states.len());
        (out, st)
    }
}

pub struct Cell {
    lowered: rustz80::Lowered,
    entry: &'static str,
    stride: usize,
    pub native: Native,
    #[cfg(target_os = "macos")]
    pub metal: rustmsl::GpuBatch,
}
impl Cell {
    fn new(src: &str, entry: &'static str, stride: usize, dir: &Path, name: &str) -> Result<Self> {
        let file = syn::parse_str(src)?;
        let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default())?;
        let consts = lowered.const_data();
        let cells = [rustmsl::LibraryCell {
            funcs: &lowered.funcs,
            consts: &consts,
            entry,
            state_len: stride,
        }];
        let cuda = rustmsl::compile_library_cuda(&cells)?;
        let msl = rustmsl::compile_library(&cells)?;
        fs::create_dir_all(dir)?;
        fs::write(dir.join(format!("{name}.metal")), &msl.source)?;
        fs::write(
            dir.join(format!("{name}.ir.txt")),
            format!("{:#?}", lowered.funcs),
        )?;
        let native = Native::new(&cuda, dir, name)?;
        #[cfg(target_os = "macos")]
        let metal = rustmsl::GpuBatch::new(&msl)?;
        #[cfg(not(target_os = "macos"))]
        return Err("NE-0 requires Metal; CPU-only execution cannot pass G0".into());
        Ok(Self {
            lowered,
            entry,
            stride,
            native,
            #[cfg(target_os = "macos")]
            metal,
        })
    }
    pub fn run(
        &self,
        mode: Mode,
        inputs: &[[u16; 3]],
        states: &[u8],
    ) -> Result<(Vec<[u16; 6]>, Vec<u8>)> {
        assert_eq!(states.len(), self.stride * inputs.len());
        let output = match mode {
            Mode::Cpu => self.native.run(inputs, states, self.stride),
            Mode::Metal => {
                #[cfg(target_os = "macos")]
                {
                    objc::rc::autoreleasepool(|| self.metal.run_with_state(inputs, states))?
                }
                #[cfg(not(target_os = "macos"))]
                {
                    return Err("Metal unavailable".into());
                }
            }
            Mode::Ir => {
                let consts = self.lowered.const_data();
                let mut out = Vec::with_capacity(inputs.len());
                let mut st = Vec::with_capacity(states.len());
                // A fresh interpreter per input also checks that no scratch bytes leak.
                for (i, args) in inputs.iter().enumerate() {
                    let mut vm = Interp::new(
                        &self.lowered.funcs,
                        consts.iter().map(|(n, b)| (n.as_str(), b.as_slice())),
                        Target::Cell.descriptor(),
                    );
                    let call = if self.stride > 0 {
                        vm.plant(
                            rustmsl::STATE_BASE,
                            &states[i * self.stride..(i + 1) * self.stride],
                        );
                        vec![rustmsl::STATE_BASE, args[0]]
                    } else {
                        args.to_vec()
                    };
                    let result = vm.run(self.entry, &call)?;
                    let steps = vm.steps();
                    let mut row = [0u16; 6];
                    for (r, v) in row[..3].iter_mut().zip(result) {
                        *r = v;
                    }
                    row[4] = steps as u16;
                    row[5] = (steps >> 16) as u16;
                    out.push(row);
                    st.extend_from_slice(
                        &vm.mem[rustmsl::STATE_BASE as usize
                            ..rustmsl::STATE_BASE as usize + self.stride],
                    );
                }
                (out, st)
            }
            Mode::Reference => return Err("reference uses independent arithmetic".into()),
        };
        if let Some((i, row)) = output
            .0
            .iter()
            .enumerate()
            .find(|(_, r)| r[3] != rustmsl::STATUS_OK)
        {
            return Err(format!(
                "cell trap entry={} mode={mode:?} row={i} input={:?} result={row:?}",
                self.entry, inputs[i]
            )
            .into());
        }
        Ok(output)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Mode {
    Reference,
    Cpu,
    Metal,
    Ir,
}
pub struct Cells {
    pub node: Cell,
    pub edge: Cell,
}
impl Cells {
    pub fn new(dir: &Path) -> Result<Self> {
        Ok(Self {
            node: Cell::new(NODE_SOURCE, "Node::run", 8, dir, "node")?,
            edge: Cell::new(EDGE_SOURCE, "run", 0, dir, "plasticity")?,
        })
    }
    pub fn nodes(&self, mode: Mode, old: &[Node], currents: &[i64]) -> Result<Vec<Node>> {
        assert_eq!(old.len(), currents.len());
        if matches!(mode, Mode::Reference) {
            return Ok(old
                .iter()
                .zip(currents)
                .map(|(&n, &i)| reference_node(n, i))
                .collect());
        }
        let mut inputs = Vec::with_capacity(old.len());
        let mut bytes = Vec::with_capacity(old.len() * 6);
        for (&n, &i) in old.iter().zip(currents) {
            assert!(n.valid() && (-512..=560).contains(&i));
            inputs.push([i16::try_from(i)? as u16, 0, 0]);
            bytes.extend_from_slice(&n.abi_bytes());
        }
        let (out, state) = self.node.run(mode, &inputs, &bytes)?;
        let nodes: Vec<_> = state.chunks_exact(8).map(Node::from_abi).collect();
        if nodes
            .iter()
            .zip(&out)
            .any(|(n, r)| !n.valid() || n.s as u16 != r[0])
        {
            return Err("invalid node output".into());
        }
        Ok(nodes)
    }
    pub fn deltas(
        &self,
        mode: Mode,
        old: &[Node],
        new: &[Node],
        sources: &[usize],
    ) -> Result<Vec<i16>> {
        if matches!(mode, Mode::Reference) {
            return Ok(sources
                .iter()
                .enumerate()
                .map(|(e, &j)| reference_delta(old[j], old[e / 16], new[e / 16]))
                .collect());
        }
        let inputs: Vec<_> = sources
            .iter()
            .enumerate()
            .map(|(e, &j)| {
                let i = e / 16;
                [
                    old[j].q,
                    old[i].q,
                    old[j].s as u16 | ((new[i].s as u16) << 1),
                ]
            })
            .collect();
        let (out, _) = self.edge.run(mode, &inputs, &[])?;
        let ds: Vec<_> = out.iter().map(|r| r[0] as i16).collect();
        if ds.iter().any(|d| !(-1..=1).contains(d)) {
            return Err("invalid plasticity output".into());
        }
        Ok(ds)
    }
}
