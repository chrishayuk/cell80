//! The E1+E2 library battery (Phase 6 WS-E, docs 14): every integer **value
//! cell** in the library — straight-line (E1) and looping (E2) — compiled to
//! MSL and run on the system's Metal device against the reference
//! interpreter. The full `[r0, r1, r2, status]` quad **and the IR-step
//! count** (docs 14 Q2 — the canonical family cost, metered identically on
//! both substrates) must agree bit for bit on every input, or the cell is a
//! filed defect.
//!
//! The E3 layouts ride the same kernel shape: `library_megakernel` fuses every
//! eligible cell into one translation unit and runs the whole library × a
//! probe set in a single launch — retrieval by execution's substrate (WS-F).
//! The `throughput_*` benches (ignored) print measured evals/s; docs 14's
//! ≥10⁸/s target is benchmarked, never assumed.
//!
//! Coverage is honest, not silent (docs 14 "no silent caps"): cells outside
//! the fragment refuse at codegen with a typed reason and are *counted* —
//! f32 is E4, state cells await typed-state readback on the GPU host path
//! (owed with the host integration).
//!
//! **Oracle transcripts** (docs 12's fact-file idea applied to this gate):
//! the interpreter's verdict for a `(cell, input schedule)` is deterministic,
//! so it memoizes as a digest in `tests/golden/msl_oracle_transcripts.json`,
//! keyed by the combined source hash. A hit turns grading into GPU-run +
//! digest compare — no interpreter wall clock at all; a miss or disagreement
//! falls back to the live oracle (which stays the only authority — a stale
//! transcript costs a live grade, never a verdict). The pre-registered gate
//! (10⁶ random inputs per cell) runs — and re-blesses transcripts for new or
//! changed cells — via:
//!
//! ```sh
//! UPDATE_GOLDEN=1 cargo test --release -p cell80 --test msl_battery \
//!     -- --ignored --nocapture
//! ```
//!
//! A deliberate *interpreter semantics* change must regenerate every
//! transcript (delete the file and re-bless); the always-live corner battery
//! in `rustmsl/tests/corners.rs` guards that seam on every push.

#![cfg(target_os = "macos")]

use cell80_core::{Interp, Target};
use rustmsl::{steps_of, GpuBatch, STATUS_DIV0, STATUS_FUEL, STATUS_HALT, STATUS_OK};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The `cell_fuzz` xorshift — fixed seeds, no `rand`, fully reproducible.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn u16(&mut self) -> u16 {
        self.next() as u16
    }
}

type Funcs = Vec<(String, cell80_core::Func)>;
type Consts = Vec<(String, Vec<u8>)>;

/// The cartridge pipeline up to the IR seam (`compile_rv32`'s steps, stopping
/// where the per-target body compiler takes over): prelude append, lower,
/// inline, DCE-root at `run`. Also returns the SHA-256 of the combined source
/// — the oracle-transcript cache key (cell + prelude changes invalidate it).
fn lower_cell(src: &str) -> Result<(Funcs, Consts, String), String> {
    let combined = format!("{src}\n{}{}", cell80::CELL_PRELUDE, rustz80::F32_KERNELS);
    let file: syn::File = syn::parse_str(&combined).map_err(|e| format!("parse: {e}"))?;
    let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default())?;
    if !lowered.funcs.iter().any(|(n, _)| n == "run") {
        return Err("no free `run` entry (state cell)".into());
    }
    let consts = lowered.const_data();
    let funcs = cell80_core::inline::inline(lowered.funcs, &["run"]);
    let funcs = cell80_core::dce::prune(funcs, &["run"]);
    let src_hash = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(combined.as_bytes());
        hex(&h.finalize())
    };
    Ok((funcs, consts, src_hash))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A value cell's entry takes only scalar params — a pointer param driven with
/// a random u16 would write through wild addresses, which is the state-cell
/// harness's job (owed with the host integration), not this battery's.
fn scalar_signature(src: &str) -> bool {
    match rustz80::entry_signature(src, "run") {
        Ok(sig) => {
            sig.state.is_empty()
                && sig.params.iter().all(|(_, ty)| {
                    matches!(ty.as_str(), "u8" | "u16" | "i16" | "u32" | "i32" | "bool")
                })
        }
        Err(_) => false,
    }
}

