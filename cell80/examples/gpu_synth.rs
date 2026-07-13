//! GPU genetic program synthesis — the scale where the interpreter backend wins.
//!
//! Evolve a population of candidate expression-programs and score the WHOLE
//! population against a target function's I/O in ONE `InterpBatch` dispatch per
//! generation (kernel compiled once, bytecode buffer reloaded each gen). This is
//! synthesis by execution at GPU fitness-evaluation rates — a big population is a
//! big grid, exactly where the flat/no-cliff substrate pays off, and it's the
//! "cells as a grown population under behavioural selection" thesis made real.
//!
//! Run: `cargo run --release -p cell80 --example gpu_synth` (macOS)

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("gpu_synth needs macOS (Metal) — the codegen builds everywhere, the executor doesn't");
}

#[cfg(target_os = "macos")]
fn main() {
    macos::run();
}

#[cfg(target_os = "macos")]
mod macos {
    use cell80_core::ir::{BinOp, Expr, Func, Width};
    use rustmsl::interp::{cpu_run, linearize, CellProgram, VmOut};
    use std::time::Instant;

    const ARITY: usize = 2;
    const POP: usize = 4096;
    const MAX_GEN: usize = 200;
    const OPS: &[BinOp] = &[
        BinOp::Add,
        BinOp::Sub,
        BinOp::Mul,
        BinOp::And,
        BinOp::Or,
        BinOp::Xor,
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
        fn below(&mut self, n: u32) -> u32 {
            self.u32() % n.max(1)
        }
        fn u16(&mut self) -> u16 {
            self.u32() as u16
        }
    }

    /// A random expression over Var(0..ARITY), Lit, and the op set (shifts take a
    /// literal amount). Depth-bounded so the linearized stack stays within cap.
    fn rand_expr(rng: &mut Rng, depth: u32) -> Expr {
        if depth == 0 || rng.below(100) < 35 {
            if rng.below(2) == 0 {
                Expr::Var(rng.below(ARITY as u32) as usize)
            } else {
                Expr::Lit(rng.u16())
            }
        } else if rng.below(100) < 18 {
            let left = rng.below(2) == 0;
            let k = rng.below(16) as u16;
            Expr::Bin(
                if left { BinOp::Shl } else { BinOp::Shr },
                Box::new(rand_expr(rng, depth - 1)),
                Box::new(Expr::Lit(k)),
                Width::Word,
            )
        } else {
            let op = OPS[rng.below(OPS.len() as u32) as usize];
            Expr::Bin(
                op,
                Box::new(rand_expr(rng, depth - 1)),
                Box::new(rand_expr(rng, depth - 1)),
                Width::Word,
            )
        }
    }

    fn func_of(e: &Expr) -> Vec<(String, Func)> {
        vec![(
            "run".to_string(),
            Func {
                params: ARITY,
                n_locals: ARITY,
                body: vec![],
                ret: vec![e.clone()],
                wide_param: false,
                wide_second: false,
                wide_ret: false,
            },
        )]
    }

    fn size(e: &Expr) -> usize {
        match e {
            Expr::Bin(_, l, r, _) => 1 + size(l) + size(r),
            _ => 1,
        }
    }

    /// The subtree at preorder index `n` (clamped).
    fn nth(e: &Expr, n: usize) -> Expr {
        fn go(e: &Expr, n: usize, c: &mut usize) -> Option<Expr> {
            if *c == n {
                return Some(e.clone());
            }
            *c += 1;
            if let Expr::Bin(_, l, r, _) = e {
                if let Some(x) = go(l, n, c) {
                    return Some(x);
                }
                if let Some(x) = go(r, n, c) {
                    return Some(x);
                }
            }
            None
        }
        let mut c = 0;
        go(e, n, &mut c).unwrap_or_else(|| e.clone())
    }

    /// Replace the subtree at preorder index `n` with `sub`.
    fn replace_nth(e: &Expr, n: usize, sub: &Expr, c: &mut usize) -> Expr {
        if *c == n {
            *c += 1;
            return sub.clone();
        }
        *c += 1;
        match e {
            Expr::Bin(op, l, r, w) => {
                let nl = replace_nth(l, n, sub, c);
                let nr = replace_nth(r, n, sub, c);
                Expr::Bin(*op, Box::new(nl), Box::new(nr), *w)
            }
            other => other.clone(),
        }
    }

