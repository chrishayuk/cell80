//! C1 — superoptimization over the library, under a real cost model from the start.
//! Pre-registered in `../../experiments/cell-superoptimization-preregistration.md`
//! before this ran.
//!
//! Same DAG-with-fan-out search C0 (`gpu_fanout_gate.rs`) already built and proved —
//! GA+CEGIS over the library's free-fn vocabulary, GPU-batched full-domain
//! verification — but every target is now *some library cell's own behaviour*
//! (no synthetic canary), and the cost model is fixed to be honest from generation
//! zero: C0's re-run found a construction (`next_pow2`) that looked like a 3.25x win
//! under raw IR steps and inverted to 2.57x worse once repriced for the real Z80
//! substrate's mul/div host trap. Two stages: (1) GPU search fitness repriced by a
//! static, per-vocabulary-cell mean-trapped-ops signal (measured on the real Z80
//! body, not guessed from bytecode `rustmsl` won't expose); (2) every IR-repriced
//! "cheaper" candidate is hand-composed into one source and re-costed on the real
//! Z80 body — only a stage-2-confirmed, P_T=0-robust win counts.
//!
//! Run: `cargo run --release -p cell80 --example gpu_superopt` (macOS)

#[cfg(not(target_os = "macos"))]
fn main() {
    println!(
        "gpu_superopt needs macOS (Metal) — the codegen builds everywhere, the executor doesn't"
    );
}

#[cfg(target_os = "macos")]
fn main() {
    macos::run();
}

#[cfg(target_os = "macos")]
mod macos {
    use cell80::{Cartridge, CartridgeOpts, CellConfig, Halt, Runner};
    use cell80_core::ir::{Expr, Func};
    use rustmsl::interp::{linearize, CellProgram, InterpBatch};
    use std::collections::HashMap;

    const POP: usize = 4096;
    const MAX_GEN: usize = 400;
    const STAGNATION: usize = 40;
    const NO_HIT_STAGNATION: usize = 20;
    const MAX_DEPTH: u32 = 4;
    const MAX_CEX: usize = 128;
    const CANDIDATE: &str = "$candidate";
    /// The 14-constant set `cell-cost-discovery` pre-registered — kept identical.
    const CONSTANTS: [u16; 14] = [
        0, 1, 2, 3, 4, 5, 8, 10, 16, 255, 256, 0x00FF, 0xFF00, 0xFFFF,
    ];
    const DOMAIN: usize = 1 << 16;

