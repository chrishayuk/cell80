//! Maps where A* actually stops finding `mystery_bits_2` (and, for contrast, the easier
//! `mystery_bits`), instead of the single data point the main `evolved-cells` run found (fails
//! at pool=34/depth=8, succeeds at pool=23/depth=6). Two sweeps: pool size at fixed depth, and
//! depth at fixed pool size — `build_ops`'s pool ordering keeps `mystery_bits_2` solvable at
//! every size in `[MIN_POOL, MAX_POOL]`, so a found->not-found transition here is genuinely
//! about search difficulty, not the target becoming unreachable.
use cell80::synthesize;
use cell_synth_evolve::{evolve, mcts};
use evolved_cells::{build_ops, mystery_bits_2_ref, mystery_bits_ref, MAX_POOL, MIN_POOL};
use std::path::Path;

const BUDGET: usize = 500_000;
const PROBES: &[u16] = &[
    0, 1, 4, 6, 9, 10, 99, 255, 256, 0x0F0F, 0xAAAA, 0x5555, 0xFF00, 0x00FF, 9999, 39999, 59999,
    65535,
];
const GA_MCTS_SEED: u64 = 1; // one seed per grid point — this maps the boundary's shape, not seed variance (already characterized in the main run)

struct Point {
    astar: Option<usize>, // Some(nodes tested) if found, None if not
    ga: Option<usize>,
    mcts: Option<usize>,
}

fn run_point(
    cells_dir: &Path,
    oracle: fn(u16) -> u16,
    pool_size: usize,
    max_depth: usize,
) -> Point {
    let (ops, _) = build_ops(cells_dir, pool_size);
    let examples: Vec<(u16, u16)> = PROBES.iter().map(|&x| (x, oracle(x))).collect();
    Point {
        astar: synthesize(&examples, &ops, max_depth, BUDGET).map(|p| p.tested),
        ga: evolve(&examples, &ops, max_depth, BUDGET, GA_MCTS_SEED).map(|p| p.tested),
        mcts: mcts(&examples, &ops, max_depth, BUDGET, GA_MCTS_SEED).map(|p| p.tested),
    }
}

fn cell(p: &Option<usize>) -> String {
    match p {
        Some(n) => format!("{n}"),
        None => "—".to_string(),
    }
}

fn main() {
    let cells_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cell80/cells");

    for (target_name, oracle) in [
        ("mystery_bits", mystery_bits_ref as fn(u16) -> u16),
        ("mystery_bits_2", mystery_bits_2_ref),
    ] {
        println!("=== {target_name}: pool-size sweep (max_depth=8, budget={BUDGET}) ===");
        println!(
            "{:<10} {:>12} {:>12} {:>12}",
            "pool", "A* tested", "GA tested", "MCTS tested"
        );
        for pool_size in [MIN_POOL, 22, 26, 30, MAX_POOL] {
            let pt = run_point(&cells_dir, oracle, pool_size, 8);
            println!(
                "{:<10} {:>12} {:>12} {:>12}",
                pool_size,
                cell(&pt.astar),
                cell(&pt.ga),
                cell(&pt.mcts)
            );
        }
        println!();

        println!("=== {target_name}: depth sweep (pool={MAX_POOL}, budget={BUDGET}) ===");
        println!(
            "{:<10} {:>12} {:>12} {:>12}",
            "depth", "A* tested", "GA tested", "MCTS tested"
        );
        for max_depth in [5, 6, 7, 8, 9, 10] {
            let pt = run_point(&cells_dir, oracle, MAX_POOL, max_depth);
            println!(
                "{:<10} {:>12} {:>12} {:>12}",
                max_depth,
                cell(&pt.astar),
                cell(&pt.ga),
                cell(&pt.mcts)
            );
        }
        println!();
    }
    println!("'—' = no chain found within budget. One seed per grid point (mapping the boundary's shape, not seed variance — already characterized for both targets in the main evolved-cells run).");
}
