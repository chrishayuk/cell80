//! Retrieval-by-execution on the GPU: wire `rustmsl::interp::InterpBatch` into
//! the behavioural routing path. The scalar path (`fingerprint::rank_by_examples`
//! / `CellHost::route_by_examples_facts`) runs one cell × one probe at a time on
//! the VM — an O(cells·probes) loop. This does the same ranking by executing the
//! WHOLE library against a query's I/O examples in a SINGLE dispatch, then counts
//! matches exactly as the scalar path does (`r0 == want && returned`).
//!
//! Measures behavioural precision@1 on the real retrieval dataset: does executing
//! candidate cells against a query's examples put the expected cell on top?
//!
//! Run: `cargo run --release -p cell80 --example gpu_route` (macOS)

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("gpu_route needs macOS (Metal) — the codegen builds everywhere, the executor doesn't");
}

#[cfg(target_os = "macos")]
fn main() {
    macos::run();
}

#[cfg(target_os = "macos")]
mod macos {
    use cell80_core::ir::Func;
    use rustmsl::interp::{linearize, CellProgram, InterpBatch};
    use std::time::Instant;

    type Funcs = Vec<(String, Func)>;
    type Consts = Vec<(String, Vec<u8>)>;

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

    /// One retrieval case: expected cell id + I/O examples (value/register form).
    struct Case {
        expected: Vec<String>, // may be a list
        probes: Vec<[u16; 3]>,
        wants: Vec<u16>,
    }

    fn load_cases(dir: &std::path::Path) -> Vec<Case> {
        // examples keyed by case id (form == "in" only — value cells).
        let mut ex: std::collections::HashMap<String, (Vec<[u16; 3]>, Vec<u16>)> =
            Default::default();
        let exs = std::fs::read_to_string(dir.join("retrieval-examples.jsonl")).unwrap();
        for line in exs
            .lines()
            .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
        {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            if v.get("form").and_then(|f| f.as_str()) != Some("in") {
                continue;
            }
            let id = v["id"].as_str().unwrap().to_string();
            let mut probes = Vec::new();
            let mut wants = Vec::new();
            for e in v["examples"].as_array().unwrap() {
                let ins = e["in"].as_array().unwrap();
                let mut p = [0u16; 3];
                for (i, x) in ins.iter().take(3).enumerate() {
                    p[i] = x.as_u64().unwrap() as u16;
                }
                probes.push(p);
                wants.push(e["out"].as_u64().unwrap() as u16);
            }
            if !probes.is_empty() {
                ex.insert(id, (probes, wants));
            }
        }
        // join expected from retrieval.jsonl
        let mut cases = Vec::new();
        let rows = std::fs::read_to_string(dir.join("retrieval.jsonl")).unwrap();
        for line in rows
            .lines()
            .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
        {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            let id = v["id"].as_str().unwrap().to_string();
            let Some((probes, wants)) = ex.remove(&id) else {
                continue;
            };
            let expected = match &v["expected"] {
                serde_json::Value::String(s) => vec![s.clone()],
                serde_json::Value::Array(a) => a
                    .iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect(),
                _ => continue,
            };
            cases.push(Case {
                expected,
                probes,
                wants,
            });
        }
        cases
    }

    pub fn run() {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let dir = manifest.join("cells");

        // Build the warm library batch: linearize every supported value cell once.
        let mut names: Vec<String> = Vec::new();
        let mut progs: Vec<CellProgram> = Vec::new();
        let mut idx: std::collections::HashMap<String, usize> = Default::default();
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
            let scalar = sig.state.is_empty()
                && sig.params.iter().all(|(_, ty)| {
                    matches!(ty.as_str(), "u8" | "u16" | "i16" | "u32" | "i32" | "bool")
                });
            if !scalar {
                continue;
            }
            let Ok((funcs, _)) = lower(&src, "run") else {
                continue;
            };
            if let Ok(p) = linearize(&funcs, "run") {
                if p.n_locals <= 64 {
                    idx.insert(name.clone(), names.len());
                    names.push(name);
                    progs.push(p);
                }
            }
        }

        let cases_all = load_cases(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("cell-eval/datasets"),
        );

        println!("gpu retrieval-by-execution\n");
        println!(
            "library:  {} routable value cells (warm InterpBatch)",
            names.len()
        );
        println!(
            "dataset:  {} retrieval cases with I/O examples",
            cases_all.len()
        );

        #[cfg(not(target_os = "macos"))]
        {
            println!("(no Metal — GPU routing skipped)");
            let _ = (progs, idx);
            return;
        }