    fn mutate(e: &Expr, rng: &mut Rng) -> Expr {
        let pos = rng.below(size(e) as u32) as usize;
        let fresh = rand_expr(rng, 3);
        replace_nth(e, pos, &fresh, &mut 0)
    }

    fn crossover(a: &Expr, b: &Expr, rng: &mut Rng) -> Expr {
        let sub = nth(b, rng.below(size(b) as u32) as usize);
        replace_nth(a, rng.below(size(a) as u32) as usize, &sub, &mut 0)
    }

    fn show(e: &Expr) -> String {
        match e {
            Expr::Var(i) => format!("{}", (b'x' + *i as u8) as char),
            Expr::Lit(n) => format!("{n}"),
            Expr::Bin(op, l, r, _) => {
                let s = match op {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::And => "&",
                    BinOp::Or => "|",
                    BinOp::Xor => "^",
                    BinOp::Shl => "<<",
                    BinOp::Shr => ">>",
                    _ => "?",
                };
                format!("({} {s} {})", show(l), show(r))
            }
            _ => "?".into(),
        }
    }

    fn eval(prog: &CellProgram, args: &[u16]) -> Option<u16> {
        match cpu_run(prog, args) {
            VmOut::Value(v, _) => v.first().copied(),
            _ => None,
        }
    }