/// What the interpreter said, folded to the GPU's quad shape.
fn interp_quad(res: Result<Vec<u16>, String>) -> Result<[u16; 4], String> {
    match res {
        Ok(v) => Ok([
            v.first().copied().unwrap_or(0),
            v.get(1).copied().unwrap_or(0),
            v.get(2).copied().unwrap_or(0),
            STATUS_OK,
        ]),
        Err(e) if e.contains("divide by zero") => Ok([0, 0, 0, STATUS_DIV0]),
        Err(e) if e.contains("fuel exhausted") => Ok([0, 0, 0, STATUS_FUEL]),
        Err(e) => e
            .strip_prefix("interp: halt(")
            .and_then(|s| s.strip_suffix(')'))
            .and_then(|s| s.parse::<u16>().ok())
            .map(|code| [code, 0, 0, STATUS_HALT])
            .ok_or(e),
    }
}

fn cell_paths() -> Vec<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut paths: Vec<PathBuf> =
        cell80::discover_cell_files(manifest.join("cells").to_str().unwrap())
            .unwrap_or_else(|e| panic!("{e}"))
            .into_iter()
            .filter(|p| p.extension().is_some_and(|x| x == "rs"))
            .collect();
    paths.sort();
    paths
}

/// An interpreter wrapper that recreates itself before fuel can run low —
/// the battery reuses one instance per block for speed, but the 100M budget
/// is per-cell-run semantics, so headroom is enforced explicitly.
struct InterpBlock<'a> {
    funcs: &'a Funcs,
    consts: &'a Consts,
    interp: Interp<'a>,
    pristine: Vec<u8>,
}

impl<'a> InterpBlock<'a> {
    fn new(funcs: &'a Funcs, consts: &'a Consts) -> Self {
        let interp = Interp::new(
            funcs,
            consts.iter().map(|(n, b)| (n.as_str(), b.as_slice())),
            Target::Cell.descriptor(),
        );
        let pristine = interp.mem.clone();
        InterpBlock {
            funcs,
            consts,
            interp,
            pristine,
        }
    }

    fn recreate(&mut self) {
        self.interp = Interp::new(
            self.funcs,
            self.consts.iter().map(|(n, b)| (n.as_str(), b.as_slice())),
            Target::Cell.descriptor(),
        );
    }

    /// One pristine run: `(quad-shaped result, steps this run)`. Memory resets
    /// between runs (cheap memcpy); the instance recreates only when the
    /// cumulative fuel spend could shortchange a heavy run — ≥ half the budget
    /// always remains, orders of magnitude above any admitted cell run.
    fn run(&mut self, args: &[u16]) -> (Result<Vec<u16>, String>, u32) {
        if self.interp.steps() > 50_000_000 {
            self.recreate();
        } else {
            self.interp.mem.copy_from_slice(&self.pristine);
        }
        let fresh = self.interp.steps() == 0;
        let s0 = self.interp.steps();
        let res = self.interp.run("run", args);
        // A fuel trap must burn the *full* budget to mirror the GPU exactly —
        // a warm instance would trap early, so regrade that input cold (only
        // runaway inputs reach this, and they cost ~1 s each regardless).
        if !fresh && matches!(&res, Err(e) if e.contains("fuel exhausted")) {
            self.recreate();
            let res = self.interp.run("run", args);
            let steps = u32::try_from(self.interp.steps()).expect("steps fit");
            return (res, steps);
        }
        let used = u32::try_from(self.interp.steps() - s0).expect("steps fit");
        (res, used)
    }
}

/// The battery's input schedule: a corner sweep, then `n` seeded-random
/// triples. Deterministic — the oracle transcript is keyed on (src, seed, n).
fn gen_inputs(n: usize, seed: u64) -> Vec<[u16; 3]> {
    let corners: &[u16] = &[0, 1, 2, 0x7F, 0x80, 0xFF, 0x100, 0x7FFF, 0x8000, 0xFFFF];
    let mut inputs = Vec::with_capacity(n + corners.len() * corners.len());
    for &a in corners {
        for &b in corners {
            inputs.push([a, b, 1]);
        }
    }
    let mut rng = Rng(seed);
    for _ in 0..n {
        inputs.push([rng.u16(), rng.u16(), rng.u16()]);
    }
    inputs
}

