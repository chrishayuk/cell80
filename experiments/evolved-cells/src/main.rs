//! Runs the protocol pre-registered in `../evolved-cells-preregistration.md`. For each fixed
//! arity-1 target: search for a chain of real, existing stdlib cells (`cell80::synthesize` —
//! A* — for every target; also `cell_synth_evolve::{evolve, mcts, portfolio}` — the *actual*
//! GA/MCTS code from `cell-synth-evolve`, reused via its lib, not duplicated — for the
//! deliberately harder targets, `Target::harder`), validate the chain over the *entire* u16
//! domain (not just the probes used to search), hand-compose it into one candidate cell
//! source (general, `syn`-based codegen in `lib.rs`), and check that candidate against the
//! real admission-gate mechanism (`cell80::{Fingerprint, DEFAULT_PROBES}`) against every cell
//! currently in `cell80/cells/*.rs` — not a reimplementation of the gate, the actual
//! fingerprint code.
//!
//! The pre-registration's original "smooth targets, A* suffices, skip GA/MCTS" scope
//! reduction turned out not to hold in general: `mystery_bits_2` breaks A* outright — no chain
//! found within budget — while GA and MCTS both find one reliably. `bin/boundary_sweep.rs`
//! maps where that failure boundary actually sits, instead of the one data point found here.
//! See `../evolved-cells-findings.md` for the full account.
use cell80::{synthesize, Cartridge, CartridgeOpts, CellConfig, Fingerprint};
use cell_synth_evolve::{evolve, mcts, portfolio, summarize};
use evolved_cells::{
    build_ops, codegen, compile, digital_root_ref, high_byte_popcount_ref, indent,
    is_semiprime_ref, low_byte_popcount_ref, mystery_bits_2_ref, mystery_bits_ref,
    rotated_low_byte_popcount_ref, MAX_POOL,
};
use std::fs;
use std::path::Path;

const DUPLICATE_AGREEMENT: f32 = 1.0; // mirrors cell80/src/admission.rs's own threshold
const BUDGET: usize = 500_000;

struct Target {
    name: &'static str,
    oracle: fn(u16) -> u16,
    max_depth: usize,
    /// `//! summary` / `//! tags:` for a passing candidate — real cell sources carry these,
    /// and `library_cartridge` (`cell80/src/cli.rs`) parses them into the manifest, so a
    /// candidate without them wouldn't look like a real contribution to the actual gate.
    summary: &'static str,
    tags: &'static str,
    /// Also run GA/MCTS/portfolio (not just A*) — reserved for the deliberately harder
    /// targets, to bound runtime rather than running the full search-method comparison on
    /// every easy target too.
    harder: bool,
}

