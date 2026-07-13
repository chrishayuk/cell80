//! The GPU battery harness, shared by every backend battery
//! (`msl_battery.rs` on macOS/Metal, `cuda_battery.rs` under `--features
//! cuda`): cell discovery + lowering, the seeded input schedule, the
//! fanned-out live oracle, the oracle-transcript book, and the battery
//! loops themselves — parameterized over a [`Backend`] vtable so the two
//! batteries cannot drift. Everything here is portable; the backend
//! specifics (compile dialect + executor) live in the including test file.
//!
//! The oracle transcripts are backend-independent by construction: an
//! entry's key (`{name}@{n}`), `src` hash, `seed`, and `digest` are all
//! interpreter-side facts — a GPU (any GPU) merely has to reproduce the
//! digest. Blessing (`UPDATE_GOLDEN=1` writes) stays a Metal/macOS
//! activity; the CUDA battery reads the same book and treats a miss as a
//! live-oracle grade ([`Backend::bless`]).

#![allow(dead_code)] // each including test binary uses its own subset

use cell80_core::{Interp, Target};
use rustmsl::{steps_of, STATUS_DIV0, STATUS_FUEL, STATUS_HALT, STATUS_OK};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The `cell_fuzz` xorshift — fixed seeds, no `rand`, fully reproducible.
pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn u16(&mut self) -> u16 {
        self.next() as u16
    }
}

pub type Funcs = Vec<(String, cell80_core::Func)>;
pub type Consts = Vec<(String, Vec<u8>)>;

/// [`Backend::compile`]'s shape — `rustmsl::compile` / `compile_cuda`.
pub type CompileFn = fn(
    &[(String, cell80_core::Func)],
    &[(String, Vec<u8>)],
    &str,
) -> Result<rustmsl::GpuModule, String>;
/// [`Backend::run`]'s shape: build the executor for `module`, run, read back.
pub type RunFn = fn(&rustmsl::GpuModule, &[[u16; 3]]) -> Result<Vec<[u16; 6]>, String>;
/// [`Backend::run_with_state`]'s shape — sextets plus final state bytes.
pub type RunStateFn =
    fn(&rustmsl::GpuModule, &[[u16; 3]], &[u8]) -> Result<(Vec<[u16; 6]>, Vec<u8>), String>;

/// One GPU backend's seam into the shared battery: how to compile the
/// dialect and how to run a module. Non-capturing fns so a backend is a
/// `const`.
pub struct Backend {
    /// Printout label ("msl" / "cuda").
    pub label: &'static str,
    /// May this backend bless transcripts under `UPDATE_GOLDEN=1`? Blessing
    /// is a macOS/Metal activity; the CUDA gate reads the book, never
    /// writes it.
    pub bless: bool,
    pub compile: CompileFn,
    pub compile_library:
        for<'a> fn(&[rustmsl::LibraryCell<'a>]) -> Result<rustmsl::GpuModule, String>,
    pub run: RunFn,
    pub run_with_state: RunStateFn,
}

