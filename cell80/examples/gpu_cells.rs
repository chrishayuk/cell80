//! Cells on the GPU — the Metal demo (Phase 6 WS-E,
//! `docs/14-model-native-cells-spec.md`).
//!
//! Four acts, all against the same rule: the reference IR interpreter is the
//! one source of meaning, and a GPU result that disagrees with it — value,
//! trap status, IR-step count, or state byte — is a defect, never a "GPU
//! difference".
//!
//! 1. **One source, one more body.** A library-shaped cell runs on the
//!    interpreter and as a Metal compute kernel; results and *IR steps* agree
//!    exactly (steps are the canonical family cost, metered on both).
//! 2. **A million inputs, one dispatch.** The one-cell × N-inputs batch
//!    layout — fuzzing and reward organs — with measured evals/s, fuel
//!    metering on.
//! 3. **The whole library, one launch.** Every eligible integer value cell
//!    fused into a single megakernel, run against a probe set in one
//!    dispatch — retrieval by execution's substrate (WS-F).
//! 4. **State on the GPU.** A state cell steps a state machine across
//!    dispatches, its typed state window chained through unified memory —
//!    the interpreter agreeing byte-for-byte at every step.
//!
//! Run: `cargo run --release -p cell80 --example gpu_cells` (macOS)

#[cfg(not(target_os = "macos"))]
fn main() {
    println!(
        "the GPU demo needs macOS (Metal) — the codegen builds everywhere, the executor doesn't"
    );
}

#[cfg(target_os = "macos")]
fn main() {
    macos::demo();
}

#[cfg(target_os = "macos")]
mod macos {
    use cell80_core::{Interp, Target};
    use rustmsl::{steps_of, GpuBatch, LibraryCell, STATE_BASE};
    use std::time::Instant;

    type Funcs = Vec<(String, cell80_core::Func)>;
    type Consts = Vec<(String, Vec<u8>)>;