fn main() {
    let cells_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cell80/cells");
    let (ops, op_meta) = build_ops(&cells_dir, MAX_POOL);
    println!("Op pool: {} ops built from real stdlib cells.\n", ops.len());

    // Fingerprint every currently-real library cell once, up front, for comparison — this is
    // the actual mechanism `cell80::admission::admit` uses, applied here to a candidate
    // outside the gate (this never calls or touches admission.rs itself).
    let library_fps: Vec<(String, Fingerprint)> = fs::read_dir(&cells_dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", cells_dir.display()))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "rs"))
        .filter_map(|e| {
            e.path()
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
        })
        .filter_map(|name| {
            let src = fs::read_to_string(cells_dir.join(format!("{name}.rs"))).ok()?;
            let cart = Cartridge::compile(
                &src,
                CellConfig::sandboxed(),
                CartridgeOpts {
                    id: Some(name.clone()),
                    ..Default::default()
                },
            )
            .ok()?;
            Some((name, Fingerprint::of(&cart)))
        })
        .collect();
    println!(
        "Fingerprinted {} existing library cells for comparison.\n",
        library_fps.len()
    );

    let targets: Vec<Target> = vec![
        Target {
            name: "digital_root",
            oracle: digital_root_ref,
            max_depth: 5,
            summary: "Digital root of n: repeatedly sum digits until one digit remains.",
            tags: "number, digits, digital-root, decimal, reduce, math",
            harder: false,
        },
        Target {
            name: "low_byte_popcount",
            oracle: low_byte_popcount_ref,
            max_depth: 3,
            summary: "Population count of just the low byte of x (high byte ignored).",
            tags: "bits, popcount, byte, low, count, ones, bitcount",
            harder: false,
        },
        Target {
            name: "high_byte_popcount",
            oracle: high_byte_popcount_ref,
            max_depth: 3,
            summary: "Population count of just the high byte of x (low byte ignored).",
            tags: "bits, popcount, byte, high, count, ones, bitcount",
            harder: false,
        },
        Target {
            name: "is_semiprime",
            oracle: is_semiprime_ref,
            max_depth: 5,
            summary: "1 if n is a product of exactly two primes (with multiplicity), else 0.",
            tags: "number, prime, semiprime, factorization, predicate",
            harder: false,
        },
        Target {
            name: "rotated_low_byte_popcount",
            oracle: rotated_low_byte_popcount_ref,
            max_depth: 4,
            summary: "Population count of the low byte of x after rotating its bits left by 4.",
            tags: "bits, popcount, rotate, byte, count, ones",
            harder: false,
        },
        Target {
            name: "mystery_bits",
            oracle: mystery_bits_ref,
            max_depth: 6,
            summary: "Popcount of x, OR'd with 0xAAAA, rotated left 8, XOR'd with 0x5555.",
            tags: "bits, popcount, mask, rotate, xor, experimental",
            harder: true,
        },
        Target {
            name: "mystery_bits_2",
            oracle: mystery_bits_2_ref,
            max_depth: 8,
            summary: "Popcount of a 6-step OR/rotate/AND/rotate/XOR mask chain over x.",
            tags: "bits, popcount, mask, rotate, xor, experimental",
            harder: true,
        },
    ];

    // 39999 is load-bearing, not decorative: digit_sum(39999)=39, digit_sum(39)=12,
    // digit_sum(12)=3 — a chain of only 2 digit_sum applications gives 12 here, not the
    // correct 3, which is exactly the gap the first run's full-domain check caught (a 2-pass
    // chain matched every other probe below but was wrong for 8,075/65,536 real inputs).
    const PROBES: &[u16] = &[
        0, 1, 4, 6, 9, 10, 99, 255, 256, 0x0F0F, 0xAAAA, 0x5555, 0xFF00, 0x00FF, 9999, 39999,
        59999, 65535,
    ];

    const HARDER_SEEDS: &[u64] = &[1, 2, 3, 4, 5];

    let mut passes = 0usize;
    for t in &targets {
        println!("=== {} (max_depth={}) ===", t.name, t.max_depth);
        let examples: Vec<(u16, u16)> = PROBES.iter().map(|&x| (x, (t.oracle)(x))).collect();

        let astar_plan = synthesize(&examples, &ops, t.max_depth, BUDGET);
        match &astar_plan {
            Some(p) => println!("  A*:   found {:?} ({} nodes tested)", p.steps, p.tested),
            None => println!(
                "  A*:   no chain found (budget {BUDGET}, depth {})",
                t.max_depth
            ),
        }

        let plan = if t.harder {
            // Also run GA/MCTS/portfolio — not just A* — to test whether they find something
            // A* can't at this pool size (or find it far more cheaply), instead of only
            // reporting whatever A* alone returns.
            let ga_results: Vec<_> = HARDER_SEEDS
                .iter()
                .map(|&seed| evolve(&examples, &ops, t.max_depth, BUDGET, seed))
                .collect();
            let mcts_results: Vec<_> = HARDER_SEEDS
                .iter()
                .map(|&seed| mcts(&examples, &ops, t.max_depth, BUDGET, seed))
                .collect();
            println!("  GA:   {}", summarize(&ga_results));
            println!("  MCTS: {}", summarize(&mcts_results));
            let portfolio_results: Vec<_> = (0..HARDER_SEEDS.len())
                .map(|i| {
                    portfolio(&[
                        astar_plan.clone(),
                        ga_results[i].clone(),
                        mcts_results[i].clone(),
                    ])
                })
                .collect();
            println!(
                "  Portfolio (best of A*/GA/MCTS): {}",
                summarize(&portfolio_results)
            );

            // Prefer A*'s plan for the downstream full-domain/codegen/fingerprint steps if it
            // found one (matching the non-harder targets' behaviour); otherwise fall back to
            // whichever method actually succeeded, so a target A* can't solve still gets
            // checked all the way through instead of being silently skipped.
            astar_plan
                .or_else(|| ga_results.into_iter().flatten().next())
                .or_else(|| mcts_results.into_iter().flatten().next())
        } else {
            astar_plan
        };

        let Some(plan) = plan else {
            println!("  no method found a chain within budget — see note below\n");
            continue;
        };

        let mut wrong = 0u32;
        for x in 0..=u16::MAX {
            let mut v = x;
            for name in &plan.steps {
                let op = ops.iter().find(|o| &o.name == name).unwrap();
                v = op.apply(v);
            }
            if v != (t.oracle)(x) {
                wrong += 1;
            }
        }
        if wrong > 0 {
            println!(
                "  FULL-DOMAIN CHECK FAILED: {wrong}/65536 mismatches — matched the probes but \
                 not the real function\n"
            );
            continue;
        }
        println!("  full-domain check: PASS (all 65,536 inputs correct)");

        let body = codegen(&plan.steps, &op_meta);
        let src = format!("//! {}\n//! tags: {}\n{body}", t.summary, t.tags);

        let out_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("candidates");
        fs::create_dir_all(&out_dir)
            .unwrap_or_else(|e| panic!("creating {}: {e}", out_dir.display()));
        let out_path = out_dir.join(format!("{}.rs", t.name));
        fs::write(&out_path, &src)
            .unwrap_or_else(|e| panic!("writing {}: {e}", out_path.display()));
        let cart = compile(&format!("candidate_{}", t.name), &src);
        let fp = Fingerprint::of(&cart);
        let (best_agree, closest) = library_fps
            .iter()
            .map(|(name, other)| (fp.agreement(other), name.as_str()))
            .fold(
                (f32::MIN, ""),
                |best, cur| if cur.0 > best.0 { cur } else { best },
            );
        let is_dup = best_agree >= DUPLICATE_AGREEMENT;
        println!(
            "  fingerprint: closest existing cell = `{closest}` (agreement {best_agree:.3}) -> {}",
            if is_dup {
                "DUPLICATE — would be refused"
            } else {
                "NOVEL — would pass the fingerprint check"
            }
        );
        if !is_dup {
            passes += 1;
        }
        println!("  generated candidate source:\n{}\n", indent(&src));
    }

    println!(
        "=== {passes}/{} reachable targets passed (full-domain-correct AND non-duplicate) ===",
        targets.len() - 1 // is_semiprime is the negative control, not counted toward the bar
    );
}