/// SHA-256 over an output-sextet stream (LE words, input order) — the
/// transcript identity: equal digests ⟺ every value, status, and step count
/// equal.
fn sextet_digest(outs: &[[u16; 6]]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for o in outs {
        for w in o {
            h.update(w.to_le_bytes());
        }
    }
    hex(&h.finalize())
}

/// The live oracle for one cell: grade every GPU sextet against the reference
/// interpreter — values, status, and steps. Returns the number of disagreeing
/// inputs and the full oracle sextet stream (the transcript to memoize).
///
/// The GPU graded everything in one dispatch; the *oracle* is the wall clock,
/// and it fans out — each worker grades disjoint chunks with its own
/// interpreter, so a step-heavy cell uses every core, not one.
fn grade_cell(
    name: &str,
    funcs: &Funcs,
    consts: &Consts,
    n_args: usize,
    inputs: &[[u16; 3]],
    got: &[[u16; 6]],
) -> (usize, Vec<[u16; 6]>) {
    let workers = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(8);
    let chunk = (inputs.len() / (workers * 8)).max(256);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let total_bad = std::sync::atomic::AtomicUsize::new(0);
    let parts: std::sync::Mutex<Vec<(usize, Vec<[u16; 6]>)>> = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                let mut block = InterpBlock::new(funcs, consts);
                loop {
                    let start = next.fetch_add(chunk, std::sync::atomic::Ordering::Relaxed);
                    if start >= inputs.len() {
                        break;
                    }
                    let end = (start + chunk).min(inputs.len());
                    let mut want_out = Vec::with_capacity(end - start);
                    for (args, gpu_out) in inputs[start..end].iter().zip(&got[start..end]) {
                        let (res, steps) = block.run(&args[..n_args]);
                        let want = interp_quad(res).unwrap_or_else(|e| {
                            panic!("{name}: unexpected interpreter refusal: {e}")
                        });
                        let sext = [
                            want[0],
                            want[1],
                            want[2],
                            want[3],
                            steps as u16,
                            (steps >> 16) as u16,
                        ];
                        if gpu_out != &sext {
                            let seen = total_bad.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            if seen < 5 {
                                eprintln!(
                                    "{name}: args {args:?} — gpu {gpu_out:?} != \
                                     interpreter {sext:?}"
                                );
                            }
                        }
                        want_out.push(sext);
                    }
                    parts.lock().unwrap().push((start, want_out));
                }
            });
        }
    });
    let mut parts = parts.into_inner().unwrap();
    parts.sort_by_key(|(start, _)| *start);
    let oracle: Vec<[u16; 6]> = parts.into_iter().flat_map(|(_, v)| v).collect();
    (total_bad.load(std::sync::atomic::Ordering::Relaxed), oracle)
}

/// The oracle-transcript book (docs 12's fact-file idea applied to the GPU
/// gate): one digest per `(cell, input-schedule)`, keyed by the combined
/// source hash. A cache hit turns the gate into GPU-run + digest compare — no
/// interpreter time at all; a miss or stale entry falls back to live grading.
/// Regenerate with `UPDATE_GOLDEN=1` (the write is the oracle's own output,
/// not a human judgment). A *deliberate interpreter semantics change* must
/// regenerate every transcript — the always-live corner battery in
/// `rustmsl/tests/corners.rs` still guards that seam per push.
const TRANSCRIPTS: &str = "tests/golden/msl_oracle_transcripts.json";

type Book = BTreeMap<String, serde_json::Value>;

fn load_book() -> Book {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(TRANSCRIPTS);
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_book(book: &Book) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(TRANSCRIPTS);
    let text = serde_json::to_string_pretty(book).expect("serialize transcripts");
    std::fs::write(path, text + "\n").expect("write transcripts");
}