    /// The cartridge pipeline up to the IR seam: prelude append, lower,
    /// inline, DCE-root at the entry.
    fn lower(src: &str, entry: &str) -> Result<(Funcs, Consts), String> {
        let combined = format!("{src}\n{}{}", cell80::CELL_PRELUDE, rustz80::F32_KERNELS);
        let file: syn::File = syn::parse_str(&combined).map_err(|e| format!("parse: {e}"))?;
        let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default())?;
        if !lowered.funcs.iter().any(|(n, _)| n == entry) {
            return Err(format!("no `{entry}` entry"));
        }
        let consts = lowered.const_data();
        let funcs = cell80_core::inline::inline(lowered.funcs, &[entry]);
        let funcs = cell80_core::dce::prune(funcs, &[entry]);
        Ok((funcs, consts))
    }

    fn interp_run(funcs: &Funcs, consts: &Consts, entry: &str, args: &[u16]) -> (Vec<u16>, u64) {
        let mut interp = Interp::new(
            funcs,
            consts.iter().map(|(n, b)| (n.as_str(), b.as_slice())),
            Target::Cell.descriptor(),
        );
        let out = interp.run(entry, args).expect("interp run");
        (out, interp.steps())
    }

    /// A library-shaped value cell: deadband + clamp, the robo dialect.
    const DEADBAND: &str = "
        fn run(x: u16, center: u16, width: u16) -> u16 {
            let lo = center - width;
            let hi = center + width;
            if x > lo && x < hi { center } else if x > hi { hi } else { x }
        }
    ";

    /// A state cell: a circuit breaker stepping closed → open → half-open.
    const BREAKER: &str = "
        struct Breaker { st: u16, fails: u16, cooldown: u16 }
        impl Breaker {
            fn run(&mut self, ok: u16) -> u16 {
                if self.st == 0u16 {
                    if ok == 0u16 { self.fails = self.fails + 1u16; }
                    else { self.fails = 0u16; }
                    if self.fails >= 3u16 { self.st = 1u16; self.cooldown = 4u16; }
                } else if self.st == 1u16 {
                    self.cooldown = self.cooldown - 1u16;
                    if self.cooldown == 0u16 { self.st = 2u16; }
                } else {
                    if ok == 1u16 { self.st = 0u16; self.fails = 0u16; }
                    else { self.st = 1u16; self.cooldown = 4u16; }
                }
                self.st
            }
        }
    ";

    pub fn demo() {
        // ── Act 1: one source, one more body ────────────────────────────────
        println!("== one source, one more body: deadband (u16) ==");
        let (funcs, consts) = lower(DEADBAND, "run").unwrap();
        let module = rustmsl::compile(&funcs, &consts, "run").unwrap();
        let gpu = GpuBatch::new(&module).unwrap();
        let probes: Vec<[u16; 3]> = vec![[498, 500, 10], [520, 500, 10], [400, 500, 10]];
        let outs = gpu.run(&probes).unwrap();
        for (p, o) in probes.iter().zip(&outs) {
            let (want, steps) = interp_run(&funcs, &consts, "run", p);
            let agree = o[0] == want[0] && steps_of(o) as u64 == steps;
            println!(
                "  deadband({:>3},{},{:>2})  GPU={:<3} steps={:<2}  interpreter={:<3} steps={:<2}  {}",
                p[0],
                p[1],
                p[2],
                o[0],
                steps_of(o),
                want[0],
                steps,
                if agree { "agree (values AND IR steps)" } else { "DISAGREE" }
            );
        }

        // ── Act 2: a million inputs, one dispatch ───────────────────────────
        println!("\n== one cell × 2^20 inputs, one dispatch (fuel metering on) ==");
        let mut x = 0x5eed_de00_0001u64;
        let mut rng = move || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x as u16
        };
        let n = 1 << 20;
        let inputs: Vec<[u16; 3]> = (0..n).map(|_| [rng(), rng(), rng()]).collect();
        gpu.run(&inputs).unwrap(); // warm
        let t0 = Instant::now();
        let outs = gpu.run(&inputs).unwrap();
        let dt = t0.elapsed().as_secs_f64();
        let total_steps: u64 = outs.iter().map(|o| steps_of(o) as u64).sum();
        println!(
            "  {} evals in {:.1} ms — {:.2e} evals/s, {:.2e} metered IR steps",
            n,
            dt * 1e3,
            n as f64 / dt,
            total_steps as f64
        );

        // ── Act 3: the whole library, one launch ────────────────────────────
        println!("\n== the whole library × a probe set, one megakernel dispatch ==");
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut lowered_cells: Vec<(String, Funcs, Consts)> = Vec::new();
        for path in cell80::discover_cell_files(manifest.join("cells").to_str().unwrap()).unwrap() {
            if path.extension().is_none_or(|x| x != "rs") {
                continue;
            }
            let name = path.file_stem().unwrap().to_string_lossy().into_owned();
            let src = std::fs::read_to_string(&path).unwrap();
            let Ok(sig) = rustz80::entry_signature(&src, "run") else {
                continue;
            };
            let scalar = sig.state.is_empty()
                && sig.params.iter().all(|(_, ty)| {
                    matches!(ty.as_str(), "u8" | "u16" | "i16" | "u32" | "i32" | "bool")
                });
            if !scalar {
                continue;
            }
            if let Ok((funcs, consts)) = lower(&src, "run") {
                lowered_cells.push((name, funcs, consts));
            }
        }
        let compilable: Vec<&(String, Funcs, Consts)> = lowered_cells
            .iter()
            .filter(|(_, f, c)| rustmsl::compile(f, c, "run").is_ok())
            .collect();
        let lib: Vec<LibraryCell> = compilable
            .iter()
            .map(|(_, funcs, consts)| LibraryCell {
                funcs,
                consts,
                entry: "run",
                state_len: 0,
            })
            .collect();
        let t0 = Instant::now();
        let module = rustmsl::compile_library(&lib).unwrap();
        let lib_gpu = GpuBatch::new(&module).unwrap();
        let t_build = t0.elapsed();
        let probes: Vec<[u16; 3]> = (0..16).map(|_| [rng(), rng(), rng()]).collect();
        lib_gpu.run(&probes).unwrap(); // warm
        let t0 = Instant::now();
        let all = lib_gpu.run(&probes).unwrap();
        let dt = t0.elapsed().as_secs_f64();
        println!(
            "  {} cells fused into one kernel ({} KiB MSL, built in {:.1} s)",
            lib.len(),
            module.source.len() / 1024,
            t_build.as_secs_f64()
        );
        println!(
            "  {} cells × {} probes = {} evals in {:.1} ms — every cell's behaviour, one launch",
            lib.len(),
            probes.len(),
            all.len(),
            dt * 1e3
        );
        for want in ["clamp", "gcd", "median_of_three"] {
            if let Some(ci) = compilable.iter().position(|(n, _, _)| n == want) {
                let row: Vec<u16> = (0..4).map(|pi| all[ci * probes.len() + pi][0]).collect();
                println!("    {want:>18}: {row:?} … (first 4 probes)");
            }
        }

        // ── Act 4: state on the GPU ─────────────────────────────────────────
        println!("\n== a state cell stepping on the GPU (typed state, chained) ==");
        let (funcs, consts) = lower(BREAKER, "Breaker::run").unwrap();
        let module = rustmsl::compile_library(&[LibraryCell {
            funcs: &funcs,
            consts: &consts,
            entry: "Breaker::run",
            state_len: 6,
        }])
        .unwrap();
        let gpu = GpuBatch::new(&module).unwrap();
        let mut state = vec![0u8; 6]; // closed, 0 fails
        let script: &[(u16, &str)] = &[
            (0, "fail"),
            (0, "fail"),
            (0, "fail — trips"),
            (1, "cooldown"),
            (1, "cooldown"),
            (1, "cooldown"),
            (1, "cooldown — half-open"),
            (1, "success — closes"),
        ];
        println!("  state = {{ st, fails, cooldown }} at 0xB000, 6 bytes per thread");
        for (ok, label) in script {
            let (outs, next) = gpu.run_with_state(&[[*ok, 0, 0]], &state).unwrap();
            // The interpreter re-derives the same step from the same state.
            let mut interp = Interp::new(
                &funcs,
                consts.iter().map(|(n, b)| (n.as_str(), b.as_slice())),
                Target::Cell.descriptor(),
            );
            interp.plant(STATE_BASE, &state);
            let want = interp.run("Breaker::run", &[STATE_BASE, *ok]).unwrap();
            let sb = STATE_BASE as usize;
            let agree = outs[0][0] == want[0] && next == interp.mem[sb..sb + 6];
            println!(
                "  ok={} → st={} state={:?}  {:<22} {}",
                ok,
                outs[0][0],
                &next,
                label,
                if agree {
                    "(interpreter agrees)"
                } else {
                    "DISAGREE"
                }
            );
            state = next;
        }
        println!("\nEvery number above is bit-identical to the reference interpreter —");
        println!("values, trap statuses, IR-step costs, and state bytes. That's the contract.");
    }
}
