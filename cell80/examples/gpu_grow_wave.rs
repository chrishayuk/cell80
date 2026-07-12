//! Grow a WAVE of cells — the library-growth engine running, compounding.
//!
//! Target a batch of gaps; for each, GPU-scored evolution composes existing
//! cells into a candidate, verifies it full-domain and behaviourally novel, and
//! ADMITS it as a new building block — so later cells can compose earlier grown
//! ones and the library grows itself. One `InterpBatch` (kernel compiled once,
//! buffer reloaded per generation) scores every population.
//!
//! Run: `cargo run --release -p cell80 --example gpu_grow_wave` (macOS)

use cell80_core::ir::{Expr, Func};
use rustmsl::interp::{cpu_run, linearize, CellProgram, VmOut};
use std::time::Instant;

const POP: usize = 2048;
const MAX_GEN: usize = 80;
const MAX_LEN: usize = 5;
const WAVE: usize = 12; // gaps to attempt
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

fn chain_func(chain: &[usize], names: &[String]) -> Func {
    let mut e = Expr::Var(0);
    for &c in chain {
        e = Expr::Call(names[c].clone(), vec![e]);
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

/// Linearize a chain against the (growing) pool — inlines the cell calls.
fn chain_prog(chain: &[usize], names: &[String], pool: &[(String, Func)]) -> Option<CellProgram> {
    let mut all = Vec::with_capacity(pool.len() + 1);
    all.push((CANDIDATE.to_string(), chain_func(chain, names)));
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
fn varied(f: &[Option<u16>]) -> bool {
    f.iter()
        .flatten()
        .collect::<std::collections::HashSet<_>>()
        .len()
        > 3
}

fn main() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = manifest.join("cells");
    let fp_probes: Vec<u16> = {
        let mut r = Rng(0xC0FFEE);
        (0..32).map(|_| r.u16()).collect()
    };

    // Building blocks (unary single-func cells) + a fingerprint per value cell.
    let mut names: Vec<String> = Vec::new();
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
        let arity1 = sig.state.is_empty()
            && sig.params.len() == 1
            && matches!(sig.params[0].1.as_str(), "u8" | "u16" | "i16" | "bool");
        let Ok(funcs) = lower(&src) else { continue };
        if funcs.len() == 1 {
            if let Ok(p) = linearize(&funcs, "run") {
                lib_fp.push((name.clone(), fp(&p, &fp_probes)));
                if arity1 && p.n_locals <= 64 {
                    names.push(name.clone());
                    pool.push((name.clone(), funcs[0].1.clone()));
                }
            }
        }
    }
    let base_blocks = names.len();
    println!("GPU library-growth wave\n");
    println!(
        "start: {base_blocks} unary building blocks, {} library fingerprints\n",
        lib_fp.len()
    );

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (pool, MAX_GEN, WAVE);
        println!("(no Metal — skipped)");
    }

    #[cfg(target_os = "macos")]
    {
        use rustmsl::interp::InterpBatch;
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        let probes: Vec<[u16; 3]> = (0..48).map(|_| [rng.u16(), 0, 0]).collect();
        let np = probes.len();
        let seed = chain_prog(&[0], &names, &pool).expect("seed");
        let (mut batch, _) = InterpBatch::new(&[seed]).expect("metal");

        let mut total_evals = 0usize;
        let mut total_dt = 0.0;
        let mut grown: Vec<(String, Vec<usize>, f32)> = Vec::new();

        for w in 0..WAVE {
            // Pick a novel, non-degenerate gap: a random 2–3 block composition
            // (blocks include already-grown cells ⇒ the library compounds).
            let mut target: Option<(Vec<usize>, CellProgram, Vec<Option<u16>>)> = None;
            for _ in 0..400 {
                let len = 2 + rng.below(2);
                let ch: Vec<usize> = (0..len).map(|_| rng.below(names.len())).collect();
                let Some(tp) = chain_prog(&ch, &names, &pool) else {
                    continue;
                };
                let tfp = fp(&tp, &fp_probes);
                // novel, non-degenerate, AND returns cleanly on every scoring probe
                // (a growable cell that always produces a value).
                if varied(&tfp)
                    && lib_fp.iter().all(|(_, f)| agreement(&tfp, f) < 1.0)
                    && probes.iter().all(|p| eval(&tp, p[0]).is_some())
                {
                    target = Some((ch, tp, tfp));
                    break;
                }
            }
            let Some((_tch, target_prog, _tfp)) = target else {
                continue;
            };
            let wants: Vec<u16> = probes
                .iter()
                .map(|p| eval(&target_prog, p[0]).unwrap())
                .collect();

            // Evolve a composition to match the target's I/O (GPU-scored).
            let mut pop: Vec<Vec<usize>> = (0..POP)
                .map(|_| {
                    (0..1 + rng.below(MAX_LEN))
                        .map(|_| rng.below(names.len()))
                        .collect()
                })
                .collect();
            let mut solved: Option<Vec<usize>> = None;
            for _ in 0..MAX_GEN {
                let mut progs = Vec::new();
                let mut slot = Vec::with_capacity(POP);
                for ch in &pop {
                    match chain_prog(ch, &names, &pool) {
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
                if let Some(bi) = slot.iter().flatten().find(|&&bi| exact(bi) == np) {
                    let ci = slot.iter().position(|s| *s == Some(*bi)).unwrap();
                    solved = Some(pop[ci].clone());
                    break;
                }
                let mut order: Vec<usize> = (0..POP).collect();
                order.sort_by(|&a, &b| fit[b].cmp(&fit[a]).then(pop[a].len().cmp(&pop[b].len())));
                let en = (POP / 10).max(2);
                let elite: Vec<Vec<usize>> = order[..en].iter().map(|&i| pop[i].clone()).collect();
                let mut next = elite.clone();
                while next.len() < POP {
                    if rng.below(100) < 15 {
                        next.push(
                            (0..1 + rng.below(MAX_LEN))
                                .map(|_| rng.below(names.len()))
                                .collect(),
                        );
                        continue;
                    }
                    let mut c = elite[rng.below(en)].clone();
                    let len = c.len();
                    match rng.below(4) {
                        0 if len > 0 => {
                            let i = rng.below(len);
                            c[i] = rng.below(names.len());
                        }
                        1 if len < MAX_LEN => {
                            let (i, v) = (rng.below(len + 1), rng.below(names.len()));
                            c.insert(i, v);
                        }
                        2 if len > 1 => {
                            let i = rng.below(len);
                            c.remove(i);
                        }
                        _ if len > 1 => {
                            let (i, j) = (rng.below(len), rng.below(len));
                            c.swap(i, j);
                        }
                        _ => {}
                    }
                    next.push(c);
                }
                pop = next;
            }

            // Verify + admit.
            let Some(sol) = solved else {
                println!("  gap {:>2}: unsolved", w + 1);
                continue;
            };
            let sp = chain_prog(&sol, &names, &pool).unwrap();
            let mism = (0..=u16::MAX)
                .filter(|&x| eval(&sp, x) != eval(&target_prog, x))
                .count();
            let sfp = fp(&sp, &fp_probes);
            let (best_id, best_a) = lib_fp
                .iter()
                .map(|(id, f)| (id.clone(), agreement(&sfp, f)))
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .unwrap();
            let chain_str: Vec<&str> = sol.iter().map(|&c| names[c].as_str()).collect();

            if mism == 0 && best_a < 0.834 {
                let gid = format!("grown_{}", grown.len());
                println!("  gap {:>2}: ✓ ADMIT {gid} = [{}]  (full-domain 0/65536, novelty {best_a:.3} vs {best_id})", w + 1, chain_str.join(" → "));
                // Compound: the grown cell becomes a building block + a fingerprint.
                names.push(gid.clone());
                pool.push((gid.clone(), chain_func(&sol, &names[..names.len() - 1])));
                lib_fp.push((gid.clone(), sfp));
                grown.push((gid, sol, best_a));
            } else if mism == 0 {
                println!("  gap {:>2}: ~ solved [{}] but agrees {best_a:.3} with {best_id} — dedup gate would reject", w + 1, chain_str.join(" → "));
            } else {
                println!("  gap {:>2}: solved on probes [{}] but {mism}/65536 full-domain mismatches — rejected", w + 1, chain_str.join(" → "));
            }
        }

        println!(
            "\nwave complete: {} cells GROWN and admitted (library {base_blocks} → {} blocks)",
            grown.len(),
            names.len()
        );
        println!(
            "GPU: {} candidate·example evals in {:.1} s ({:.2e}/s) across the whole wave",
            total_evals,
            total_dt,
            total_evals as f64 / total_dt.max(1e-9)
        );
        println!("Each grown cell is full-domain-verified and novel — grown, not authored.");
    }
}