/// Every eligible cell, lowered: `(name, funcs, consts, src_hash)`. Refusal
/// buckets and skip counts ride along so coverage is reported honestly.
struct Eligible {
    cells: Vec<(String, Funcs, Consts, String)>,
    skipped_state: usize,
    skipped_sig: usize,
}

fn eligible_cells() -> Eligible {
    let mut cells = Vec::new();
    let mut skipped_state = 0usize;
    let mut skipped_sig = 0usize;
    for path in &cell_paths() {
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let src = std::fs::read_to_string(path).unwrap();
        if !scalar_signature(&src) {
            // State cells (impl `run`, or named state fields) vs value cells
            // with pointer params — both await the typed-state GPU harness.
            if src.contains("impl ") {
                skipped_state += 1;
            } else {
                skipped_sig += 1;
            }
            continue;
        }
        match lower_cell(&src) {
            Ok((funcs, consts, src_hash)) => cells.push((name, funcs, consts, src_hash)),
            Err(e) if e.contains("state cell") => skipped_state += 1,
            Err(e) => panic!("{name}: lower failed: {e}"),
        }
    }
    Eligible {
        cells,
        skipped_state,
        skipped_sig,
    }
}

/// Sweep the library: compile every eligible cell to MSL, run the battery, and
/// report coverage + refusals. `n` random inputs per cell. Cells run in
/// sequence; a transcript hit costs one GPU dispatch + a digest compare, a
/// miss grades live with the interpreter fanned across every core inside
/// [`grade_cell`], so one step-heavy cell can't pin the sweep to a single core.
fn battery(n: usize) {
    let lib = eligible_cells();
    let mut book = load_book();
    let update = std::env::var("UPDATE_GOLDEN").is_ok();
    let mut compiled = 0usize;
    let mut clean = 0usize;
    let mut cached = 0usize;
    let mut defects: Vec<String> = Vec::new();
    let mut refusals: BTreeMap<String, usize> = BTreeMap::new();
    for (i, (name, funcs, consts, src_hash)) in lib.cells.iter().enumerate() {
        let module = match rustmsl::compile(funcs, consts, "run") {
            Ok(m) => m,
            Err(e) => {
                // Typed refusals, bucketed by reason — coverage stays honest.
                let key = if e.contains("f32") || e.contains("E4") {
                    "f32 (E4)".to_string()
                } else {
                    e
                };
                *refusals.entry(key).or_default() += 1;
                continue;
            }
        };
        compiled += 1;
        // Per-cell seed: stable across runs (the eligible list is sorted),
        // distinct across cells, order-independent.
        let seed = 0x5eed_e100_0000_0000 ^ i as u64;
        let inputs = gen_inputs(n, seed);
        let gpu = GpuBatch::new(&module)
            .unwrap_or_else(|e| panic!("{name}: gpu pipeline failed: {e}\n{}", module.source));
        let got = gpu.run(&inputs).unwrap_or_else(|e| panic!("{name}: {e}"));
        let gpu_digest = sextet_digest(&got);

        let key = format!("{name}@{n}");
        let hit = book.get(&key).is_some_and(|e| {
            e["src"].as_str() == Some(src_hash.as_str())
                && e["seed"].as_u64() == Some(seed)
                && e["digest"].as_str() == Some(gpu_digest.as_str())
        });
        if hit {
            cached += 1;
            clean += 1;
            continue;
        }
        // Miss, stale, or disagreement: the live oracle decides (and
        // localizes any disagreeing inputs).
        let (bad, oracle) = grade_cell(name, funcs, consts, module.cells[0].params, &inputs, &got);
        if bad == 0 {
            clean += 1;
        } else {
            defects.push(format!("{name}: {bad} disagreeing inputs"));
        }
        if update {
            book.insert(
                key,
                serde_json::json!({
                    "src": src_hash,
                    "seed": seed,
                    "digest": sextet_digest(&oracle),
                }),
            );
            // Save per cell, not at the end: a blessing run over the heavy
            // tail is long, and an interrupted one should keep every verdict
            // it already paid for.
            save_book(&book);
        }
    }
    println!(
        "msl E1+E2 battery: {} eligible — {compiled} compiled ({clean} clean, \
         {cached} via transcript), {} non-scalar/no-entry, {} state, refusals: {:?}",
        lib.cells.len(),
        lib.skipped_sig,
        lib.skipped_state,
        refusals
    );
    assert!(
        defects.is_empty(),
        "GPU ≠ interpreter on {} cells:\n{}",
        defects.len(),
        defects.join("\n")
    );
    // A floor so a silent regression (everything refusing) can't read as green.
    assert!(
        compiled >= 230,
        "only {compiled} cells reached the GPU — the E1+E2 fragment shrank"
    );
}

