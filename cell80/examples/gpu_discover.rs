//! Discover a cell for an INDEPENDENTLY-specified function.
//!
//! Unlike gpu_grow_wave (targets defined AS compositions, so evolution
//! rediscovers them), here each target is a reference function specified by its
//! behaviour (a Rust closure). Evolution must DISCOVER a composition of library
//! cells — a tree over unary and binary cells — whose full-domain behaviour
//! matches. The whole population is scored on the GPU in one InterpBatch dispatch
//! per generation. Solved targets are full-domain-verified vs the reference and
//! novelty-checked (would they earn admission?).
//!
//! Run: `cargo run --release -p cell80 --example gpu_discover` (macOS)

#[cfg(not(target_os = "macos"))]
fn main() {
    println!(
        "gpu_discover needs macOS (Metal) — the codegen builds everywhere, the executor doesn't"
    );
}

#[cfg(target_os = "macos")]
fn main() {
    macos::run();
}

#[cfg(target_os = "macos")]
mod macos {
    use cell80_core::ir::{Expr, Func};
    use rustmsl::interp::{cpu_run, linearize, CellProgram, VmOut};

    const POP: usize = 4096;
    const MAX_GEN: usize = 400; // CEGIS spends generations per counterexample round
    const MAX_DEPTH: u32 = 5;
    const CANDIDATE: &str = "$candidate";

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

    /// A library cell usable as a tree node: name, arity (1 or 2), and its func.
    struct Cell {
        name: String,
        arity: usize,
    }

