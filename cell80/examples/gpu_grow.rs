//! Grow a real library cell by GPU-scored evolution over the EXISTING library.
//!
//! The interpreter's inliner is the composition engine: a candidate that `Call`s
//! library cells linearizes (inlines) into one bytecode program. So we evolve
//! chains of existing cells, score the whole population against a target's I/O in
//! one `InterpBatch` dispatch per generation, and check the winner is (a)
//! full-domain-correct and (b) novel — behaviourally distinct from every library
//! cell — i.e. it would earn admission. Structured search (compose existing
//! cells) is the tractable route free-form arithmetic synthesis is not.
//!
//! Run: `cargo run --release -p cell80 --example gpu_grow` (macOS)

use cell80_core::ir::{Expr, Func};
use rustmsl::interp::{cpu_run, linearize, CellProgram, VmOut};
use std::time::Instant;

const POP: usize = 4096;
const MAX_GEN: usize = 120;
const MAX_LEN: usize = 6;
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

/// A chain `c_{k-1}(...c_0(x)...)` as a candidate function that Calls cells by
/// name — the inliner composes it at linearize time.
fn chain_func(chain: &[usize], unary: &[String]) -> Func {
    let mut e = Expr::Var(0);
    for &c in chain {
        e = Expr::Call(unary[c].clone(), vec![e]);
    }
    Func {
        params: 1,
        n_locals: 1,
        body: vec![],
        ret: vec![e],
        wide_param: false,
        wide_second: false,
        wide_ret: false,
    }
}

/// Linearize a chain against the library pool (inlines the cell calls).
fn chain_prog(chain: &[usize], unary: &[String], pool: &[(String, Func)]) -> Option<CellProgram> {
    let mut all = Vec::with_capacity(pool.len() + 1);
    all.push((CANDIDATE.to_string(), chain_func(chain, unary)));
    all.extend(pool.iter().cloned());
    match linearize(&all, CANDIDATE) {
        Ok(p) if p.max_depth <= 32 && p.n_locals <= 64 => Some(p),
        _ => None,
    }
}

fn eval(prog: &CellProgram, x: u16) -> Option<u16> {
    match cpu_run(prog, &[x]) {
        VmOut::Value(v, _) => v.first().copied(),
        _ => None,
    }
}

/// Fingerprint a unary program's outputs on the probe bank (Some=returned).
fn fp(prog: &CellProgram, probes: &[u16]) -> Vec<Option<u16>> {
    probes.iter().map(|&x| eval(prog, x)).collect()
}
fn agreement(a: &[Option<u16>], b: &[Option<u16>]) -> f32 {
    let n = a.len().min(b.len());
    if n == 0 {
        return 1.0;
    }
    a.iter().zip(b).filter(|(x, y)| x == y).count() as f32 / n as f32
}