/// The CI-speed battery: every eligible cell, a corner sweep + 512 random
/// inputs each. The full pre-registered gate is [`gate_one_million`].
#[test]
fn e1_e2_battery() {
    let n = std::env::var("CELL80_MSL_FUZZ_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(512);
    battery(n);
}

/// The E1+E2 gate (docs 14): 10⁶ random inputs per admitted integer value
/// cell, values + status + steps bit-exact. Ignored by default — run in
/// release; the interpreter side dominates the wall clock.
#[test]
#[ignore = "the 10^6-input gate — run explicitly in release"]
fn gate_one_million() {
    battery(1_000_000);
}

/// E3, the library × probe-set layout: every eligible cell fused into ONE
/// translation unit, the whole library run against one probe set in ONE
/// dispatch — and every (cell, probe) sextet still agrees with the
/// interpreter. This is retrieval-by-execution's substrate (WS-F).
#[test]
fn library_megakernel_matches_interpreter() {
    let lib = eligible_cells();
    let compilable: Vec<&(String, Funcs, Consts, String)> = lib
        .cells
        .iter()
        .filter(|(_, funcs, consts, _)| rustmsl::compile(funcs, consts, "run").is_ok())
        .collect();
    let cells: Vec<rustmsl::LibraryCell> = compilable
        .iter()
        .map(|(_, funcs, consts, _)| rustmsl::LibraryCell {
            funcs,
            consts,
            entry: "run",
        })
        .collect();
    let module = rustmsl::compile_library(&cells).expect("library compile");
    let gpu = GpuBatch::new(&module).unwrap_or_else(|e| panic!("library pipeline: {e}"));

    // A probe-set-shaped input batch (the fingerprint probes' spirit: small
    // values, signed corners, a divisor-shaped third arg).
    let mut probes: Vec<[u16; 3]> = vec![
        [3, 7, 12],
        [0, 0, 1],
        [1, 1, 1],
        [100, 250, 40],
        [65531, 3, 6],
        [9000, 2500, 40],
        [0xFFFF, 0xFFFF, 0xFFFF],
        [0x8000, 2, 3],
    ];
    let mut rng = Rng(0x5eed_e300);
    for _ in 0..8 {
        probes.push([rng.u16(), rng.u16(), rng.u16()]);
    }
    let got = gpu.run(&probes).expect("library run");
    assert_eq!(got.len(), compilable.len() * probes.len());

    let mut bad = 0usize;
    for (ci, (name, funcs, consts, _)) in compilable.iter().enumerate() {
        let n_args = module.cells[ci].params;
        let mut block = InterpBlock::new(funcs, consts);
        for (pi, probe) in probes.iter().enumerate() {
            let gpu_out = &got[ci * probes.len() + pi];
            let (res, want_steps) = block.run(&probe[..n_args]);
            let want = interp_quad(res)
                .unwrap_or_else(|e| panic!("{name}: unexpected interpreter refusal: {e}"));
            let got_quad = [gpu_out[0], gpu_out[1], gpu_out[2], gpu_out[3]];
            if got_quad != want || steps_of(gpu_out) != want_steps {
                bad += 1;
                if bad <= 5 {
                    eprintln!(
                        "{name} probe {probe:?}: gpu {got_quad:?}/{} != interp {want:?}/{want_steps}",
                        steps_of(gpu_out)
                    );
                }
            }
        }
    }
    println!(
        "msl E3 megakernel: {} cells × {} probes in one dispatch, {} disagreements",
        compilable.len(),
        probes.len(),
        bad
    );
    assert_eq!(
        bad, 0,
        "megakernel ≠ interpreter on {bad} (cell, probe) pairs"
    );
}

/// E3 throughput, layout 1 (one cell × N inputs): steady-state end-to-end
/// evals/s (buffer setup + dispatch + readback included — the honest number).
#[test]
#[ignore = "throughput bench — run in release with --nocapture"]
fn throughput_one_cell() {
    let src = "fn run(x: u16, lo: u16, hi: u16) -> u16 { if x > hi { hi } else if x < lo { lo } else { x } }";
    let combined = format!("{src}\n{}{}", cell80::CELL_PRELUDE, rustz80::F32_KERNELS);
    let file: syn::File = syn::parse_str(&combined).unwrap();
    let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default()).unwrap();
    let consts = lowered.const_data();
    let funcs = cell80_core::inline::inline(lowered.funcs, &["run"]);
    let funcs = cell80_core::dce::prune(funcs, &["run"]);
    let module = rustmsl::compile(&funcs, &consts, "run").unwrap();
    let gpu = GpuBatch::new(&module).unwrap();

    let mut rng = Rng(0x5eed_e301);
    for n in [1 << 16, 1 << 20, 1 << 22, 1 << 24] {
        let inputs: Vec<[u16; 3]> = (0..n).map(|_| [rng.u16(), rng.u16(), rng.u16()]).collect();
        // Warm once, then time the steady state.
        gpu.run(&inputs).unwrap();
        let reps = 5;
        let t0 = std::time::Instant::now();
        for _ in 0..reps {
            gpu.run(&inputs).unwrap();
        }
        let dt = t0.elapsed().as_secs_f64() / reps as f64;
        println!(
            "one-cell throughput: N={n:>9} — {:>7.1} ms/launch, {:.2e} evals/s",
            dt * 1e3,
            n as f64 / dt
        );
    }
}