    struct Rng(u64);
    impl Rng {
        fn u32(&mut self) -> u32 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            (x >> 32) as u32
        }
        fn below(&mut self, n: usize) -> usize {
            (self.u32() as usize) % n.max(1)
        }
        fn u16(&mut self) -> u16 {
            self.u32() as u16
        }
        fn constant(&mut self) -> u16 {
            CONSTANTS[self.below(CONSTANTS.len())]
        }
    }

    fn lower(src: &str) -> Result<Vec<(String, Func)>, String> {
        let combined = format!("{src}\n{}{}", cell80::CELL_PRELUDE, rustz80::F32_KERNELS);
        let file: syn::File = syn::parse_str(&combined).map_err(|e| format!("{e}"))?;
        let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default())?;
        if !lowered.funcs.iter().any(|(n, _)| n == "run") {
            return Err("no run".into());
        }
        let funcs = cell80_core::inline::inline(lowered.funcs, &["run"]);
        Ok(cell80_core::dce::prune(funcs, &["run"]))
    }

    /// A library cell usable as a tree node: name, arity, and its raw source (kept
    /// around for stage-0 real-Z80 profiling and stage-2 hand-composition — the
    /// rustmsl-lowered `Func` alone can't drive either).
    struct Cell {
        name: String,
        arity: usize,
        src: String,
    }

    fn calls(e: &Expr, name: &str) -> bool {
        match e {
            Expr::Call(n, args) => n == name || args.iter().any(|a| calls(a, name)),
            _ => false,
        }
    }

    fn rand_tree(rng: &mut Rng, depth: u32, cells: &[Cell]) -> Expr {
        if depth == 0 || rng.below(100) < 30 {
            if rng.below(3) == 0 {
                Expr::Lit(rng.constant())
            } else {
                Expr::Var(0)
            }
        } else {
            let c = rng.below(cells.len());
            let args = (0..cells[c].arity)
                .map(|_| rand_tree(rng, depth - 1, cells))
                .collect();
            Expr::Call(cells[c].name.clone(), args)
        }
    }

    fn size(e: &Expr) -> usize {
        match e {
            Expr::Call(_, a) => 1 + a.iter().map(size).sum::<usize>(),
            _ => 1,
        }
    }
    /// Sum of each `Call` node's precomputed mean real-Z80 `trapped_ops`, recursively
    /// — the static structural repricing signal (§2 of the pre-registration): a
    /// dynamically-observed-per-cell, statically-applied-per-candidate proxy.
    fn mulish_score(e: &Expr, trapped: &HashMap<String, f64>) -> f64 {
        match e {
            Expr::Call(name, args) => {
                trapped.get(name).copied().unwrap_or(0.0)
                    + args.iter().map(|a| mulish_score(a, trapped)).sum::<f64>()
            }
            _ => 0.0,
        }
    }
    fn replace_nth(e: &Expr, n: usize, sub: &Expr, c: &mut usize) -> Expr {
        if *c == n {
            *c += 1;
            return sub.clone();
        }
        *c += 1;
        match e {
            Expr::Call(name, args) => Expr::Call(
                name.clone(),
                args.iter().map(|a| replace_nth(a, n, sub, c)).collect(),
            ),
            other => other.clone(),
        }
    }
    fn mutate(e: &Expr, rng: &mut Rng, cells: &[Cell]) -> Expr {
        let pos = rng.below(size(e));
        replace_nth(e, pos, &rand_tree(rng, 3, cells), &mut 0)
    }

    fn cand_func(e: &Expr) -> Func {
        Func {
            params: 1,
            n_locals: 1,
            body: vec![],
            ret: vec![e.clone()],
            wide_param: false,
            wide_second: false,
            wide_ret: false,
        }
    }
    fn cand_prog(all: &mut [(String, Func)], e: &Expr) -> Option<CellProgram> {
        all[0].1 = cand_func(e);
        match linearize(all, CANDIDATE) {
            Ok(p) if p.max_depth <= 32 && p.n_locals <= 64 => Some(p),
            _ => None,
        }
    }
    fn show(e: &Expr) -> String {
        match e {
            Expr::Var(_) => "x".into(),
            Expr::Lit(n) => format!("{n}"),
            Expr::Call(name, a) => format!(
                "{name}({})",
                a.iter().map(show).collect::<Vec<_>>().join(", ")
            ),
            _ => "?".into(),
        }
    }
    // ---- Real-Z80 compile + full/sampled-domain profiling (stage 0 + stage 2). ----

    fn z80_compile(id: &str, src: &str) -> Option<Cartridge> {
        Cartridge::compile(
            src,
            CellConfig::permissive(),
            CartridgeOpts {
                id: Some(id.into()),
                kernel_bank: true,
                ..Default::default()
            },
        )
        .ok()
    }

    /// Mean real Z80 (cycles, trapped_ops) over every response `run_fast` returns —
    /// regardless of halt variant (a partial cell's trap behaviour on the inputs it
    /// *does* return on is still real signal), used identically for unary vocabulary
    /// cells and for target reference costs (a target must be additionally checked
    /// `Halt::Returned`-total separately — this fn alone doesn't gate that).
    fn z80_profile_unary(cart: &Cartridge) -> Option<(f64, f64, Vec<u16>, bool)> {
        let mut r = Runner::new(cart.z80().ok()?);
        let entry = cart.manifest.entry.clone();
        let mut cyc = 0u64;
        let mut trp = 0u64;
        let mut table = vec![0u16; DOMAIN];
        let mut total = true;
        for (v, slot) in table.iter_mut().enumerate() {
            let f = r
                .run_fast(Some(&entry), &[v as u16], cell80::DEFAULT_CYCLES)
                .ok()?;
            cyc += f.cycles;
            trp += f.trapped_ops;
            if matches!(f.halt, Halt::Returned) {
                *slot = f.result;
            } else {
                total = false;
            }
        }
        Some((
            cyc as f64 / DOMAIN as f64,
            trp as f64 / DOMAIN as f64,
            table,
            total,
        ))
    }
    /// Same, for a binary cell, over the u8xu8 grid (cost-discovery's own P-measurement
    /// grid — arity-2 full-domain-on-CPU is exactly what that programme deferred).
    fn z80_profile_binary(cart: &Cartridge) -> Option<(f64, f64)> {
        let mut r = Runner::new(cart.z80().ok()?);
        let entry = cart.manifest.entry.clone();
        let mut cyc = 0u64;
        let mut trp = 0u64;
        let mut n = 0u64;
        for a in 0..=255u16 {
            for b in 0..=255u16 {
                let f = r
                    .run_fast(Some(&entry), &[a, b], cell80::DEFAULT_CYCLES)
                    .ok()?;
                cyc += f.cycles;
                trp += f.trapped_ops;
                n += 1;
            }
        }
        Some((cyc as f64 / n as f64, trp as f64 / n as f64))
    }

    /// `cost-discovery`'s own P-measurement, reproduced fresh: a trap-free
    /// shift-and-add mul16 vs. a plain `a*b` trap cell, over the u8x8 grid — in
    /// T-state units (stage 2's `P_T`) via `Runner`, or IR-step units (stage 1's
    /// `P_IR`) via `InterpBatch`, per `ir_space`.
    const SOFT_MUL: &str = "fn run(a: u16, b: u16) -> u16 { let mut acc = 0u16; let mut x = a; let mut y = b; let mut i = 0u16; while i < 16u16 { if (y & 1u16) != 0u16 { acc = acc.wrapping_add(x); } x = x << 1u16; y = y >> 1u16; i = i + 1u16; } acc }";
    const TRAP_MUL: &str = "fn run(a: u16, b: u16) -> u16 { a * b }";

    fn measure_p_t() -> f64 {
        let soft = z80_compile("xp_soft_mul16", SOFT_MUL).expect("soft mul16 compiles");
        let trap = z80_compile("xp_trap_mul16", TRAP_MUL).expect("trap mul16 compiles");
        let (sm, _) = z80_profile_binary(&soft).expect("soft mul16 profiles");
        let (tm, _) = z80_profile_binary(&trap).expect("trap mul16 profiles");
        (sm - tm).max(0.0)
    }

    fn measure_p_ir(batch: &mut InterpBatch) -> f64 {
        let soft_funcs = lower(SOFT_MUL).expect("soft mul16 lowers");
        let trap_funcs = lower(TRAP_MUL).expect("trap mul16 lowers");
        let soft_prog = linearize(&soft_funcs, "run").expect("soft mul16 linearizes");
        let trap_prog = linearize(&trap_funcs, "run").expect("trap mul16 linearizes");
        batch.reload(&[soft_prog, trap_prog]);
        let grid: Vec<[u16; 3]> = (0..=255u16)
            .flat_map(|a| (0..=255u16).map(move |b| [a, b, 0]))
            .collect();
        let out = batch.run(&grid);
        let n = grid.len();
        let mean = |bi: usize| {
            let mut s = 0u64;
            for k in 0..n {
                let o = out[bi * n + k];
                s += o[4] as u64 | ((o[5] as u64) << 16);
            }
            s as f64 / n as f64
        };
        (mean(0) - mean(1)).max(0.0)
    }

    /// Extract a cell's own parameter names and its body's exact source text (the
    /// outermost brace-delimited block, verbatim — no re-serialization, so no risk
    /// of a pretty-printer subtly changing semantics). Parameter names come from a
    /// real `syn` parse (exact identifier resolution); body text comes from the
    /// original source directly (single-function cell sources have no other braces
    /// at the top level, by the same `funcs.len() == 1` admission criterion the
    /// vocabulary pool itself already requires).
    fn extract_params_and_body(src: &str) -> Option<(Vec<String>, String)> {
        let stripped: String = src
            .lines()
            .filter(|l| !l.trim_start().starts_with("//!"))
            .collect::<Vec<_>>()
            .join("\n");
        let item: syn::ItemFn = syn::parse_str(&stripped).ok()?;
        let params: Vec<String> = item
            .sig
            .inputs
            .iter()
            .filter_map(|arg| match arg {
                syn::FnArg::Typed(pt) => match &*pt.pat {
                    syn::Pat::Ident(id) => Some(id.ident.to_string()),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        let open = stripped.find('{')?;
        let close = stripped.rfind('}')?;
        if close <= open {
            return None;
        }
        Some((params, stripped[open + 1..close].trim().to_string()))
    }

    /// Word-boundary-aware identifier replace (Rust identifier chars: alnum +
    /// underscore) — used to substitute a callee's own parameter names with the
    /// caller-supplied argument text, without risking a false match inside a
    /// longer identifier that merely contains `name` as a substring.
    fn word_replace(text: &str, name: &str, replacement: &str) -> String {
        let is_ident = |c: char| c.is_alphanumeric() || c == '_';
        let chars: Vec<char> = text.chars().collect();
        let needle: Vec<char> = name.chars().collect();
        let mut out = String::with_capacity(text.len());
        let mut i = 0;
        while i < chars.len() {
            if chars[i..].starts_with(needle.as_slice()) {
                let before_ok = i == 0 || !is_ident(chars[i - 1]);
                let after = i + needle.len();
                let after_ok = after >= chars.len() || !is_ident(chars[after]);
                if before_ok && after_ok {
                    out.push_str(replacement);
                    i = after;
                    continue;
                }
            }
            out.push(chars[i]);
            i += 1;
        }
        out
    }

    /// Genuine expression-level inlining — no separate function, no `CALL`/`RET`.
    /// Found necessary live: an earlier cut of this harness composed candidates as
    /// `fn run(x) { is_le(x, 1) }` + a *separately defined* `fn is_le(a,b) {...}`,
    /// which is a real Z80 subroutine call (confirmed directly: ~68 T-states of
    /// pure CALL/RET+arg-marshalling overhead, enough by itself to flip
    /// `is_weekend`'s stage-2 verdict from a confirmed win to a rejection). This
    /// walks the candidate bottom-up, substituting each `Call` with its callee's
    /// own body text (parameters replaced by the — already-recursively-inlined —
    /// argument text via `word_replace`). A leaf argument (`x` or a bare literal)
    /// is spliced directly, matching how a human would hand-write the composition
    /// (`cost-discovery`'s `confirm.rs` style) and, crucially, letting the real
    /// Z80 codegen skip a register load entirely for a constant-bound operand —
    /// exactly the partial-application saving `cost-discovery` itself measured. A
    /// non-leaf argument gets a fresh top-level `let` first (this dialect has no
    /// block-expression-as-value — confirmed directly, `{ let a = ...; ... }` used
    /// as a value is a compile error — so nesting must stay flat, and a `let`
    /// avoids both re-evaluating a complex argument twice and any operator-
    /// precedence risk from splicing raw expression text into an arbitrary
    /// position).
    struct Inliner<'a> {
        sources: &'a HashMap<String, String>,
        counter: usize,
    }
    impl<'a> Inliner<'a> {
        fn fresh(&mut self) -> String {
            self.counter += 1;
            format!("__t{}", self.counter)
        }
        fn inline(&mut self, e: &Expr, prelude: &mut Vec<String>) -> Option<String> {
            match e {
                Expr::Var(_) => Some("x".to_string()),
                Expr::Lit(n) => Some(format!("{n}")),
                Expr::Call(name, args) => {
                    let arg_texts: Vec<String> = args
                        .iter()
                        .map(|a| self.inline(a, prelude))
                        .collect::<Option<Vec<_>>>()?;
                    let (params, body) = extract_params_and_body(self.sources.get(name)?)?;
                    let mut body_sub = body;
                    for (p, a) in params.iter().zip(&arg_texts) {
                        let is_leaf = a == "x" || a.parse::<i64>().is_ok();
                        if is_leaf {
                            body_sub = word_replace(&body_sub, p, a);
                        } else {
                            let fresh = self.fresh();
                            prelude.push(format!("let {fresh}: u16 = {a};"));
                            body_sub = word_replace(&body_sub, p, &fresh);
                        }
                    }
                    Some(body_sub)
                }
                _ => None,
            }
        }
    }

    /// Hand-compose a candidate `Expr` into one flat, real-`rustz80`-compilable,
    /// genuinely inlined source (no separate functions, no call overhead) —
    /// generalizing `cost-discovery`'s `confirm.rs` technique to an arbitrary tree
    /// instead of one hand-picked winner.
    fn compose_source(e: &Expr, sources: &HashMap<String, String>) -> Option<String> {
        let mut inliner = Inliner {
            sources,
            counter: 0,
        };
        let mut prelude = Vec::new();
        let tail = inliner.inline(e, &mut prelude)?;
        Some(format!(
            "fn run(x: u16) -> u16 {{ {} {tail} }}",
            prelude.join(" ")
        ))
    }

    /// One target: a library cell's own name, full-domain table, and both cost
    /// references — IR-step (stage 1's search reference) and real-Z80 (stage 2's
    /// actual gate).
    struct Target {
        name: String,
        table: Vec<u16>,
        ir_mean_cost: f64,
        z80_mean_cycles: f64,
        z80_mean_trapped: f64,
    }

    pub fn run() {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let dir = manifest.join("cells");

        // ---- Stage 0a: vocabulary pool (identical filter to gpu_fanout_gate.rs),
        //      keeping each cell's raw source for stage-0b profiling and stage-2
        //      hand-composition. ----
        let mut cells: Vec<Cell> = Vec::new();
        let mut pool: Vec<(String, Func)> = Vec::new();
        let mut sources: HashMap<String, String> = HashMap::new();
        let mut files: Vec<_> = cell80::discover_cell_files(dir.to_str().unwrap()).unwrap();
        files.sort();
        for path in &files {
            if path.extension().is_none_or(|x| x != "rs") {
                continue;
            }
            let name = path.file_stem().unwrap().to_string_lossy().into_owned();
            let src = std::fs::read_to_string(path).unwrap();
            let Ok(sig) = rustz80::entry_signature(&src, "run") else {
                continue;
            };
            if !sig.state.is_empty() {
                continue;
            }
            let scalar = sig
                .params
                .iter()
                .all(|(_, t)| matches!(t.as_str(), "u8" | "u16" | "i16" | "u32" | "i32" | "bool"));
            let arity = sig.params.len();
            if !scalar || arity == 0 || arity > 2 {
                continue;
            }
            let Ok(funcs) = lower(&src) else { continue };
            if funcs.len() == 1 {
                if let Ok(p) = linearize(&funcs, "run") {
                    if p.n_locals <= 64 {
                        cells.push(Cell {
                            name: name.clone(),
                            arity,
                            src: src.clone(),
                        });
                        pool.push((name.clone(), funcs[0].1.clone()));
                        sources.insert(name, src);
                    }
                }
            }
        }
        let n_un = cells.iter().filter(|c| c.arity == 1).count();
        println!(
            "C1 superoptimization — vocabulary pool: {} cells ({n_un} unary, {} binary)",
            cells.len(),
            cells.len() - n_un
        );

        // Linearize pool ONCE; slot 0 is the candidate.
        let mut all: Vec<(String, Func)> = vec![(CANDIDATE.to_string(), cand_func(&Expr::Var(0)))];
        all.extend(pool.iter().cloned());
        let seed = cand_prog(&mut all, &Expr::Var(0)).expect("seed");
        let (mut batch, _) = InterpBatch::new(&[seed]).expect("metal");

        // ---- Stage 0b: P_IR (GPU, IR-step units) and P_T (CPU, real T-states),
        //      each measured fresh in this harness, per §2 of the pre-registration. ----
        let p_ir = measure_p_ir(&mut batch);
        let p_t = measure_p_t();
        println!("P_IR (IR-step trap surcharge) = {p_ir:.3}");
        println!("P_T  (T-state trap surcharge)  = {p_t:.1}\n");

        // ---- Stage 0c: real-Z80 profile every vocabulary cell once — mean cycles +
        //      mean trapped_ops, the static per-cell repricing signal §2 describes.
        //      Unary cells' full-domain table + totality also comes out of this same
        //      pass (targets are the subset that's total). ----
        let mut trapped_by_cell: HashMap<String, f64> = HashMap::new();
        let mut targets: Vec<Target> = Vec::new();
        let mut n_partial = 0usize;
        for c in &cells {
            let Some(cart) = z80_compile(&c.name, &c.src) else {
                continue;
            };
            if c.arity == 1 {
                let Some((cyc, trp, table, total)) = z80_profile_unary(&cart) else {
                    continue;
                };
                trapped_by_cell.insert(c.name.clone(), trp);
                if total {
                    // IR-step mean cost via the GPU path (needed for the stage-1
                    // "worth confirming" filter) — tabulate this one target alone.
                    let solo =
                        vec![pool[cells.iter().position(|x| x.name == c.name).unwrap()].clone()];
                    if let Ok(prog) = linearize(&solo, &c.name) {
                        batch.reload(std::slice::from_ref(&prog));
                        let full_domain: Vec<[u16; 3]> =
                            (0..=u16::MAX).map(|v| [v, 0, 0]).collect();
                        let out = batch.run(&full_domain);
                        let mut steps = 0u64;
                        for o in out.iter().take(DOMAIN) {
                            steps += o[4] as u64 | ((o[5] as u64) << 16);
                        }
                        targets.push(Target {
                            name: c.name.clone(),
                            table,
                            ir_mean_cost: steps as f64 / DOMAIN as f64,
                            z80_mean_cycles: cyc,
                            z80_mean_trapped: trp,
                        });
                    }
                } else {
                    n_partial += 1;
                }
            } else {
                let Some((_cyc, trp)) = z80_profile_binary(&cart) else {
                    continue;
                };
                trapped_by_cell.insert(c.name.clone(), trp);
            }
        }
        println!(
            "targets: {} total unary cells ({n_partial} unary cells excluded as partial on Z80)\n",
            targets.len()
        );

        let full_domain: Vec<[u16; 3]> = (0..=u16::MAX).map(|v| [v, 0, 0]).collect();
        let nfd = full_domain.len();
        let mut rng = Rng(0x1357_9BDF_2468_ACE0);
        let base_probes: Vec<[u16; 3]> = (0..48).map(|_| [rng.u16(), 0, 0]).collect();
        let n_targets = targets.len();
        let mut confirmed_wins: Vec<(String, f64, f64, f64, String)> = Vec::new(); // name, z80_repriced_target, z80_repriced_cand, p0_ratio_ok, chain

        for (ti, target) in targets.iter().enumerate() {
            print!("[{}/{n_targets}] {} ... ", ti + 1, target.name);
            use std::io::Write;
            std::io::stdout().flush().ok();
            let target_ir_repriced = target.ir_mean_cost; // stage-1 reference is unrepriced IR steps for the TARGET itself (it's the thing being beaten, not a candidate)
            let is_self = |e: &Expr| calls(e, &target.name);

            let mut probes = base_probes.clone();
            let mut pop: Vec<(Expr, Option<CellProgram>)> = Vec::with_capacity(POP);
            for _ in 0..POP {
                let e = rand_tree(&mut rng, MAX_DEPTH, &cells);
                let p = if is_self(&e) {
                    None
                } else {
                    cand_prog(&mut all, &e)
                };
                pop.push((e, p));
            }
            let mut best: Option<(Expr, CellProgram, f64)> = None; // cheapest IR-repriced verified-correct
            let mut cex = 0usize;
            let mut since_improved = 0usize;
            let mut best_fit_seen = i64::MIN;
            let mut since_fit_improved = 0usize;
            let t0 = std::time::Instant::now();
            let mut gens = 0usize;

            for _ in 0..MAX_GEN {
                gens += 1;
                let wants: Vec<u16> = probes.iter().map(|p| target.table[p[0] as usize]).collect();
                let np = probes.len();
                let mut progs = Vec::with_capacity(POP);
                let mut slot = Vec::with_capacity(POP);
                for (_, p) in &pop {
                    match p {
                        Some(cp) => {
                            slot.push(Some(progs.len()));
                            progs.push(cp.clone());
                        }
                        None => slot.push(None),
                    }
                }
                batch.reload(&progs);
                let out = batch.run(&probes);
                let exact = |bi: usize| {
                    (0..np)
                        .filter(|&k| out[bi * np + k][3] == 0 && out[bi * np + k][0] == wants[k])
                        .count()
                };
                // Repriced-cost estimate over the probe set: mean IR steps + P_IR *
                // static mulish_score — used as the fitness tie-break from
                // generation zero (replacing C0's plain `size()` tie-break).
                let repriced_est = |bi: usize, e: &Expr| {
                    let mut s = 0u64;
                    for k in 0..np {
                        let o = out[bi * np + k];
                        s += o[4] as u64 | ((o[5] as u64) << 16);
                    }
                    (s as f64 / np as f64) + p_ir * mulish_score(e, &trapped_by_cell)
                };
                let fit: Vec<i64> = pop
                    .iter()
                    .zip(&slot)
                    .map(|((e, _), s)| match s {
                        Some(bi) => {
                            exact(*bi) as i64 * 1_000_000_000 - repriced_est(*bi, e).round() as i64
                        }
                        None => -1,
                    })
                    .collect();
                const MAX_VERIFY_PER_GEN: usize = 4;
                let mut perfect: Vec<(usize, usize)> = slot
                    .iter()
                    .enumerate()
                    .filter_map(|(i, s)| s.map(|b| (i, b)))
                    .filter(|&(_, bi)| exact(bi) == np)
                    .collect();
                perfect.sort_by_key(|&(idx, bi)| repriced_est(bi, &pop[idx].0).round() as i64);
                perfect.truncate(MAX_VERIFY_PER_GEN);
                let mut improved = false;
                if !perfect.is_empty() {
                    let verify_progs: Vec<CellProgram> = perfect
                        .iter()
                        .map(|&(idx, _)| pop[idx].1.clone().unwrap())
                        .collect();
                    batch.reload(&verify_progs);
                    let full_out = batch.run(&full_domain);
                    for (vi, &(idx, _)) in perfect.iter().enumerate() {
                        let mut mismatch: Option<u16> = None;
                        let mut total_steps = 0u64;
                        for v in 0..nfd {
                            let o = full_out[vi * nfd + v];
                            if o[3] != 0 || o[0] != target.table[v] {
                                mismatch = Some(v as u16);
                                break;
                            }
                            total_steps += o[4] as u64 | ((o[5] as u64) << 16);
                        }
                        match mismatch {
                            Some(cx) if cex < MAX_CEX => {
                                probes.push([cx, 0, 0]);
                                cex += 1;
                            }
                            Some(_) => {}
                            None => {
                                let (e, p) = &pop[idx];
                                let ir_cost = total_steps as f64 / nfd as f64
                                    + p_ir * mulish_score(e, &trapped_by_cell);
                                let sp = p.as_ref().unwrap();
                                let better = best.as_ref().is_none_or(|(_, _, c)| ir_cost < *c);
                                if better {
                                    best = Some((e.clone(), sp.clone(), ir_cost));
                                    improved = true;
                                }
                            }
                        }
                    }
                }
                since_improved = if improved { 0 } else { since_improved + 1 };
                if best.is_some() && since_improved >= STAGNATION {
                    break;
                }
                let gen_best_fit = fit.iter().copied().max().unwrap_or(i64::MIN);
                if gen_best_fit > best_fit_seen {
                    best_fit_seen = gen_best_fit;
                    since_fit_improved = 0;
                } else {
                    since_fit_improved += 1;
                }
                if best.is_none() && since_fit_improved >= NO_HIT_STAGNATION {
                    break;
                }
                let mut order: Vec<usize> = (0..POP).collect();
                order.sort_by(|&a, &b| fit[b].cmp(&fit[a]));
                let en = (POP / 10).max(2);
                let elite: Vec<(Expr, Option<CellProgram>)> =
                    order[..en].iter().map(|&i| pop[i].clone()).collect();
                let mut next = elite.clone();
                while next.len() < POP {
                    let ce = if rng.below(100) < 12 {
                        rand_tree(&mut rng, MAX_DEPTH, &cells)
                    } else {
                        mutate(&elite[rng.below(en)].0, &mut rng, &cells)
                    };
                    let cp = if is_self(&ce) {
                        None
                    } else {
                        cand_prog(&mut all, &ce)
                    };
                    next.push((ce, cp));
                }
                pop = next;
            }

            match best {
                None => println!(
                    "no hit [+{cex} cex, {gens} gens, {:.1}s]",
                    t0.elapsed().as_secs_f64()
                ),
                Some((e, _, ir_cost)) => {
                    if ir_cost >= target_ir_repriced {
                        println!(
                            "found (not IR-cheaper): {} ({ir_cost:.1} vs {target_ir_repriced:.1} IR-repriced) [+{cex} cex, {gens} gens, {:.1}s]",
                            show(&e), t0.elapsed().as_secs_f64()
                        );
                        continue;
                    }
                    // ---- Stage 2: hand-compose, real Z80, P_T-reprice, P_T=0 lane. ----
                    print!(
                        "IR-repriced-cheaper: {} ({ir_cost:.1} vs {target_ir_repriced:.1}) — confirming on real Z80... ",
                        show(&e)
                    );
                    std::io::stdout().flush().ok();
                    let Some(src) = compose_source(&e, &sources) else {
                        println!("compose failed (missing source)");
                        continue;
                    };
                    let Some(cart) = z80_compile("composed", &src) else {
                        println!("real-Z80 compile failed");
                        continue;
                    };
                    let Some((cand_cyc, cand_trp, cand_table, cand_total)) =
                        z80_profile_unary(&cart)
                    else {
                        println!("real-Z80 profile failed");
                        continue;
                    };
                    if !cand_total || cand_table != target.table {
                        println!(
                            "REJECTED: composed source disagrees with target on real Z80 (total={cand_total}) — an IR-step-vs-Z80-semantics divergence, not a hit"
                        );
                        continue;
                    }
                    let cand_repriced = cand_cyc + p_t * cand_trp;
                    let target_repriced = target.z80_mean_cycles + p_t * target.z80_mean_trapped;
                    let cand_p0 = cand_cyc;
                    let target_p0 = target.z80_mean_cycles;
                    let wins_repriced = cand_repriced < target_repriced;
                    let wins_p0 = cand_p0 < target_p0;
                    println!(
                        "\n    Z80 repriced: {cand_repriced:.1} vs {target_repriced:.1} ({:.2}x) {}",
                        target_repriced / cand_repriced,
                        if wins_repriced { "COMPOSED WINS" } else { "reference wins — REJECTED" }
                    );
                    println!(
                        "    Z80 raw (P_T=0): {cand_p0:.1} vs {target_p0:.1} ({:.2}x) {}",
                        target_p0 / cand_p0,
                        if wins_p0 {
                            "survives"
                        } else {
                            "repricing-dependent"
                        }
                    );
                    if wins_repriced && wins_p0 {
                        confirmed_wins.push((
                            target.name.clone(),
                            target_repriced,
                            cand_repriced,
                            target_repriced / cand_repriced,
                            show(&e),
                        ));
                        println!("    ==> CONFIRMED WIN");
                    } else if wins_repriced {
                        println!("    ==> repricing-dependent, NOT counted (P_T=0 sensitivity lane failed it)");
                    } else {
                        println!(
                            "    ==> stage 2 REJECTED (this session's next_pow2 outcome, exactly)"
                        );
                    }
                }
            }
        }

        println!("\n=== C1 gate summary ===");
        println!(
            "stage-2-confirmed, P_T=0-robust wins: {} (bar: >= 5 to materially exceed cost-discovery's 4)",
            confirmed_wins.len()
        );
        println!(
            "{}",
            if confirmed_wins.len() >= 5 {
                "sweep gate: PASS"
            } else {
                "sweep gate: FAIL (or below the pre-registered bar)"
            }
        );
        for (name, base, cost, ratio, chain) in &confirmed_wins {
            println!("  {name:<24} {chain}  {base:.1} -> {cost:.1} ({ratio:.2}x)");
        }
    }
}
