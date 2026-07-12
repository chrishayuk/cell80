//! Cost-pressure equivalence search over the stdlib — the protocol pre-registered in
//! `../cell-cost-discovery-preregistration.md`, built before any search ran.
//!
//! For every **total** unary `u16 -> u16` stdlib cell (clean `Returned` on all 65,536
//! inputs), search pipeline compositions of other stdlib cells for a **full-domain
//! identical** implementation at **strictly lower mean cost** on the Z80 body. Equivalence
//! is exact table equality — a proof, never a fingerprint sample. Cost is
//! `cycles + P × trapped_ops`, with P *measured* in stage 0 as the differential between a
//! trap-free shift-add mul16 and a plain `a * b` trap cell over the same u8×u8 grid (the
//! substrate prices its own trap; shared call overhead cancels exactly).
//!
//! Search shape (pre-registered): depth ≤ 2 over the full vocabulary (unary cells + binary
//! cells with the second argument bound to one of 14 fixed constants), depth 3 by extending
//! the deduped, cost-pruned depth-2 frontier with unary ops only. Dedup keeps the
//! min-mean-cost chain per composed table — lossless for the mean objective, because any
//! extension's added cost depends only on the table, not the chain that produced it.
//! Chains are costed as the sum of separate per-op runs (d call overheads vs the target's
//! one), a bias *against* discovery.
//!
//! Depth-1 unary hits are admission-gate escapes (the gate's fingerprint is sampled), not
//! discoveries; they are reported separately.

// The hot loops walk several 65,536-entry tables in lockstep by index; iterator zips of
// three-plus tables read worse and optimize the same.
#![allow(clippy::needless_range_loop)]

use cell80::{
    discover_cell_files, Cartridge, CartridgeOpts, CellConfig, Halt, Runner, DEFAULT_CYCLES,
};
use rayon::prelude::*;
use std::collections::HashMap;

const DOMAIN: usize = 1 << 16;
/// Fixed second-argument constants for binary ops (pre-registered; 14 values).
const CONSTANTS: [u16; 14] = [
    0, 1, 2, 3, 4, 5, 8, 10, 16, 255, 256, 0x00FF, 0xFF00, 0xFFFF,
];
/// Depth-3 frontier cap — a scope cut, never silent: truncation is logged with counts.
const DEPTH3_FRONTIER_CAP: usize = 500_000;

const FNV0: u64 = 0xcbf29ce484222325;
#[inline(always)]
fn fnv_step(h: u64, w: u16) -> u64 {
    (h ^ w as u64).wrapping_mul(0x100000001b3)
}

/// One pipeline stage: a total cell (unary, or binary with its second argument bound),
/// tabulated over the full domain with per-input repriced and raw costs.
struct OpT {
    name: String,
    /// A bare unary stdlib cell (vs a binary cell partially applied at a constant).
    unary: bool,
    table: Vec<u16>,
    /// Per-input repriced cost: `cycles + P × trapped_ops`.
    rp: Vec<u32>,
    /// Per-input raw cycles (the P = 0 sensitivity lane).
    p0: Vec<u32>,
    hash: u64,
    mean_rp: f64,
    mean_p0: f64,
    /// Min per-input repriced cost — the safe lower bound any extension adds.
    min_rp: u32,
}

struct Target {
    op: usize,
    mean_rp: f64,
    mean_p0: f64,
}

#[derive(Clone)]
struct Hit {
    target: usize,
    chain: Vec<usize>,
    mean_rp: f64,
    mean_p0: f64,
}

/// Depth-2 frontier entry: the min-mean-cost chain known for one composed table.
struct F2 {
    hash: u64,
    i: u32,
    j: u32,
    mean_rp: f64,
    mean_p0: f64,
}