/// E3 throughput, layout 2 (library × probe set): whole-library launch
/// latency at fingerprint-probe scale and at retrieval scale.
#[test]
#[ignore = "throughput bench — run in release with --nocapture"]
fn throughput_library() {
    let lib = eligible_cells();
    let compilable: Vec<&(String, Funcs, Consts, String)> = lib
        .cells
        .iter()
        .filter(|(_, funcs, consts, _)| rustmsl::compile(funcs, consts, "run").is_ok())
        .collect();
    let cells: Vec<rustmsl::LibraryCell> = compilable
        .iter()
        .map(|(_, funcs, consts, _)| rustmsl::LibraryCell {
            funcs,
            consts,
            entry: "run",
        })
        .collect();
    let t0 = std::time::Instant::now();
    let module = rustmsl::compile_library(&cells).expect("library compile");
    let t_codegen = t0.elapsed();
    let t0 = std::time::Instant::now();
    let gpu = GpuBatch::new(&module).unwrap();
    let t_metal = t0.elapsed();
    println!(
        "library codegen: {} cells in {:.1} ms (MSL {} KiB), metal compile {:.1} ms",
        cells.len(),
        t_codegen.as_secs_f64() * 1e3,
        module.source.len() / 1024,
        t_metal.as_secs_f64() * 1e3
    );
    let mut rng = Rng(0x5eed_e302);
    for n_probes in [8usize, 64, 512] {
        let probes: Vec<[u16; 3]> = (0..n_probes)
            .map(|_| [rng.u16(), rng.u16(), rng.u16()])
            .collect();
        gpu.run(&probes).unwrap();
        let reps = 10;
        let t0 = std::time::Instant::now();
        for _ in 0..reps {
            gpu.run(&probes).unwrap();
        }
        let dt = t0.elapsed().as_secs_f64() / reps as f64;
        let evals = cells.len() * n_probes;
        println!(
            "library×probes: {} cells × {n_probes:>3} probes = {evals:>7} evals — \
             {:>6.2} ms/launch, {:.2e} evals/s",
            cells.len(),
            dt * 1e3,
            evals as f64 / dt
        );
    }
}