    fn rand_tree(rng: &mut Rng, depth: u32, cells: &[Cell]) -> Expr {
        if depth == 0 || rng.below(100) < 30 {
            if rng.below(3) == 0 {
                Expr::Lit(rng.u16())
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
    /// library pool built ONCE). Swapping only slot 0 avoids cloning all 168 pool
    /// funcs per candidate — the search loop's real cost.
    fn cand_prog(all: &mut [(String, Func)], e: &Expr) -> Option<CellProgram> {
        all[0].1 = cand_func(e);
        match linearize(all, CANDIDATE) {
            Ok(p) if p.max_depth <= 32 && p.n_locals <= 64 => Some(p),
            _ => None,
        }
    }
    fn eval(p: &CellProgram, x: u16) -> Option<u16> {
        match cpu_run(p, &[x]) {
            VmOut::Value(v, _) => v.first().copied(),
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

    /// Digital root: repeatedly sum decimal digits to a single digit.
    fn digital_root(x: u16) -> u16 {
        let mut n = x as u32;
        while n >= 10 {
            let mut s = 0;
            while n > 0 {
                s += n % 10;
                n /= 10;
            }
            n = s;
        }
        n as u16
    }

    pub fn run() {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let dir = manifest.join("cells");
        let fp_probes: Vec<u16> = {
            let mut r = Rng(0xC0FFEE);
            (0..32).map(|_| r.u16()).collect()
        };

        // Building blocks: unary + binary single-func value cells; a fingerprint per
        // unary cell (for the novelty check).
        let mut cells: Vec<Cell> = Vec::new();
        let mut pool: Vec<(String, Func)> = Vec::new();
        let mut lib_fp: Vec<(String, Vec<Option<u16>>)> = Vec::new();
        let mut files: Vec<_> = cell80::discover_cell_files(dir.to_str().unwrap()).unwrap();
        files.sort();
        for path in files {
            if path.extension().is_none_or(|x| x != "rs") {
                continue;
            }
            let name = path.file_stem().unwrap().to_string_lossy().into_owned();
            let src = std::fs::read_to_string(&path).unwrap();
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
                        if arity == 1 {
                            lib_fp.push((
                                name.clone(),
                                fp_probes.iter().map(|&x| eval(&p, x)).collect(),
                            ));
                        }
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
        println!("GPU cell discovery (independent targets)\n");
        println!(
            "building blocks: {} cells ({} unary, {} binary)\n",
            cells.len(),
            n_un,
            cells.len() - n_un
        );

        #[allow(clippy::type_complexity)]
        let targets: Vec<(&str, fn(u16) -> u16)> = vec![
            ("digital_root", digital_root),
            ("hi_byte_popcount", |x| (x >> 8).count_ones() as u16),
            ("lo_byte_popcount", |x| (x & 0xFF).count_ones() as u16),
            ("byte_sum", |x| (x & 0xFF).wrapping_add(x >> 8)),
            ("byte_xor", |x| (x & 0xFF) ^ (x >> 8)),
            ("byte_max", |x| (x & 0xFF).max(x >> 8)),
            ("parity", |x| (x.count_ones() & 1) as u16),
        ];

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (pool, MAX_GEN, targets, lib_fp);
            println!("(no Metal — skipped)");
        }

        #[cfg(target_os = "macos")]
        {
            use rustmsl::interp::InterpBatch;
            use std::time::Instant;
            let mut rng = Rng(0x1357_9BDF_2468_ACE0);
            let base_probes: Vec<[u16; 3]> = (0..48).map(|_| [rng.u16(), 0, 0]).collect();
            // Build the linearize pool ONCE — slot 0 is the candidate, the rest the
            // library. cand_prog swaps only slot 0 (no per-candidate pool clone).
            let mut all: Vec<(String, Func)> =
                vec![(CANDIDATE.to_string(), cand_func(&Expr::Var(0)))];
            all.extend(pool.iter().cloned());
            let seed = cand_prog(&mut all, &Expr::Var(0)).expect("seed");
            let (mut batch, _) = InterpBatch::new(&[seed]).expect("metal");

            let agree = |a: &[Option<u16>], b: &[Option<u16>]| {
                let n = a.len().min(b.len());
                a.iter().zip(b).filter(|(x, y)| x == y).count() as f32 / n.max(1) as f32
            };
            // First input where the candidate disagrees with the target. A cheap
            // strided sample (~2100 inputs) rejects most fakes fast; only when the
            // sample is clean do we scan the full domain — so an accepted solve is
            // still exhaustively verified, but hard targets don't thrash the CPU.
            let counterexample = |sp: &CellProgram, tf: fn(u16) -> u16| -> Option<u16> {
                if let Some(x) = (0..=u16::MAX)
                    .step_by(31)
                    .find(|&x| eval(sp, x) != Some(tf(x)))
                {
                    return Some(x);
                }
                (0..=u16::MAX).find(|&x| eval(sp, x) != Some(tf(x)))
            };

            for (tname, tf) in &targets {
                // CEGIS: grow the probe set with counterexamples until a candidate
                // that matches every probe is ALSO full-domain-correct — by then,
                // "matches the probes" == "is the function".
                let mut probes = base_probes.clone();
                // Linearize each candidate ONCE, at birth; survivors carry their
                // compiled bytecode forward instead of recompiling every generation.
                let mut pop: Vec<(Expr, Option<CellProgram>)> = Vec::with_capacity(POP);
                for _ in 0..POP {
                    let e = rand_tree(&mut rng, MAX_DEPTH, &cells);
                    let p = cand_prog(&mut all, &e);
                    pop.push((e, p));
                }
                let mut solved: Option<(Expr, CellProgram)> = None;
                let mut cex = 0usize;
                let t0 = Instant::now();
                let mut gens = 0usize;

                for _ in 0..MAX_GEN {
                    gens += 1;
                    let wants: Vec<u16> = probes.iter().map(|p| tf(p[0])).collect();
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
                            .filter(|&k| {
                                out[bi * np + k][3] == 0 && out[bi * np + k][0] == wants[k]
                            })
                            .count()
                    };
                    let fit: Vec<usize> = slot
                        .iter()
                        .map(|s| match s {
                            Some(bi) => (0..np)
                                .map(|k| {
                                    let o = out[bi * np + k];
                                    if o[3] == 0 {
                                        16 - (o[0] ^ wants[k]).count_ones() as usize
                                    } else {
                                        0
                                    }
                                })
                                .sum(),
                            None => 0,
                        })
                        .collect();
                    // A candidate matching every probe: verify full-domain; accept, or
                    // add the counterexample input and let evolution continue.
                    if let Some(&bi) = slot.iter().flatten().find(|&&bi| exact(bi) == np) {
                        let idx = slot.iter().position(|s| *s == Some(bi)).unwrap();
                        let sp = pop[idx].1.as_ref().unwrap(); // cached prog — no re-linearize
                        match counterexample(sp, *tf) {
                            None => {
                                solved = Some((pop[idx].0.clone(), sp.clone()));
                                break;
                            }
                            Some(cx) if cex < 128 => {
                                probes.push([cx, 0, 0]);
                                cex += 1;
                            }
                            Some(_) => break, // give up if the probe set balloons
                        }
                    }
                    let mut order: Vec<usize> = (0..POP).collect();
                    order.sort_by(|&a, &b| {
                        fit[b]
                            .cmp(&fit[a])
                            .then(size(&pop[a].0).cmp(&size(&pop[b].0)))
                    });
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
                        let cp = cand_prog(&mut all, &ce); // linearize once, at birth
                        next.push((ce, cp));
                    }
                    pop = next;
                }

                match solved {
                    // A solve is full-domain-correct BY CONSTRUCTION (CEGIS) — only
                    // novelty remains to decide admission.
                    Some((e, sp)) => {
                        let sfp: Vec<Option<u16>> =
                            fp_probes.iter().map(|&x| eval(&sp, x)).collect();
                        let (bid, ba) = lib_fp
                            .iter()
                            .map(|(id, f)| (id.clone(), agree(&sfp, f)))
                            .max_by(|a, b| a.1.total_cmp(&b.1))
                            .unwrap();
                        let verdict = if ba >= 0.834 {
                            format!("EXISTS as `{bid}` (agreement {ba:.3}) — dedup would reject")
                        } else {
                            format!("DISCOVERED — full-domain-correct, novel ({ba:.3} vs {bid}) — would admit")
                        };
                        println!("  {tname:<18} = {}", show(&e));
                        println!(
                            "  {:<18}   {verdict}  [+{cex} cex, {gens} gens, {:.1}s]",
                            "",
                            t0.elapsed().as_secs_f64()
                        );
                    }
                    None => println!(
                        "  {tname:<18} : not discovered ({cex} cex, {gens} gens, {:.1}s)",
                        t0.elapsed().as_secs_f64()
                    ),
                }
            }
            println!(
            "\nCEGIS: the probe set grows with counterexamples until matching every probe means"
        );
            println!(
            "being the function — so every solve is full-domain-correct by construction. The dedup"
        );
            println!("gate then decides novelty. Probe-only fakes can no longer slip through.");
        }
    }
}
