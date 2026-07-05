//! Benchmark harness for the cell-synth-evolve experiment: does genetic search / MCTS find
//! algorithms that `cell80::synth`'s A*/Hamming-heuristic synthesizer struggles with? The
//! actual search methods (GA, MCTS, portfolio, A*-seeded hybrid) live in `lib.rs` now, so
//! `evolved-cells` can reuse the real code instead of a duplicated copy.
use cell80::{synthesize, Cartridge, CartridgeOpts, CellConfig, Op, Plan};
use cell_synth_evolve::{evolve, evolve_seeded, mcts, portfolio, summarize};

fn cell(id: &str, src: &str) -> Cartridge {
    Cartridge::compile(
        src,
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some(id.into()),
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("compiling {id}: {e}"))
}

struct Benchmark {
    name: &'static str,
    lossy: bool,
    truth: &'static [&'static str],
    probe_inputs: &'static [u16],
}

fn main() {
    let and_c = cell("mask_and", "fn run(a: u16, b: u16) -> u16 { a & b }");
    let or_c = cell("mask_or", "fn run(a: u16, b: u16) -> u16 { a | b }");
    let xor_c = cell("mask_xor", "fn run(a: u16, b: u16) -> u16 { a ^ b }");
    let add_c = cell(
        "add_sat",
        "fn run(a: u16, b: u16) -> u16 { let s = a.wrapping_add(b); if s < a { 65535u16 } else { s } }",
    );
    let sub_c = cell(
        "sub_sat",
        "fn run(a: u16, b: u16) -> u16 { if a >= b { a - b } else { 0u16 } }",
    );
    let rot2_c = cell(
        "rotate_left_2",
        "fn run(x: u16) -> u16 { (x << 2u16) | (x >> 14u16) }",
    );
    let rot4_c = cell(
        "rotate_left_4",
        "fn run(x: u16) -> u16 { (x << 4u16) | (x >> 12u16) }",
    );
    let rot6_c = cell(
        "rotate_left_6",
        "fn run(x: u16) -> u16 { (x << 6u16) | (x >> 10u16) }",
    );
    let rot8_c = cell(
        "rotate_left_8",
        "fn run(x: u16) -> u16 { (x << 8u16) | (x >> 8u16) }",
    );

    let ops = vec![
        Op::from_cell("and_00ff", &and_c, 0x00FF),
        Op::from_cell("and_0f0f", &and_c, 0x0F0F),
        Op::from_cell("and_5555", &and_c, 0x5555),
        Op::from_cell("or_ff00", &or_c, 0xFF00),
        Op::from_cell("or_00f0", &or_c, 0x00F0),
        Op::from_cell("or_a5a5", &or_c, 0xA5A5),
        Op::from_cell("xor_00ff", &xor_c, 0x00FF),
        Op::from_cell("xor_ff00", &xor_c, 0xFF00),
        Op::from_cell("xor_5a5a", &xor_c, 0x5A5A),
        Op::from_cell("add_5", &add_c, 5),
        Op::from_cell("add_100", &add_c, 100),
        Op::from_cell("add_37", &add_c, 37),
        Op::from_cell("sub_5", &sub_c, 5),
        Op::from_cell("sub_37", &sub_c, 37),
        Op::from_cell("rotate2", &rot2_c, 0),
        Op::from_cell("rotate4", &rot4_c, 0),
        Op::from_cell("rotate6", &rot6_c, 0),
        Op::from_cell("rotate8", &rot8_c, 0),
    ];
    let op_index: std::collections::HashMap<&str, usize> = ops
        .iter()
        .enumerate()
        .map(|(i, o)| (o.name.as_str(), i))
        .collect();

    let benchmarks: &[Benchmark] = &[
        Benchmark {
            name: "smooth-2step",
            lossy: false,
            truth: &["add_5", "add_100"],
            probe_inputs: &[3, 40, 100, 7],
        },
        Benchmark {
            name: "smooth-3step",
            lossy: false,
            truth: &["add_5", "sub_5", "add_100"],
            probe_inputs: &[3, 40, 100, 7, 900],
        },
        Benchmark {
            name: "lossy-2step-mask+rotate",
            lossy: true,
            truth: &["and_0f0f", "rotate4"],
            probe_inputs: &[3, 40, 100, 7, 0xABCD, 0x1234],
        },
        Benchmark {
            name: "lossy-3step-mask+xor+rotate",
            lossy: true,
            truth: &["and_00ff", "xor_ff00", "rotate8"],
            probe_inputs: &[3, 40, 100, 7, 0xABCD, 0x1234, 0xFFFF],
        },
        Benchmark {
            name: "lossy-4step-deep",
            lossy: true,
            truth: &["or_00f0", "and_0f0f", "rotate4", "xor_00ff"],
            probe_inputs: &[3, 40, 100, 7, 0xABCD, 0x1234, 0xFFFF, 0x8001],
        },
        Benchmark {
            name: "lossy-6step-deep",
            lossy: true,
            truth: &[
                "or_a5a5", "and_0f0f", "rotate2", "xor_5a5a", "rotate6", "and_5555",
            ],
            probe_inputs: &[3, 40, 100, 7, 0xABCD, 0x1234, 0xFFFF, 0x8001, 0x00F0],
        },
    ];

    const MAX_DEPTH: usize = 8;
    const BUDGET: usize = 50_000;
    const GA_SEEDS: &[u64] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];

    println!(
        "{:<28} {:>6}  {:^22}  {:^28}  {:^28}",
        "benchmark",
        "truth",
        "A* (Hamming heuristic)",
        "GA (mutation + selection)",
        "MCTS (UCT, no heuristic)"
    );
    println!(
        "{:<28} {:>6}  {:<22}  {:<28}  {:<28}",
        "",
        "depth",
        "found tested depth",
        "found/N avg_tested avg_depth",
        "found/N avg_tested avg_depth"
    );

    struct Row<'a> {
        b: &'a Benchmark,
        astar: Option<Plan>,
        ga: Vec<Option<Plan>>,
        mcts: Vec<Option<Plan>>,
        seeded: Vec<Option<Plan>>,
    }
    let mut rows: Vec<Row> = Vec::new();

    for b in benchmarks {
        let examples: Vec<(u16, u16)> = b
            .probe_inputs
            .iter()
            .map(|&x| {
                let y = b
                    .truth
                    .iter()
                    .fold(x, |v, name| ops[op_index[name]].apply(v));
                (x, y)
            })
            .collect();

        let astar = synthesize(&examples, &ops, MAX_DEPTH, BUDGET);
        let astar_str = match &astar {
            Some(p) => format!("yes    {:>6} {:>5}", p.tested, p.depth),
            None => "no     (exhausted budget)".to_string(),
        };

        let ga_results: Vec<Option<Plan>> = GA_SEEDS
            .iter()
            .map(|&seed| evolve(&examples, &ops, MAX_DEPTH, BUDGET, seed))
            .collect();
        let mcts_results: Vec<Option<Plan>> = GA_SEEDS
            .iter()
            .map(|&seed| mcts(&examples, &ops, MAX_DEPTH, BUDGET, seed))
            .collect();
        let seeded_results: Vec<Option<Plan>> = GA_SEEDS
            .iter()
            .map(|&seed| evolve_seeded(&examples, &ops, MAX_DEPTH, BUDGET, seed))
            .collect();

        println!(
            "{:<28} {:>6}  {:<22}  {:<28}  {:<28}  {}",
            b.name,
            b.truth.len(),
            astar_str,
            summarize(&ga_results),
            summarize(&mcts_results),
            if b.lossy { "[lossy]" } else { "" }
        );

        rows.push(Row {
            b,
            astar,
            ga: ga_results,
            mcts: mcts_results,
            seeded: seeded_results,
        });
    }

    println!();
    println!(
        "Hybrids (same benchmarks, same budget, same {} seeds):",
        GA_SEEDS.len()
    );
    println!(
        "{:<28} {:>6}  {:<28}  {:<28}  {:<28}",
        "benchmark",
        "truth",
        "Portfolio (best of A*/GA/MCTS)",
        "GA seeded from A*-harvest",
        "plain GA (for reference)"
    );
    for row in &rows {
        let portfolio_results: Vec<Option<Plan>> = (0..GA_SEEDS.len())
            .map(|i| portfolio(&[row.astar.clone(), row.ga[i].clone(), row.mcts[i].clone()]))
            .collect();
        println!(
            "{:<28} {:>6}  {:<28}  {:<28}  {:<28}  {}",
            row.b.name,
            row.b.truth.len(),
            summarize(&portfolio_results),
            summarize(&row.seeded),
            summarize(&row.ga),
            if row.b.lossy { "[lossy]" } else { "" }
        );
    }
}