    pub fn run() {
        // Target to rediscover: a byte-merge, hi(x) | lo(y) = (x & 0xFF00)|(y & 0x00FF).
        // A bitwise target is where the bit-gradient fitness is honest (each output
        // bit tracks one input bit) — arithmetic synthesis is search-bound, not
        // eval-bound, which is why cells compose existing cells, not free expressions.
        let target = Expr::Bin(
            BinOp::Or,
            Box::new(Expr::Bin(
                BinOp::And,
                Box::new(Expr::Var(0)),
                Box::new(Expr::Lit(0xFF00)),
                Width::Word,
            )),
            Box::new(Expr::Bin(
                BinOp::And,
                Box::new(Expr::Var(1)),
                Box::new(Expr::Lit(0x00FF)),
                Width::Word,
            )),
            Width::Word,
        );
        let target_prog = linearize(&func_of(&target), "run").unwrap();

        // I/O examples: the target on a fixed probe bank.
        let mut rng = Rng(0x1234_5678_9abc_def0);
        // More probes constrain the function better — fewer probe-equivalent-but-wrong
        // solutions survive (the synthesis generalization gap).
        let probes: Vec<[u16; 3]> = (0..64).map(|_| [rng.u16(), rng.u16(), 0]).collect();
        let wants: Vec<u16> = probes
            .iter()
            .map(|p| eval(&target_prog, &p[..ARITY]).unwrap())
            .collect();
        let np = probes.len();

        println!("GPU genetic program synthesis");
        println!("target:   f(x,y) = (x & 0xFF00) | (y & 0x00FF)   [{np} I/O examples]");
        println!("search:   population {POP}, scored on GPU in one dispatch/generation\n");

        #[cfg(not(target_os = "macos"))]
        {
            println!("(no Metal — GPU synthesis skipped)");
            let _ = (wants, np);
            return;
        }

        #[cfg(target_os = "macos")]
        {
            use rustmsl::interp::InterpBatch;
            let mut pop: Vec<Expr> = (0..POP).map(|_| rand_expr(&mut rng, 5)).collect();
            let (mut batch, _) =
                InterpBatch::new(&[linearize(&func_of(&pop[0]), "run").unwrap()]).expect("metal");

            let mut solution: Option<Expr> = None;
            let mut gen_found = 0;
            let mut total_dispatch = 0.0f64;
            let mut total_evals = 0usize;
            let mut best_curve: Vec<usize> = Vec::new();

            for gen in 0..MAX_GEN {
                // Linearize the population; invalid (too-deep) candidates score 0.
                let mut progs: Vec<CellProgram> = Vec::new();
                let mut slot: Vec<Option<usize>> = Vec::with_capacity(POP);
                for e in &pop {
                    match linearize(&func_of(e), "run") {
                        Ok(p) if p.max_depth <= 32 && p.n_locals <= 64 => {
                            slot.push(Some(progs.len()));
                            progs.push(p);
                        }
                        _ => slot.push(None),
                    }
                }
                batch.reload(&progs);
                let t = Instant::now();
                let out = batch.run(&probes); // whole population × examples, ONE dispatch
                total_dispatch += t.elapsed().as_secs_f64();
                total_evals += progs.len() * np;

                // Selection fitness = correct output BITS summed over probes — a
                // gradient (exact-value match gives evolution nothing to climb).
                // Solution test stays exact: all probes matched to the value.
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
                let exact = |bi: usize| -> usize {
                    (0..np)
                        .filter(|&k| out[bi * np + k][3] == 0 && out[bi * np + k][0] == wants[k])
                        .count()
                };
                let mut best_exact = 0usize;
                for s in slot.iter().flatten() {
                    best_exact = best_exact.max(exact(*s));
                }
                best_curve.push(best_exact);
                if best_exact == np {
                    let ci = slot.iter().position(|s| s.map(exact) == Some(np)).unwrap();
                    solution = Some(pop[ci].clone());
                    gen_found = gen;
                    break;
                }

                // Next generation: elitism + mutation + crossover + immigrants.
                // Parsimony: rank by fitness, then prefer the SMALLER program — a
                // pressure against GP bloat (equal-behaviour, minimal size).
                let mut order: Vec<usize> = (0..POP).collect();
                order.sort_by(|&a, &b| fit[b].cmp(&fit[a]).then(size(&pop[a]).cmp(&size(&pop[b]))));
                let elite_n = (POP / 10).max(2);
                let elite: Vec<Expr> = order[..elite_n].iter().map(|&i| pop[i].clone()).collect();
                let mut next = elite.clone();
                while next.len() < POP {
                    let r = rng.below(100);
                    let child = if r < 60 {
                        mutate(&elite[rng.below(elite_n as u32) as usize], &mut rng)
                    } else if r < 88 {
                        crossover(
                            &elite[rng.below(elite_n as u32) as usize],
                            &elite[rng.below(elite_n as u32) as usize],
                            &mut rng,
                        )
                    } else {
                        rand_expr(&mut rng, 5)
                    };
                    next.push(child);
                }
                pop = next;
            }

            // Progress trace (best matches per generation, sampled).
            print!("best fitness / {np} by gen: ");
            for (g, f) in best_curve.iter().enumerate() {
                if g % (best_curve.len() / 12).max(1) == 0 || g + 1 == best_curve.len() {
                    print!("g{g}:{f} ");
                }
            }
            println!();

            match solution {
                Some(sol) => {
                    println!(
                        "\n✓ SOLVED at generation {gen_found}: f(x,y) = {}",
                        show(&sol)
                    );
                    // Full-domain-ish check: does it match the target beyond the probes?
                    let sol_prog = linearize(&func_of(&sol), "run").unwrap();
                    let mut mism = 0usize;
                    let n_check = 200_000;
                    for _ in 0..n_check {
                        let a = [rng.u16(), rng.u16()];
                        if eval(&sol_prog, &a) != eval(&target_prog, &a) {
                            mism += 1;
                        }
                    }
                    if mism == 0 {
                        println!("  full-domain: 0/{n_check} mismatches on random inputs — genuinely equivalent.");
                    } else {
                        println!("  full-domain: {mism}/{n_check} mismatches — probe-equivalent but NOT the target");
                        println!("  (the classic synthesis gap: matching the probes ≠ matching the function).");
                    }
                }
                None => println!(
                    "\nnot solved in {MAX_GEN} generations (best {}/{np}).",
                    best_curve.iter().max().unwrap()
                ),
            }

            println!(
                "\nGPU fitness evaluation: {} candidate·example evals across {} generations",
                total_evals,
                best_curve.len()
            );
            println!(
            "  {:.1} ms total dispatch, {:.2e} evals/s — the whole population scored per generation",
            total_dispatch * 1e3,
            total_evals as f64 / total_dispatch
        );
            println!(
                "  in one launch. This is the scale (large population) where the backend wins;"
            );
            println!("  the same primitive that queries a library retrieves, and scores a population evolves.");
        }
    }
}
