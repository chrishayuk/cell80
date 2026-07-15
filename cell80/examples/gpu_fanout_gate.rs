//! C0 — the fan-out gate. Pre-registered in
//! `../../experiments/cell-fanout-gate-preregistration.md` before this ran.
//!
//! `cell-cost-discovery` searched pure pipelines (`g(f(x))`) and found 4 proved-cheaper
//! rewrites. A pipeline cannot express `x*3 = (x<<1)+x` — the input is needed twice.
//! This extends the grammar by exactly one point: `Expr` trees (already built for
//! `gpu_discover.rs`) where `Var(0)` may appear more than once, combined via genuine
//! two-argument library cells. Two gates, both pre-registered: (1) a synthetic x*3
//! canary — does the search find *any* fan-out construction for the one motivating
//! example; (2) a sweep over every total unary library cell — does fan-out find
//! full-domain-identical, strictly-cheaper (mean IR steps) constructions, and does the
//! count materially exceed cost-discovery's 4.
//!
//! Run: `cargo run --release -p cell80 --example gpu_fanout_gate` (macOS)

#[cfg(not(target_os = "macos"))]
fn main() {
    println!(
        "gpu_fanout_gate needs macOS (Metal) — the codegen builds everywhere, the executor doesn't"
    );
}

#[cfg(target_os = "macos")]
fn main() {
    macos::run();
}

#[cfg(target_os = "macos")]
mod macos {
    use cell80_core::ir::{Expr, Func};
    use rustmsl::interp::{linearize, CellProgram, InterpBatch};

    const POP: usize = 4096;
    const MAX_GEN: usize = 400;
    /// Early-stop: no new full-domain-correct-and-cheaper find for this many
    /// generations after at least one hit → stop (still within the "up to 400"
    /// pre-registered budget, just not spending all of it on a plateau).
    const STAGNATION: usize = 40;
    /// Pre-hit early-stop, tightened after the first live run: with population 4,096
    /// and elitism, 4,096 candidates producing zero improvement for this many
    /// generations is already a strong plateau signal — a laxer bound (originally
    /// `STAGNATION * 2` = 80) let targets that could get *close* but never exact crawl
    /// toward the full 400-generation cap on marginal fitness ticks, with no visibility
    /// into progress. Tightened for tractability across ~75 targets; disclosed here,
    /// not silently changed — it only affects wall-clock, never what counts as a hit.
    const NO_HIT_STAGNATION: usize = 20;
    const MAX_DEPTH: u32 = 4;
    const MAX_CEX: usize = 128;
    const CANDIDATE: &str = "$candidate";
    /// The 14-constant set `cell-cost-discovery` pre-registered — kept identical, not
    /// re-chosen, for comparability.
    const CONSTANTS: [u16; 14] = [
        0, 1, 2, 3, 4, 5, 8, 10, 16, 255, 256, 0x00FF, 0xFF00, 0xFFFF,
    ];

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

    /// A library cell usable as a tree node: name and arity (1 or 2).
    struct Cell {
        name: String,
        arity: usize,
    }

    /// Does `e` call `name` anywhere — the Goodhart guard `cell-cost-discovery-findings`
    /// recorded (`is_carmichael <- ... |> is_carmichael`) but did not implement: a
    /// candidate is disqualified before it is ever scored if it calls its own target.
    fn calls(e: &Expr, name: &str) -> bool {
        match e {
            Expr::Call(n, args) => n == name || args.iter().any(|a| calls(a, name)),
            _ => false,
        }
    }