fn main() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = manifest.join("cells");

    // Discover unary (arity-1) value cells that lower to a single self-contained
    // function (the building blocks the inliner can compose), plus a fingerprint
    // for every routable value cell (for the novelty check).
    let mut unary: Vec<String> = Vec::new();
    let mut pool: Vec<(String, Func)> = Vec::new();
    let mut lib_fp: Vec<(String, Vec<Option<u16>>)> = Vec::new();
    let fp_probes: Vec<u16> = {
        let mut r = Rng(0xC0FFEE);
        (0..32).map(|_| r.u16()).collect()
    };

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
        let arity1 = sig.state.is_empty()
            && sig.params.len() == 1
            && matches!(sig.params[0].1.as_str(), "u8" | "u16" | "i16" | "bool");
        let Ok(funcs) = lower(&src) else { continue };
        // single self-contained function ⇒ a clean building block
        if funcs.len() == 1 {
            if let Ok(p) = linearize(&funcs, "run") {
                lib_fp.push((name.clone(), fp(&p, &fp_probes)));
                if arity1 && p.n_locals <= 64 {
                    unary.push(name.clone());
                    pool.push((name.clone(), funcs[0].1.clone()));
                }
            }
        }
    }

    println!("GPU cell growth by evolution over the existing library\n");
    println!(
        "building blocks: {} unary cells (single-func, inlinable)",
        unary.len()
    );

    // Target: a 2-cell composition that is a genuine gap (behaviourally distinct
    // from every library cell). Pick the first pair whose composition is novel.
    let mut rng = Rng(0x5EED_1234_ABCD_0001);
    let probes: Vec<[u16; 3]> = (0..48).map(|_| [rng.u16(), 0, 0]).collect();
    let mut target: Option<(usize, usize, CellProgram)> = None;
    'pick: for a in 0..unary.len() {
        for b in 0..unary.len() {
            if a == b {
                continue;
            }
            if let Some(tp) = chain_prog(&[a, b], &unary, &pool) {
                let tfp = fp(&tp, &fp_probes);
                // novel ⇒ not behaviourally identical to any single library cell
                let dup = lib_fp.iter().any(|(_, f)| agreement(&tfp, f) >= 1.0);
                // non-degenerate ⇒ actually varies over the probes
                let varies = tfp
                    .iter()
                    .flatten()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    > 3;
                if !dup && varies {
                    target = Some((a, b, tp));
                    break 'pick;
                }
            }
        }
    }
    let Some((ta, tb, target_prog)) = target else {
        println!("no novel 2-cell target found");
        return;
    };
    let wants: Vec<u16> = probes
        .iter()
        .map(|p| eval(&target_prog, p[0]).unwrap())
        .collect();
    let np = probes.len();
    println!(
        "target (a gap): {}({}(x)) — a composition no single cell reproduces",
        unary[tb], unary[ta]
    );
    println!(
        "search: population {POP}, chains of ≤{MAX_LEN} cells, scored on GPU per generation\n"
    );

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (wants, np, MAX_GEN);
        println!("(no Metal — skipped)");
    }

    #[cfg(target_os = "macos")]
    {
        use rustmsl::interp::InterpBatch;
        let rand_chain = |rng: &mut Rng| -> Vec<usize> {
            let len = 1 + rng.below(MAX_LEN);
            (0..len).map(|_| rng.below(unary.len())).collect()
        };
        let mut population: Vec<Vec<usize>> = (0..POP).map(|_| rand_chain(&mut rng)).collect();
        // A length-1 chain always linearizes — a valid seed to compile the kernel.
        let seed = chain_prog(&[0], &unary, &pool).expect("single-cell chain linearizes");
        let (mut batch, _) = InterpBatch::new(&[seed]).expect("metal");

        let mut solution: Option<Vec<usize>> = None;
        let mut gen_found = 0;
        let mut total_evals = 0usize;
        let mut total_dt = 0.0;
        let mut best_curve = Vec::new();

        for gen in 0..MAX_GEN {
            let mut progs = Vec::new();
            let mut slot = Vec::with_capacity(POP);
            for ch in &population {
                match chain_prog(ch, &unary, &pool) {
                    Some(p) => {
                        slot.push(Some(progs.len()));
                        progs.push(p);
                    }
                    None => slot.push(None),
                }
            }
            batch.reload(&progs);
            let t = Instant::now();
            let out = batch.run(&probes);
            total_dt += t.elapsed().as_secs_f64();
            total_evals += progs.len() * np;

            let exact = |bi: usize| {
                (0..np)
                    .filter(|&k| out[bi * np + k][3] == 0 && out[bi * np + k][0] == wants[k])
                    .count()
            };
            // fitness = correct output bits (gradient); solution = all probes exact
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
            let mut best_exact = 0;
            for s in slot.iter().flatten() {
                best_exact = best_exact.max(exact(*s));
            }
            best_curve.push(best_exact);
            if best_exact == np {
                let ci = slot.iter().position(|s| s.map(exact) == Some(np)).unwrap();
                solution = Some(population[ci].clone());
                gen_found = gen;
                break;
            }

            // next gen: elitism (parsimony tie-break) + mutation + immigrants
            let mut order: Vec<usize> = (0..POP).collect();
            order.sort_by(|&a, &b| {
                fit[b]
                    .cmp(&fit[a])
                    .then(population[a].len().cmp(&population[b].len()))
            });
            let elite_n = (POP / 10).max(2);
            let elite: Vec<Vec<usize>> = order[..elite_n]
                .iter()
                .map(|&i| population[i].clone())
                .collect();
            let mut next = elite.clone();
            while next.len() < POP {
                if rng.below(100) < 15 {
                    next.push(rand_chain(&mut rng));
                    continue;
                }
                let mut c = elite[rng.below(elite_n)].clone();
                let len = c.len();
                match rng.below(4) {
                    0 if len > 0 => {
                        let i = rng.below(len);
                        c[i] = rng.below(unary.len());
                    } // point
                    1 if len < MAX_LEN => {
                        let (i, v) = (rng.below(len + 1), rng.below(unary.len()));
                        c.insert(i, v);
                    } // insert
                    2 if len > 1 => {
                        let i = rng.below(len);
                        c.remove(i);
                    } // delete
                    _ if len > 1 => {
                        let (i, j) = (rng.below(len), rng.below(len));
                        c.swap(i, j);
                    }
                    _ => {}
                }
                next.push(c);
            }
            population = next;
        }

        print!("best exact / {np} by gen: ");
        for (g, f) in best_curve.iter().enumerate() {
            if g % (best_curve.len() / 10).max(1) == 0 || g + 1 == best_curve.len() {
                print!("g{g}:{f} ");
            }
        }
        println!("\n");

        match solution {
            Some(sol) => {
                let chain_str: Vec<&str> = sol.iter().map(|&c| unary[c].as_str()).collect();
                println!("✓ GREW a cell at generation {gen_found}:");
                println!("    {} applied to x, inner-to-outer", chain_str.join(" → "));
                // full-domain verification vs the target reference
                let sp = chain_prog(&sol, &unary, &pool).unwrap();
                let mut mism = 0usize;
                for x in 0..=u16::MAX {
                    if eval(&sp, x) != eval(&target_prog, x) {
                        mism += 1;
                    }
                }
                println!("    full-domain (all 65536 u16 inputs): {mism} mismatches vs target");
                // novelty: behaviourally distinct from every library cell?
                let sfp = fp(&sp, &fp_probes);
                let (best_id, best_a) = lib_fp
                    .iter()
                    .map(|(id, f)| (id.as_str(), agreement(&sfp, f)))
                    .max_by(|a, b| a.1.total_cmp(&b.1))
                    .unwrap();
                println!(
                    "    novelty: closest library cell is `{best_id}` at agreement {:.3} (1.0 = duplicate)",
                    best_a
                );
                if mism == 0 && best_a < 0.834 {
                    println!("\n  → full-domain-correct AND novel: this composition would earn admission.");
                    println!("    A new cell, GROWN not authored — the library-growth engine, GPU-scored.");
                } else if mism == 0 {
                    println!("\n  → full-domain-correct but close to `{best_id}` — admission's dedup gate would judge.");
                }
            }
            None => println!(
                "not grown in {MAX_GEN} generations (best {}/{np})",
                best_curve.iter().max().unwrap()
            ),
        }

        println!(
            "\nGPU: {} candidate·example evals in {:.0} ms ({:.2e}/s) — a population of compositions",
            total_evals, total_dt * 1e3, total_evals as f64 / total_dt
        );
        println!("scored against the target per generation, in one dispatch.");
    }
}