/// The E2 divergence probe: gcd (data-dependent loop count) with uniform vs
/// shuffled random inputs. The hypothesis under test (docs 14, E2):
/// WCET-friendly ≈ SIMT-friendly — divergence costs what the *worst lane*
/// costs, so the ratio should track max/mean steps, not explode.
#[test]
#[ignore = "divergence probe — run in release with --nocapture"]
fn divergence_probe_gcd() {
    let src = "fn run(a: u16, b: u16) -> u16 { let mut x = a; let mut y = b; while y != 0 { let t = x % y; x = y; y = t; } x }";
    let file: syn::File = syn::parse_str(src).unwrap();
    let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default()).unwrap();
    let consts = lowered.const_data();
    let module = rustmsl::compile(&lowered.funcs, &consts, "run").unwrap();
    let gpu = GpuBatch::new(&module).unwrap();

    let n = 1 << 22;
    let mut rng = Rng(0x5eed_e303);
    // Worst-case-heavy random pairs (gcd of consecutive Fibonaccis is the WCET
    // shape; random uniforms already spread loop counts 1..~40).
    let random: Vec<[u16; 3]> = (0..n).map(|_| [rng.u16(), rng.u16(), 0]).collect();
    // Uniform: every lane the same (fib pair — deep but identical count).
    let uniform: Vec<[u16; 3]> = (0..n).map(|_| [46368, 28657, 0]).collect();

    for (label, inputs) in [
        ("uniform (deep)", &uniform),
        ("random (divergent)", &random),
    ] {
        gpu.run(inputs).unwrap();
        let reps = 5;
        let t0 = std::time::Instant::now();
        let mut out = Vec::new();
        for _ in 0..reps {
            out = gpu.run(inputs).unwrap();
        }
        let dt = t0.elapsed().as_secs_f64() / reps as f64;
        let (mut sum, mut max) = (0u64, 0u32);
        for o in &out {
            let s = steps_of(o);
            sum += s as u64;
            max = max.max(s);
        }
        println!(
            "divergence gcd {label:>18}: {:>6.2} ms/launch, {:.2e} evals/s, steps mean {:.1} max {max}",
            dt * 1e3,
            n as f64 / dt,
            sum as f64 / n as f64
        );
    }
}

/// Diagnostic: project the blessing gate's oracle cost per cell from the
/// GPU's own step counts (512-input sample × 10⁶/sample scaling). Prints the
/// heaviest cells and the cumulative distribution in processing order — the
/// map of where a blessing run's wall clock goes.
#[test]
#[ignore = "cost-map diagnostic — run with --nocapture"]
fn gate_cost_estimate() {
    let lib = eligible_cells();
    let mut rows: Vec<(String, u64)> = Vec::new(); // (name, mean steps per input)
    for (i, (name, funcs, consts, _)) in lib.cells.iter().enumerate() {
        let Ok(module) = rustmsl::compile(funcs, consts, "run") else {
            continue;
        };
        let seed = 0x5eed_e100_0000_0000 ^ i as u64;
        let inputs = gen_inputs(512, seed);
        let gpu = GpuBatch::new(&module).unwrap();
        let got = gpu.run(&inputs).unwrap();
        let total: u64 = got.iter().map(|o| steps_of(o) as u64).sum();
        rows.push((name.clone(), total / inputs.len() as u64));
    }
    let grand: u64 = rows.iter().map(|(_, m)| m * 1_000_000).sum();
    println!(
        "projected gate oracle cost: {:.2e} ticks total",
        grand as f64
    );
    let mut cum = 0u64;
    for (name, mean) in &rows {
        cum += mean * 1_000_000;
        if *mean > 10_000 {
            println!(
                "  {name:>28}: {mean:>9} steps/input — cumulative {:5.1}%",
                100.0 * cum as f64 / grand as f64
            );
        }
    }
    let mut top: Vec<_> = rows.iter().collect();
    top.sort_by_key(|(_, m)| std::cmp::Reverse(*m));
    println!("top 10 heaviest:");
    for (name, mean) in top.iter().take(10) {
        println!("  {name:>28}: {mean} steps/input");
    }
}