    /// Number of `Var(0)` leaves — the fan-out witness: a construction that never
    /// reuses the input is a pipeline in disguise, not a fan-out discovery.
    fn var_uses(e: &Expr) -> usize {
        match e {
            Expr::Var(_) => 1,
            Expr::Call(_, args) => args.iter().map(var_uses).sum(),
            _ => 0,
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
    /// Linearize a candidate against `all` (slot 0 is the candidate, the rest is the
    /// library pool built ONCE) — mirrors `gpu_discover.rs`'s `cand_prog`.
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

    /// One target: a name, its full-domain table, and its own mean IR-step cost.
    /// `closure` targets (the x*3 canary) have no library cell to exclude via `calls`;
    /// `library` targets do, and self-composition is disqualified before scoring.
    enum TargetKind {
        Closure,
        Library,
    }
    struct Target {
        name: String,
        table: Vec<u16>,
        mean_cost: f64,
        kind: TargetKind,
    }

    pub fn run() {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let dir = manifest.join("cells");

        // ---- Build the vocabulary pool once (identical filter to gpu_discover.rs). ----
        let mut cells: Vec<Cell> = Vec::new();
        let mut pool: Vec<(String, Func)> = Vec::new();
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
                        });
                        pool.push((name.clone(), funcs[0].1.clone()));
                    }
                }
            }
        }
        let n_un = cells.iter().filter(|c| c.arity == 1).count();
        println!(
            "C0 fan-out gate — vocabulary pool: {} cells ({n_un} unary, {} binary)\n",
            cells.len(),
            cells.len() - n_un
        );

        // Linearize pool ONCE; slot 0 is the candidate (mirrors gpu_discover.rs).
        let mut all: Vec<(String, Func)> = vec![(CANDIDATE.to_string(), cand_func(&Expr::Var(0)))];
        all.extend(pool.iter().cloned());
        let seed = cand_prog(&mut all, &Expr::Var(0)).expect("seed");
        let (mut batch, _) = InterpBatch::new(&[seed]).expect("metal");

        // The full 65,536-input domain, built once — used both for target
        // tabulation and for full-domain candidate verification below. Safe to
        // dispatch via `InterpBatch::run` now that it chunks to the pipeline's
        // `max_tpg`: an earlier cut of this harness dispatched the full domain in
        // one un-chunked call, which silently zeroed every probe beyond `max_tpg`
        // (found live — see `rustmsl::interp::gpu::InterpBatch::run`'s doc comment
        // and `experiments/cell-fanout-gate-preregistration.md`'s InterpBatch
        // amendment). Fixed upstream in `rustmsl`, not worked around here.
        let full_domain: Vec<[u16; 3]> = (0..=u16::MAX).map(|v| [v, 0, 0]).collect();
        let nfd = full_domain.len();

        // ---- Targets: the x*3 canary + every total unary cell in the pool, tabulated
        //      over the full domain in ONE GPU dispatch (not 85 serial CPU sweeps —
        //      that's what made the first cut of this harness stall for minutes
        //      before the search even started). ----
        let mut targets: Vec<Target> = vec![{
            let mut table = vec![0u16; 1 << 16];
            for v in 0..=u16::MAX {
                table[v as usize] = v.wrapping_mul(3);
            }
            Target {
                name: "x*3 (canary)".into(),
                table,
                mean_cost: f64::INFINITY, // no library cost to beat; existence-only gate
                kind: TargetKind::Closure,
            }
        }];
        let mut n_partial = 0usize;
        {
            let mut unary_progs: Vec<CellProgram> = Vec::new();
            let mut unary_names: Vec<String> = Vec::new();
            for (i, c) in cells.iter().enumerate() {
                if c.arity != 1 {
                    continue;
                }
                let solo = vec![pool[i].clone()];
                if let Ok(prog) = linearize(&solo, &c.name) {
                    unary_progs.push(prog);
                    unary_names.push(c.name.clone());
                }
            }
            batch.reload(&unary_progs);
            let out = batch.run(&full_domain);
            for (bi, name) in unary_names.into_iter().enumerate() {
                let mut table = vec![0u16; 1 << 16];
                let mut total_steps = 0u64;
                let mut total = true;
                for k in 0..nfd {
                    let o = out[bi * nfd + k];
                    if o[3] != 0 {
                        total = false;
                        break;
                    }
                    table[k] = o[0];
                    total_steps += o[4] as u64 | ((o[5] as u64) << 16);
                }
                if total {
                    targets.push(Target {
                        name,
                        table,
                        mean_cost: total_steps as f64 / nfd as f64,
                        kind: TargetKind::Library,
                    });
                } else {
                    n_partial += 1;
                }
            }
        }
        println!(
            "targets: {} total unary cells + 1 canary ({n_partial} unary cells excluded as partial)\n",
            targets.len() - 1
        );

        let mut rng = Rng(0x1357_9BDF_2468_ACE0);
        let base_probes: Vec<[u16; 3]> = (0..48).map(|_| [rng.u16(), 0, 0]).collect();

        let mut sweep_hits: Vec<(String, f64, f64, String)> = Vec::new();
        let mut canary_pass = false;
        let n_targets = targets.len();

        for (ti, target) in targets.iter().enumerate() {
            print!("[{}/{n_targets}] {} ... ", ti + 1, target.name);
            use std::io::Write;
            std::io::stdout().flush().ok();
            let is_self =
                |e: &Expr| matches!(target.kind, TargetKind::Library) && calls(e, &target.name);
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
            let mut best: Option<(Expr, CellProgram, f64)> = None; // cheapest verified-correct so far
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
                // ---- GPU batch dispatch: the whole population x the probe set, one
                //      call (the reason gpu_discover.rs is fast — a CPU loop calling
                //      cpu_run per (candidate, probe) does the same work ~POP times
                //      slower). ----
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
                let fit: Vec<i64> = pop
                    .iter()
                    .zip(&slot)
                    .map(|((e, _), s)| match s {
                        Some(bi) => exact(*bi) as i64 * 1_000_000 - size(e) as i64,
                        None => -1,
                    })
                    .collect();
                // Probe-perfect candidates this generation: verify full-domain on the
                // GPU (InterpBatch, chunked — see the full_domain comment above), not a
                // CPU cpu_run scan. With a small probe set, many population members can
                // spuriously agree on all of it at once; checking *every* one via CPU
                // (an earlier cut of this harness did) made a single generation's cost
                // unbounded — one target stalled for 30+ CPU-minutes this way. Still
                // capped to a handful per generation (a batch reload + dispatch per
                // verification round, not per candidate, so the cap now bounds GPU
                // round-trips rather than CPU cycles) — take the smallest-tree
                // candidates first (cheapest-looking, likeliest to survive); still
                // cost-aware (we don't stop at the first solve across the run).
                const MAX_VERIFY_PER_GEN: usize = 4;
                let mut perfect: Vec<(usize, usize)> = slot
                    .iter()
                    .enumerate()
                    .filter_map(|(i, s)| s.map(|b| (i, b)))
                    .filter(|&(_, bi)| exact(bi) == np)
                    .collect();
                perfect.sort_by_key(|&(idx, _)| size(&pop[idx].0));
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
                                // Full-domain-correct. Cost it.
                                let mean_cost = total_steps as f64 / nfd as f64;
                                let (e, p) = &pop[idx];
                                let sp = p.as_ref().unwrap();
                                let better = best.as_ref().is_none_or(|(_, _, c)| mean_cost < *c);
                                if better {
                                    best = Some((e.clone(), sp.clone(), mean_cost));
                                    improved = true;
                                }
                            }
                        }
                    }
                }
                if matches!(target.kind, TargetKind::Closure) && best.is_some() {
                    break; // canary only needs existence, not a cost race
                }
                since_improved = if improved { 0 } else { since_improved + 1 };
                if best.is_some() && since_improved >= STAGNATION {
                    break; // plateaued after a hit — stop within the "up to MAX_GEN" budget
                }
                // No hit yet: also bail once the population's best raw fitness (probe
                // agreement, pre-full-domain) has plateaued — a target this GA is never
                // going to reach shouldn't burn the full 400-generation budget serially
                // across ~75 targets. This bounds wall-clock; it never changes what
                // counts as a hit (the equivalence/cost bars above are untouched).
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
                if gens % 25 == 0 {
                    print!(
                        "    ... gen {gens}/{MAX_GEN} (best_fit {gen_best_fit}, no-improve {since_fit_improved}/{NO_HIT_STAGNATION}"
                    );
                    if let Some((_, _, cost)) = &best {
                        print!(", hit cost {cost:.1}, no-cheaper {since_improved}/{STAGNATION}");
                    }
                    println!(")");
                    use std::io::Write;
                    std::io::stdout().flush().ok();
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

            match (&target.kind, best) {
                (TargetKind::Closure, Some((e, _, _))) => {
                    let uses = var_uses(&e);
                    canary_pass = uses >= 2;
                    println!(
                        "x*3 canary: FOUND {} (Var(0) used {uses}x{}) [+{cex} cex, {gens} gens, {:.1}s]",
                        show(&e),
                        if canary_pass { ", fan-out confirmed" } else { ", NOT fan-out — pipeline in disguise" },
                        t0.elapsed().as_secs_f64()
                    );
                }
                (TargetKind::Closure, None) => {
                    println!(
                        "x*3 canary: NOT FOUND ({cex} cex, {gens} gens, {:.1}s)",
                        t0.elapsed().as_secs_f64()
                    );
                }
                (TargetKind::Library, Some((e, _, cost))) => {
                    if cost < target.mean_cost {
                        sweep_hits.push((target.name.clone(), cost, target.mean_cost, show(&e)));
                        println!(
                            "HIT  {:<24} <- {}  ({:.1} vs {:.1} IR steps, {:.2}x)  [+{cex} cex, {gens} gens, {:.1}s]",
                            target.name, show(&e), cost, target.mean_cost, target.mean_cost / cost,
                            t0.elapsed().as_secs_f64()
                        );
                    } else {
                        println!(
                            "found (not cheaper): {} ({:.1} vs {:.1} IR steps) [+{cex} cex, {gens} gens, {:.1}s]",
                            show(&e), cost, target.mean_cost, t0.elapsed().as_secs_f64()
                        );
                    }
                }
                (TargetKind::Library, None) => {
                    println!(
                        "no hit [+{cex} cex, {gens} gens, {:.1}s]",
                        t0.elapsed().as_secs_f64()
                    );
                }
            }
        }

        println!("\n=== C0 gate summary ===");
        println!(
            "canary gate: {}",
            if canary_pass {
                "PASS — fan-out construction found for x*3"
            } else {
                "FAIL — no fan-out construction found for x*3 within budget"
            }
        );
        println!(
            "sweep gate: {} strictly-cheaper full-domain hits (bar: >= 6 to materially exceed cost-discovery's 4)",
            sweep_hits.len()
        );
        if sweep_hits.len() >= 6 {
            println!("sweep gate: PASS");
        } else {
            println!("sweep gate: FAIL (or below the pre-registered bar)");
        }
        for (name, cost, base, chain) in &sweep_hits {
            println!("  {name:<24} {chain}  {base:.1} -> {cost:.1}");
        }
    }
}