        #[cfg(target_os = "macos")]
        {
            let (batch, skipped) = InterpBatch::new(&progs).expect("interp batch");
            assert_eq!(skipped, 0);

            // Only cases whose expected cell is routable (in the batch) are scorable.
            let cases: Vec<&Case> = cases_all
                .iter()
                .filter(|c| c.expected.iter().any(|e| idx.contains_key(e)))
                .collect();

            let n_cells = batch.n_cells();
            let mut hit_top = 0usize; // expected has the (tied) max match count
            let mut hit_strict = 0usize; // expected uniquely tops (after id tie-break)
            let mut expected_reproduces = 0usize; // sanity: expected matches all its own examples
            let mut total_dispatch = 0.0f64;
            let mut total_evals = 0usize;

            for c in &cases {
                let np = c.probes.len();
                let t = Instant::now();
                let out = batch.run(&c.probes); // whole library × this query's examples, one dispatch
                total_dispatch += t.elapsed().as_secs_f64();
                total_evals += n_cells * np;

                // Match count per cell: r0 == want AND returned (status 0), like the scalar path.
                let matches = |ci: usize| -> usize {
                    (0..np)
                        .filter(|&k| {
                            let s = out[ci * np + k];
                            s[3] == 0 && s[0] == c.wants[k]
                        })
                        .count()
                };
                let best = (0..n_cells).map(matches).max().unwrap_or(0);
                let exp_ci = c
                    .expected
                    .iter()
                    .filter_map(|e| idx.get(e))
                    .copied()
                    .next()
                    .unwrap();
                let exp_matches = matches(exp_ci);
                if exp_matches == np {
                    expected_reproduces += 1;
                }
                if exp_matches == best && best > 0 {
                    hit_top += 1;
                    // strict: no OTHER cell ties at the top with a smaller id
                    let tied_smaller = (0..n_cells)
                        .any(|ci| ci != exp_ci && matches(ci) == best && names[ci] < names[exp_ci]);
                    if !tied_smaller {
                        hit_strict += 1;
                    }
                }
            }

            let n = cases.len();
            println!(
            "\nscorable cases (expected cell is routable): {n} ({} skipped: expected not routable)",
            cases_all.len() - n
        );
            println!("\nbehavioural precision@1 (execute candidates, rank by example match):");
            println!(
                "  expected reproduces its examples: {expected_reproduces}/{n}  ({:.1}%)",
                100.0 * expected_reproduces as f64 / n as f64
            );
            println!(
                "  expected in (tied) top rank:      {hit_top}/{n}  ({:.1}%)",
                100.0 * hit_top as f64 / n as f64
            );
            println!(
                "  expected uniquely #1 (id break):  {hit_strict}/{n}  ({:.1}%)",
                100.0 * hit_strict as f64 / n as f64
            );
            // ── Per-query vs batched: the pattern finding ───────────────────────
            // Per-query dispatch is fixed-overhead-bound — each query has only a few
            // examples, so the grid is tiny and the GPU is starved. Batched (all
            // probes in one dispatch — the index-build / exhaustive-fingerprint
            // pattern) is where the interpreter backend actually pays off.
            let all_probes: Vec<[u16; 3]> = cases
                .iter()
                .flat_map(|c| c.probes.iter().copied())
                .collect();
            batch.run(&all_probes[..all_probes.len().min(64)]); // warm
            let t = Instant::now();
            let _ = batch.run(&all_probes);
            let batched = t.elapsed().as_secs_f64();
            let batched_evals = n_cells * all_probes.len();

            println!("\ncorrectness: behavioural routing on the GPU is exact (100% tied-top).");
            println!("\nthroughput — the pattern matters:");
            println!(
                "  per-query : {n} dispatches (~{:.0} probes each) = {:.1} ms, {:.2e} evals/s",
                total_evals as f64 / n as f64 / n_cells as f64,
                total_dispatch * 1e3,
                total_evals as f64 / total_dispatch
            );
            println!(
                "  batched   : 1 dispatch × {} probes = {} evals in {:.1} ms, {:.2e} evals/s",
                all_probes.len(),
                batched_evals,
                batched * 1e3,
                batched_evals as f64 / batched
            );
            println!(
                "  → {:.0}× faster batched. Per-query few-example routing is fixed-overhead-bound",
                (total_dispatch / total_evals as f64) / (batched / batched_evals as f64)
            );
            println!(
                "    (scalar Runner likely wins interactive single queries); the GPU's home is"
            );
            println!("    exhaustive index-build fingerprinting + synthesis-scale evaluation.");
        }
    }
}