/// Tabulate one op over the full domain. `None` unless total (clean `Returned` everywhere).
fn build_op(cart: &Cartridge, fixed: Option<u16>, name: String, p_surcharge: u64) -> Option<OpT> {
    let prog = cart.z80().ok()?;
    let mut r = Runner::new(prog);
    let entry = cart.manifest.entry.clone();
    let mut table = vec![0u16; DOMAIN];
    let mut rp = vec![0u32; DOMAIN];
    let mut p0 = vec![0u32; DOMAIN];
    for v in 0..DOMAIN {
        let args = match fixed {
            Some(c) => [v as u16, c].to_vec(),
            None => [v as u16].to_vec(),
        };
        let f = r.run_fast(Some(&entry), &args, DEFAULT_CYCLES).ok()?;
        if !matches!(f.halt, Halt::Returned) {
            return None;
        }
        table[v] = f.result;
        p0[v] = f.cycles as u32;
        rp[v] = (f.cycles + p_surcharge * f.trapped_ops) as u32;
    }
    let mut hash = FNV0;
    let mut srp = 0u64;
    let mut sp0 = 0u64;
    let mut min_rp = u32::MAX;
    for v in 0..DOMAIN {
        hash = fnv_step(hash, table[v]);
        srp += rp[v] as u64;
        sp0 += p0[v] as u64;
        min_rp = min_rp.min(rp[v]);
    }
    Some(OpT {
        name,
        unary: fixed.is_none(),
        table,
        rp,
        p0,
        hash,
        mean_rp: srp as f64 / DOMAIN as f64,
        mean_p0: sp0 as f64 / DOMAIN as f64,
        min_rp,
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cells_dir = args.get(1).map(String::as_str).unwrap_or("cell80/cells");

    // ---- Stage 0: measure the trap surcharge P (differential, pre-registered). ----
    let p_surcharge = cell_cost_discovery::measure_p();

    // ---- Stage 1: load + compile the library, filter by signature. ----
    let files = discover_cell_files(cells_dir).expect("library dir");
    let sources: Vec<(String, String)> = files
        .iter()
        .filter_map(|p| {
            let id = p.file_stem()?.to_str()?.to_string();
            let src = std::fs::read_to_string(p).ok()?;
            Some((id, src))
        })
        .collect();
    // Library cells compile the way the library tests compile them (permissive config,
    // resident kernel bank available) — the sandboxed path rejects hundreds of shipped
    // cells and would silently shrink "library-wide" to "the easy slice".
    let compiled: Vec<(String, Option<Cartridge>)> = sources
        .par_iter()
        .map(|(id, src)| {
            let o = CartridgeOpts {
                id: Some(id.clone()),
                kernel_bank: true,
                ..Default::default()
            };
            let c = Cartridge::compile(src, CellConfig::permissive(), o).ok();
            (id.clone(), c)
        })
        .collect();
    let n_fail = compiled.iter().filter(|(_, c)| c.is_none()).count();
    if std::env::var("CCD_ERRORS").is_ok() {
        let mut counts: HashMap<String, (usize, String)> = HashMap::new();
        for (id, src) in &sources {
            let o = CartridgeOpts {
                id: Some(id.clone()),
                kernel_bank: true,
                ..Default::default()
            };
            if let Err(e) = Cartridge::compile(src, CellConfig::permissive(), o) {
                // Bucket by the error's first clause so one line per failure mode.
                let key: String = e.split([':', '`']).next().unwrap_or(&e).trim().to_string();
                let entry = counts.entry(key).or_insert((0, String::new()));
                entry.0 += 1;
                if entry.1.is_empty() {
                    entry.1 = format!("{id}: {}", e.chars().take(160).collect::<String>());
                }
            }
        }
        let mut rows: Vec<_> = counts.into_iter().collect();
        rows.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
        for (key, (n, sample)) in rows {
            println!("compile-fail x{n}: {key}\n    e.g. {sample}");
        }
        return;
    }
    let carts: Vec<(String, Cartridge)> = compiled
        .into_iter()
        .filter_map(|(id, c)| c.map(|c| (id, c)))
        .collect();
    let sig_is = |c: &Cartridge, arity: usize| {
        let s = &c.manifest.signature;
        s.state.is_empty()
            && s.ret == "u16"
            && s.params.len() == arity
            && s.params.iter().all(|(_, t)| t == "u16")
    };
    let unary_carts: Vec<usize> = (0..carts.len())
        .filter(|&i| sig_is(&carts[i].1, 1))
        .collect();
    let binary_carts: Vec<usize> = (0..carts.len())
        .filter(|&i| sig_is(&carts[i].1, 2))
        .collect();
    println!(
        "library: {} files, {} compiled ({n_fail} failed), {} unary u16->u16, {} binary",
        files.len(),
        carts.len(),
        unary_carts.len(),
        binary_carts.len()
    );

    // ---- Stage 2: tabulate the vocabulary (totality-filtered). ----
    let mut specs: Vec<(usize, Option<u16>)> = Vec::new();
    for &ci in &unary_carts {
        specs.push((ci, None));
    }
    for &ci in &binary_carts {
        for &c in &CONSTANTS {
            specs.push((ci, Some(c)));
        }
    }
    let built: Vec<Option<OpT>> = specs
        .par_iter()
        .map(|&(ci, fixed)| {
            let name = match fixed {
                None => carts[ci].0.clone(),
                Some(c) => format!("{}[b={c}]", carts[ci].0),
            };
            build_op(&carts[ci].1, fixed, name, p_surcharge)
        })
        .collect();
    let n_partial_unary = specs
        .iter()
        .zip(&built)
        .filter(|((_, f), b)| f.is_none() && b.is_none())
        .count();
    let ops: Vec<OpT> = built.into_iter().flatten().collect();
    let n_unary_ops = ops.iter().filter(|o| o.unary).count();
    println!(
        "vocabulary: {} total ops ({} unary targets/stages, {} binary-with-constant; {} unary cells excluded as partial)",
        ops.len(),
        n_unary_ops,
        ops.len() - n_unary_ops,
        n_partial_unary,
    );

    // ---- Targets: every total unary cell, with its own mean cost. ----
    let targets: Vec<Target> = ops
        .iter()
        .enumerate()
        .filter(|(_, o)| o.unary)
        .map(|(i, o)| Target {
            op: i,
            mean_rp: o.mean_rp,
            mean_p0: o.mean_p0,
        })
        .collect();
    let mut target_hash: HashMap<u64, Vec<usize>> = HashMap::new();
    for (t, tg) in targets.iter().enumerate() {
        target_hash.entry(ops[tg.op].hash).or_default().push(t);
    }
    let max_target_rp = targets.iter().map(|t| t.mean_rp).fold(0.0, f64::max);
    let min_step = ops.iter().map(|o| o.min_rp).min().unwrap_or(0) as f64;
    println!(
        "targets: {} (max mean cost {max_target_rp:.1}; min extension step {min_step:.1})",
        targets.len()
    );

    let mut hits: Vec<Hit> = Vec::new();
    let mut gate_escapes: Vec<(String, String)> = Vec::new();

    // ---- Depth 1: single ops. Unary-vs-unary equality = admission-gate escape audit;
    //      binary-with-constant matches are legitimate (partial-application) candidates. ----
    for (i, op) in ops.iter().enumerate() {
        if let Some(ts) = target_hash.get(&op.hash) {
            for &t in ts {
                let tg = &targets[t];
                if tg.op == i || op.table != ops[tg.op].table {
                    continue;
                }
                if op.unary {
                    // Symmetric pair; record once.
                    if i < tg.op {
                        gate_escapes.push((op.name.clone(), ops[tg.op].name.clone()));
                    }
                } else if op.mean_rp < tg.mean_rp {
                    hits.push(Hit {
                        target: t,
                        chain: vec![i],
                        mean_rp: op.mean_rp,
                        mean_p0: op.mean_p0,
                    });
                }
            }
        }
    }

    // ---- Depth 2: full vocabulary × full vocabulary, fused eval + hash + cost. ----
    let n = ops.len();
    let per_i: Vec<(Vec<F2>, Vec<Hit>)> = (0..n)
        .into_par_iter()
        .map(|i| {
            let mut frontier = Vec::new();
            let mut local_hits = Vec::new();
            if ops[i].mean_rp + min_step >= max_target_rp {
                return (frontier, local_hits);
            }
            let ti = &ops[i].table;
            for j in 0..n {
                let tj = &ops[j].table;
                let rpj = &ops[j].rp;
                let p0j = &ops[j].p0;
                let mut h = FNV0;
                let mut srp = 0u64;
                let mut sp0 = 0u64;
                for v in 0..DOMAIN {
                    let m = ti[v] as usize;
                    h = fnv_step(h, tj[m]);
                    srp += rpj[m] as u64;
                    sp0 += p0j[m] as u64;
                }
                let mean_rp = ops[i].mean_rp + srp as f64 / DOMAIN as f64;
                let mean_p0 = ops[i].mean_p0 + sp0 as f64 / DOMAIN as f64;
                if let Some(ts) = target_hash.get(&h) {
                    for &t in ts {
                        let tg = &targets[t];
                        let tt = &ops[tg.op].table;
                        if mean_rp < tg.mean_rp && (0..DOMAIN).all(|v| tj[ti[v] as usize] == tt[v])
                        {
                            local_hits.push(Hit {
                                target: t,
                                chain: vec![i, j],
                                mean_rp,
                                mean_p0,
                            });
                        }
                    }
                }
                if mean_rp + min_step < max_target_rp {
                    frontier.push(F2 {
                        hash: h,
                        i: i as u32,
                        j: j as u32,
                        mean_rp,
                        mean_p0,
                    });
                }
            }
            (frontier, local_hits)
        })
        .collect();

    // Dedup the frontier per composed table, keeping the min-mean chain (lossless for the
    // mean objective). A 64-bit hash collision here could merge two distinct tables and
    // drop a chain — ~1e-7 odds at this scale, accepted; target matches above are always
    // verified by full table compare, so no false hit is possible.
    let mut frontier2: HashMap<u64, F2> = HashMap::new();
    let mut n_l2_chains = 0usize;
    for (fs, hs) in per_i {
        hits.extend(hs);
        n_l2_chains += fs.len();
        for f in fs {
            match frontier2.get(&f.hash) {
                Some(e) if e.mean_rp <= f.mean_rp => {}
                _ => {
                    frontier2.insert(f.hash, f);
                }
            }
        }
    }
    println!(
        "depth 2: {n_l2_chains} viable chains, {} distinct tables in frontier",
        frontier2.len()
    );

    // ---- Depth 3: extend the frontier with unary ops only (pre-registered scope). ----
    let mut frontier: Vec<F2> = frontier2.into_values().collect();
    frontier.sort_by(|a, b| a.mean_rp.total_cmp(&b.mean_rp));
    if frontier.len() > DEPTH3_FRONTIER_CAP {
        println!(
            "depth-3 frontier TRUNCATED: {} -> {DEPTH3_FRONTIER_CAP} (cost-ordered; the cut is a scope reduction, logged per pre-registration)",
            frontier.len()
        );
        frontier.truncate(DEPTH3_FRONTIER_CAP);
    }
    let unary_idx: Vec<usize> = (0..n).filter(|&i| ops[i].unary).collect();
    let l3_hits: Vec<Hit> = frontier
        .par_iter()
        .map(|e| {
            let mut local = Vec::new();
            let ti = &ops[e.i as usize].table;
            let tj = &ops[e.j as usize].table;
            let mut t2 = vec![0u16; DOMAIN];
            for v in 0..DOMAIN {
                t2[v] = tj[ti[v] as usize];
            }
            for &k in &unary_idx {
                if e.mean_rp + ops[k].min_rp as f64 >= max_target_rp {
                    continue;
                }
                let tk = &ops[k].table;
                let rpk = &ops[k].rp;
                let p0k = &ops[k].p0;
                let mut h = FNV0;
                let mut srp = 0u64;
                let mut sp0 = 0u64;
                for v in 0..DOMAIN {
                    let m = t2[v] as usize;
                    h = fnv_step(h, tk[m]);
                    srp += rpk[m] as u64;
                    sp0 += p0k[m] as u64;
                }
                if let Some(ts) = target_hash.get(&h) {
                    let mean_rp = e.mean_rp + srp as f64 / DOMAIN as f64;
                    let mean_p0 = e.mean_p0 + sp0 as f64 / DOMAIN as f64;
                    for &t in ts {
                        let tg = &targets[t];
                        let tt = &ops[tg.op].table;
                        if mean_rp < tg.mean_rp && (0..DOMAIN).all(|v| tk[t2[v] as usize] == tt[v])
                        {
                            local.push(Hit {
                                target: t,
                                chain: vec![e.i as usize, e.j as usize, k],
                                mean_rp,
                                mean_p0,
                            });
                        }
                    }
                }
            }
            local
        })
        .reduce(Vec::new, |mut a, mut b| {
            a.append(&mut b);
            a
        });
    hits.extend(l3_hits);

    // ---- Report: best chain per target, both pricings, gate escapes. ----
    println!("\n=== gate-escape audit (depth-1 unary duplicates) ===");
    if gate_escapes.is_empty() {
        println!("none — no two unary cells share a full-domain table");
    }
    for (a, b) in &gate_escapes {
        println!(
            "DUPLICATE TABLES: {a} == {b} (admission fingerprint is sampled; this pair escaped)"
        );
    }

    println!(
        "\n=== cost-pressure hits (full-domain identical, strictly cheaper, repriced mean) ==="
    );
    let mut best: HashMap<usize, Hit> = HashMap::new();
    for h in &hits {
        match best.get(&h.target) {
            Some(e) if e.mean_rp <= h.mean_rp => {}
            _ => {
                best.insert(h.target, h.clone());
            }
        }
    }
    if best.is_empty() {
        println!(
            "none — zero hits at depth <= 3: the authored library is near-optimal under \
             pipeline composition at this cost model (pre-registered kill condition; a real \
             certification result, reported as such)"
        );
    }
    let mut ordered: Vec<&Hit> = best.values().collect();
    ordered.sort_by(|a, b| {
        let ra = a.mean_rp / targets[a.target].mean_rp;
        let rb = b.mean_rp / targets[b.target].mean_rp;
        ra.total_cmp(&rb)
    });
    for h in &ordered {
        let tg = &targets[h.target];
        let chain: Vec<&str> = h.chain.iter().map(|&i| ops[i].name.as_str()).collect();
        let p0_ok = h.mean_p0 < tg.mean_p0;
        println!(
            "{}  <-  {}\n    repriced mean: {:.1} vs {:.1}  ({:.2}x)\n    raw (P=0) mean: {:.1} vs {:.1}  ({})",
            ops[tg.op].name,
            chain.join(" |> "),
            h.mean_rp,
            tg.mean_rp,
            tg.mean_rp / h.mean_rp,
            h.mean_p0,
            tg.mean_p0,
            if p0_ok {
                "unconditional: survives P=0"
            } else {
                "repricing-dependent"
            },
        );
    }
    println!(
        "\ntotals: {} raw hits, {} targets improved, {} gate-escape pairs, P={p_surcharge}",
        hits.len(),
        best.len(),
        gate_escapes.len()
    );
}