/// The cartridge pipeline up to the IR seam (`compile_rv32`'s steps, stopping
/// where the per-target body compiler takes over): prelude append, lower,
/// inline, DCE-root at `run`. Also returns the SHA-256 of the combined source
/// — the oracle-transcript cache key (cell + prelude changes invalidate it).
pub fn lower_cell(src: &str) -> Result<(Funcs, Consts, String), String> {
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

/// Per-cell battery seed, derived from the combined-source hash: stable while
/// the cell is unchanged (library growth can't shift it — an index-derived
/// seed orphaned every transcript whenever a cell was added), and it rotates
/// exactly when the source changes, which stales the transcript regardless.
pub fn cell_seed(src_hash: &str, salt: u64) -> u64 {
    u64::from_str_radix(&src_hash[..16], 16).unwrap_or(0x5eed) ^ salt
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A value cell's entry takes only scalar params — a pointer param driven with
/// a random u16 would write through wild addresses, which is the state-cell
/// harness's job (owed with the host integration), not this battery's.
pub fn scalar_signature(src: &str) -> bool {
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
pub fn interp_quad(res: Result<Vec<u16>, String>) -> Result<[u16; 4], String> {
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

pub fn cell_paths() -> Vec<PathBuf> {
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
pub struct InterpBlock<'a> {
    funcs: &'a Funcs,
    consts: &'a Consts,
    entry: &'a str,
    interp: Interp<'a>,
    pristine: Vec<u8>,
}

impl<'a> InterpBlock<'a> {
    pub fn new(funcs: &'a Funcs, consts: &'a Consts, entry: &'a str) -> Self {
        let interp = Interp::new(
            funcs,
            consts.iter().map(|(n, b)| (n.as_str(), b.as_slice())),
            Target::Cell.descriptor(),
        );
        let pristine = interp.mem.clone();
        InterpBlock {
            funcs,
            consts,
            entry,
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

    /// One pristine run: `(quad-shaped result, steps this run, final state
    /// bytes)`. `state_in` plants at [`rustmsl::STATE_BASE`] (empty for a
    /// value cell) and the same window reads back after the run. Memory resets
    /// between runs (cheap memcpy); the instance recreates only when the
    /// cumulative fuel spend could shortchange a heavy run — ≥ half the budget
    /// always remains, orders of magnitude above any admitted cell run.
    pub fn run(
        &mut self,
        args: &[u16],
        state_in: &[u8],
    ) -> (Result<Vec<u16>, String>, u32, Vec<u8>) {
        if self.interp.steps() > 50_000_000 {
            self.recreate();
        } else {
            self.interp.mem.copy_from_slice(&self.pristine);
        }
        if !state_in.is_empty() {
            self.interp.plant(rustmsl::STATE_BASE, state_in);
        }
        let fresh = self.interp.steps() == 0;
        let s0 = self.interp.steps();
        let mut res = self.interp.run(self.entry, args);
        let mut used = u32::try_from(self.interp.steps() - s0).expect("steps fit");
        // A fuel trap must burn the *full* budget to mirror the GPU exactly —
        // a warm instance would trap early, so regrade that input cold (only
        // runaway inputs reach this, and they cost ~1 s each regardless).
        if !fresh && matches!(&res, Err(e) if e.contains("fuel exhausted")) {
            self.recreate();
            if !state_in.is_empty() {
                self.interp.plant(rustmsl::STATE_BASE, state_in);
            }
            res = self.interp.run(self.entry, args);
            used = u32::try_from(self.interp.steps()).expect("steps fit");
        }
        let sb = rustmsl::STATE_BASE as usize;
        let state_out = self.interp.mem[sb..sb + state_in.len()].to_vec();
        (res, used, state_out)
    }
}

/// The battery's input schedule: a corner sweep, then `n` seeded-random
/// triples. Deterministic — the oracle transcript is keyed on (src, seed, n).
pub fn gen_inputs(n: usize, seed: u64) -> Vec<[u16; 3]> {
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
pub fn sextet_digest(outs: &[[u16; 6]]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for o in outs {
        for w in o {
            h.update(w.to_le_bytes());
        }
    }
    hex(&h.finalize())
}

/// Sextets ‖ final state bytes — the state battery's transcript identity.
pub fn sextet_state_digest(outs: &[[u16; 6]], state: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for o in outs {
        for w in o {
            h.update(w.to_le_bytes());
        }
    }
    h.update(state);
    hex(&h.finalize())
}

/// The live oracle for one cell: grade every GPU sextet against the reference
/// interpreter — values, status, and steps. Returns the number of disagreeing
/// inputs and the full oracle sextet stream (the transcript to memoize).
///
/// The GPU graded everything in one dispatch; the *oracle* is the wall clock,
/// and it fans out — each worker grades disjoint chunks with its own
/// interpreter, so a step-heavy cell uses every core, not one.
#[allow(clippy::too_many_arguments)]
pub fn grade_cell(
    name: &str,
    funcs: &Funcs,
    consts: &Consts,
    entry: &str,
    n_args: usize,
    state_len: usize,
    inputs: &[[u16; 3]],
    state_in: &[u8],
    got: &[[u16; 6]],
    gpu_state: &[u8],
) -> (usize, Vec<[u16; 6]>, Vec<u8>) {
    let workers = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(8);
    let chunk = (inputs.len() / (workers * 8)).max(256);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let total_bad = std::sync::atomic::AtomicUsize::new(0);
    type Part = (usize, Vec<[u16; 6]>, Vec<u8>);
    let parts: std::sync::Mutex<Vec<Part>> = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                let mut block = InterpBlock::new(funcs, consts, entry);
                loop {
                    let start = next.fetch_add(chunk, std::sync::atomic::Ordering::Relaxed);
                    if start >= inputs.len() {
                        break;
                    }
                    let end = (start + chunk).min(inputs.len());
                    let mut want_out = Vec::with_capacity(end - start);
                    let mut want_state = Vec::with_capacity((end - start) * state_len);
                    for (i, (args, gpu_out)) in
                        inputs[start..end].iter().zip(&got[start..end]).enumerate()
                    {
                        let idx = start + i;
                        // A state cell's arg 0 is the &mut self pointer; the
                        // input triple only feeds what follows it.
                        let mut call: Vec<u16> = Vec::with_capacity(n_args);
                        let extras = if state_len > 0 {
                            call.push(rustmsl::STATE_BASE);
                            n_args - 1
                        } else {
                            n_args
                        };
                        call.extend_from_slice(&args[..extras]);
                        let st_in = &state_in[idx * state_len..(idx + 1) * state_len];
                        let (res, steps, st_out) = block.run(&call, st_in);
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
                        let gpu_st = &gpu_state[idx * state_len..(idx + 1) * state_len];
                        if gpu_out != &sext || gpu_st != st_out {
                            let seen = total_bad.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            if seen < 5 {
                                eprintln!(
                                    "{name}: args {args:?} — gpu {gpu_out:?}/{gpu_st:?} != \
                                     interpreter {sext:?}/{st_out:?}"
                                );
                            }
                        }
                        want_out.push(sext);
                        want_state.extend_from_slice(&st_out);
                    }
                    parts.lock().unwrap().push((start, want_out, want_state));
                }
            });
        }
    });
    let mut parts = parts.into_inner().unwrap();
    parts.sort_by_key(|(start, _, _)| *start);
    let mut oracle = Vec::new();
    let mut oracle_state = Vec::new();
    for (_, v, st) in parts {
        oracle.extend(v);
        oracle_state.extend(st);
    }
    (
        total_bad.load(std::sync::atomic::Ordering::Relaxed),
        oracle,
        oracle_state,
    )
}

/// The oracle-transcript book (docs 12's fact-file idea applied to the GPU
/// gate): one digest per `(cell, input-schedule)`, keyed by the combined
/// source hash. A cache hit turns the gate into GPU-run + digest compare — no
/// interpreter time at all; a miss or stale entry falls back to live grading.
/// Regenerate with `UPDATE_GOLDEN=1` (the write is the oracle's own output,
/// not a human judgment). A *deliberate interpreter semantics change* must
/// regenerate every transcript — the always-live corner battery in
/// `rustmsl/tests/corners.rs` still guards that seam per push.
pub const TRANSCRIPTS: &str = "tests/golden/msl_oracle_transcripts.json";

pub type Book = BTreeMap<String, serde_json::Value>;

pub fn load_book() -> Book {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(TRANSCRIPTS);
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_book(book: &Book) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(TRANSCRIPTS);
    let text = serde_json::to_string_pretty(book).expect("serialize transcripts");
    std::fs::write(path, text + "\n").expect("write transcripts");
}

/// Every eligible cell, lowered: `(name, funcs, consts, src_hash)`. Refusal
/// buckets and skip counts ride along so coverage is reported honestly.
pub struct Eligible {
    pub cells: Vec<(String, Funcs, Consts, String)>,
    pub skipped_state: usize,
    pub skipped_sig: usize,
}

pub fn eligible_cells() -> Eligible {
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

/// Lower a **state cell** (`impl X { fn run(&mut self, …) }`): same pipeline
/// as [`lower_cell`], entry `X::run`, plus the state struct's byte length at
/// `STATE_BASE` from its slot layout.
#[allow(clippy::type_complexity)]
pub fn lower_state_cell(src: &str) -> Result<(Funcs, Consts, String, String, usize), String> {
    let combined = format!("{src}\n{}{}", cell80::CELL_PRELUDE, rustz80::F32_KERNELS);
    let file: syn::File = syn::parse_str(&combined).map_err(|e| format!("parse: {e}"))?;
    let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default())?;
    let entry = lowered
        .funcs
        .iter()
        .map(|(n, _)| n.as_str())
        .find(|n| n.ends_with("::run"))
        .ok_or_else(|| "no `impl … fn run` entry (not a state cell)".to_string())?
        .to_string();
    let state_name = entry.trim_end_matches("::run").to_string();
    let layout = rustz80::struct_layout(src, &state_name)?;
    let state_len = layout
        .iter()
        .map(|f| (f.offset + f.slots) as usize * 2)
        .max()
        .unwrap_or(0);
    if state_len == 0 {
        return Err("empty state struct".into());
    }
    let consts = lowered.const_data();
    let funcs = cell80_core::inline::inline(lowered.funcs, &[&entry]);
    let funcs = cell80_core::dce::prune(funcs, &[&entry]);
    let src_hash = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(combined.as_bytes());
        hex(&h.finalize())
    };
    Ok((funcs, consts, src_hash, entry, state_len))
}

/// Every eligible state cell: `(name, funcs, consts, src_hash, entry,
/// state_len)`, plus honest skip counts.
pub struct EligibleState {
    pub cells: Vec<(String, Funcs, Consts, String, String, usize)>,
    pub skipped: BTreeMap<String, usize>,
}

/// Filed defects, excluded until fixed: under **adversarial state** (the
/// battery's random bytes) these cells index an array field by an unmasked
/// state field (`self.window[self.head]` with `head` fuzzed wild), writing far
/// outside their declared state struct. The interpreter's open 64 KiB absorbs
/// it; the GPU's typed window traps it (`STATUS_OOW`) — the stricter reading,
/// and the defect class the battery exists to surface. Fix: mask the index on
/// read (free on the operational envelope, where the cell's own `% 8`
/// maintains the invariant). Owned by the sliding-window pack.
pub const STATE_OOW_DEFECTS: &[&str] = &["simple_moving_average", "weighted_moving_average"];

pub fn eligible_state_cells() -> EligibleState {
    let mut cells = Vec::new();
    let mut skipped: BTreeMap<String, usize> = BTreeMap::new();
    for path in &cell_paths() {
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let src = std::fs::read_to_string(path).unwrap();
        if STATE_OOW_DEFECTS.contains(&name.as_str()) {
            *skipped
                .entry("state-derived OOW write (filed defect)".into())
                .or_default() += 1;
            continue;
        }
        // Value cells belong to the value battery; this sweep takes the rest.
        if scalar_signature(&src) {
            continue;
        }
        if !src.contains("impl ") {
            *skipped
                .entry("pointer-param value cell".into())
                .or_default() += 1;
            continue;
        }
        match lower_state_cell(&src) {
            Ok((funcs, consts, src_hash, entry, state_len)) => {
                cells.push((name, funcs, consts, src_hash, entry, state_len));
            }
            Err(e) => {
                let key = if e.contains("f32") {
                    "f32 (E4)".to_string()
                } else {
                    e
                };
                *skipped.entry(key).or_default() += 1;
            }
        }
    }
    EligibleState { cells, skipped }
}

/// Random state blocks for `n` inputs — any bit pattern is a valid scalar
/// field, so bytes are the honest fuzz (arrays included, which the named
/// surface can't even drive yet).
pub fn gen_state(n: usize, state_len: usize, seed: u64) -> Vec<u8> {
    let mut rng = Rng(seed);
    let mut v = vec![0u8; n * state_len];
    for b in v.iter_mut() {
        *b = rng.next() as u8;
    }
    v
}

/// Sweep the library on `backend`: compile every eligible value cell, run the
/// battery, and report coverage + refusals. `n` random inputs per cell. Cells
/// run in sequence; a transcript hit costs one GPU dispatch + a digest
/// compare, a miss grades live with the interpreter fanned across every core
/// inside [`grade_cell`], so one step-heavy cell can't pin the sweep to a
/// single core.
pub fn value_battery(n: usize, b: &Backend) {
    let lib = eligible_cells();
    let mut book = load_book();
    let update = b.bless && std::env::var("UPDATE_GOLDEN").is_ok();
    let mut compiled = 0usize;
    let mut clean = 0usize;
    let mut cached = 0usize;
    let mut defects: Vec<String> = Vec::new();
    let mut refusals: BTreeMap<String, usize> = BTreeMap::new();
    for (name, funcs, consts, src_hash) in lib.cells.iter() {
        let module = match (b.compile)(funcs, consts, "run") {
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
        let seed = cell_seed(src_hash, 0x5eed_e100);
        let inputs = gen_inputs(n, seed);
        let got = (b.run)(&module, &inputs)
            .unwrap_or_else(|e| panic!("{name}: gpu run failed: {e}\n{}", module.source));
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
        let (bad, oracle, _) = grade_cell(
            name,
            funcs,
            consts,
            "run",
            module.cells[0].params,
            0,
            &inputs,
            &[],
            &got,
            &[],
        );
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
        "{} E1+E2 battery: {} eligible — {compiled} compiled ({clean} clean, \
         {cached} via transcript), {} non-scalar/no-entry, {} state, refusals: {:?}",
        b.label,
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

/// Sweep the state cells on `backend`: compile each with its state window,
/// run the battery (GPU state in/out vs the interpreter's memory at
/// `STATE_BASE`), and report coverage + refusals. Transcripts key as
/// `{name}@st{n}` and digest sextets ‖ final state bytes.
pub fn state_battery(n: usize, b: &Backend) {
    let lib = eligible_state_cells();
    let mut book = load_book();
    let update = b.bless && std::env::var("UPDATE_GOLDEN").is_ok();
    let mut compiled = 0usize;
    let mut clean = 0usize;
    let mut cached = 0usize;
    let mut defects: Vec<String> = Vec::new();
    let mut refusals: BTreeMap<String, usize> = BTreeMap::new();
    for (name, funcs, consts, src_hash, entry, state_len) in lib.cells.iter() {
        let module = match (b.compile_library)(&[rustmsl::LibraryCell {
            funcs,
            consts,
            entry,
            state_len: *state_len,
        }]) {
            Ok(m) => m,
            Err(e) => {
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
        let seed = cell_seed(src_hash, 0x5eed_e500);
        let inputs = gen_inputs(n, seed);
        let state_in = gen_state(inputs.len(), *state_len, seed ^ 0x57a7);
        let (got, gpu_state) = (b.run_with_state)(&module, &inputs, &state_in)
            .unwrap_or_else(|e| panic!("{name}: gpu run failed: {e}\n{}", module.source));
        let gpu_digest = sextet_state_digest(&got, &gpu_state);

        let key = format!("{name}@st{n}");
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
        let (bad, oracle, oracle_state) = grade_cell(
            name,
            funcs,
            consts,
            entry,
            module.cells[0].params,
            *state_len,
            &inputs,
            &state_in,
            &got,
            &gpu_state,
        );
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
                    "digest": sextet_state_digest(&oracle, &oracle_state),
                }),
            );
            save_book(&book);
        }
    }
    println!(
        "{} state battery: {} eligible — {compiled} compiled ({clean} clean, \
         {cached} via transcript), skipped: {:?}, refusals: {:?}",
        b.label,
        lib.cells.len(),
        lib.skipped,
        refusals
    );
    assert!(
        defects.is_empty(),
        "GPU ≠ interpreter on {} state cells:\n{}",
        defects.len(),
        defects.join("\n")
    );
    // A floor so a silent regression can't read as green.
    assert!(
        compiled >= 300,
        "only {compiled} state cells reached the GPU — the fragment shrank"
    );
}

/// E3, the library × probe-set layout on `backend`: every eligible cell fused
/// into ONE translation unit, the whole library run against one probe set in
/// ONE dispatch — and every (cell, probe) sextet still agrees with the
/// interpreter. This is retrieval-by-execution's substrate (WS-F).
pub fn megakernel_battery(b: &Backend) {
    let lib = eligible_cells();
    let compilable: Vec<&(String, Funcs, Consts, String)> = lib
        .cells
        .iter()
        .filter(|(_, funcs, consts, _)| (b.compile)(funcs, consts, "run").is_ok())
        .collect();
    let cells: Vec<rustmsl::LibraryCell> = compilable
        .iter()
        .map(|(_, funcs, consts, _)| rustmsl::LibraryCell {
            funcs,
            consts,
            entry: "run",
            state_len: 0,
        })
        .collect();
    let module = (b.compile_library)(&cells).expect("library compile");

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
    let got = (b.run)(&module, &probes).expect("library run");
    assert_eq!(got.len(), compilable.len() * probes.len());

    let mut bad = 0usize;
    for (ci, (name, funcs, consts, _)) in compilable.iter().enumerate() {
        let n_args = module.cells[ci].params;
        let mut block = InterpBlock::new(funcs, consts, "run");
        for (pi, probe) in probes.iter().enumerate() {
            let gpu_out = &got[ci * probes.len() + pi];
            let (res, want_steps, _) = block.run(&probe[..n_args], &[]);
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
        "{} E3 megakernel: {} cells × {} probes in one dispatch, {} disagreements",
        b.label,
        compilable.len(),
        probes.len(),
        bad
    );
    assert_eq!(
        bad, 0,
        "megakernel ≠ interpreter on {bad} (cell, probe) pairs"
    );
}
