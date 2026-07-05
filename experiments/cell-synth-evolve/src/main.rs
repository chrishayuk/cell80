//! Experiment: does genetic search (mutation + selection over candidate cell chains) find
//! algorithms that `cell80::synth`'s A*/Hamming-heuristic synthesizer struggles with?
//!
//! `cell80::synth` is outcome-specified synthesis: given input->output examples, discover a
//! short chain of cells that reproduces every one, via A* guided by Hamming distance to the
//! target. Its own docs flag a known weak spot: "lossy" ops (AND/OR/mask/rotate), where
//! Hamming distance is a deceptive signal because a step can look like it's moving away from
//! the target and still be on the only path to it. This reuses `cell80::{Op, Plan}` directly
//! (the exact same op representation and "reproduce every example" acceptance criterion) and
//! runs a from-scratch genetic search — mutate/crossover/select over whole candidate chains,
//! using only "how many examples does this reproduce" as fitness, no distance heuristic at
//! all — against the same benchmarks, same op pool, same budget, to see where each wins.
use cell80::{synthesize, Cartridge, CartridgeOpts, CellConfig, Op, Plan};

const POP_SIZE: usize = 150;
const ELITE_FRAC: usize = 5; // top 1/POP_SIZE*ELITE_FRAC survive unmutated each generation
const MUTATE_PCT: u64 = 70;
const IMMIGRANT_PCT: u64 = 10; // fraction of each new generation that's fresh random blood

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    fn chance(&mut self, pct: u64) -> bool {
        self.next_u64() % 100 < pct
    }
}

fn run_chain(ops: &[Op], chain: &[usize], mut v: u16) -> u16 {
    for &i in chain {
        v = ops[i].apply(v);
    }
    v
}

fn matched(ops: &[Op], chain: &[usize], examples: &[(u16, u16)]) -> usize {
    examples
        .iter()
        .filter(|&&(x, y)| run_chain(ops, chain, x) == y)
        .count()
}

fn random_chain(rng: &mut Rng, num_ops: usize, max_depth: usize) -> Vec<usize> {
    let len = rng.below(max_depth + 1);
    (0..len).map(|_| rng.below(num_ops)).collect()
}

fn mutate(rng: &mut Rng, chain: &[usize], num_ops: usize, max_depth: usize) -> Vec<usize> {
    let mut c = chain.to_vec();
    match rng.below(4) {
        0 if !c.is_empty() => {
            let i = rng.below(c.len());
            c[i] = rng.below(num_ops);
        }
        1 if c.len() < max_depth => {
            let i = rng.below(c.len() + 1);
            c.insert(i, rng.below(num_ops));
        }
        2 if !c.is_empty() => {
            let i = rng.below(c.len());
            c.remove(i);
        }
        3 if c.len() >= 2 => {
            let i = rng.below(c.len());
            let j = rng.below(c.len());
            c.swap(i, j);
        }
        _ => {}
    }
    c
}

/// Single-point crossover, capped at `max_depth` — uncapped crossover lets children grow
/// past `max_depth` across generations (classic GP "bloat"; caught empirically: an early
/// version of this reported `avg_depth=46.8` on a `max_depth=8` run, which is only possible
/// if this bound is missing).
fn crossover(rng: &mut Rng, a: &[usize], b: &[usize], max_depth: usize) -> Vec<usize> {
    if a.is_empty() || b.is_empty() {
        return if rng.chance(50) {
            a.to_vec()
        } else {
            b.to_vec()
        };
    }
    let cut_a = rng.below(a.len() + 1).min(max_depth);
    let mut child = a[..cut_a].to_vec();
    let remaining = max_depth.saturating_sub(child.len());
    let cut_b = rng.below(b.len() + 1);
    let tail = &b[cut_b..];
    child.extend_from_slice(&tail[..tail.len().min(remaining)]);
    child
}

/// Mutation + selection over whole candidate chains: no distance heuristic, just "how many
/// examples does this chain reproduce" as fitness. `tested` counts every chain fitness was
/// computed for (population_size * generations), the closest equivalent to `Plan::tested`'s
/// "candidates expanded" — not directly comparable 1:1 to A* node expansions, but the same
/// spirit (search effort spent). `initial` seeds part of generation 0 (used by the A*-seeded
/// hybrid below); the rest of the population is filled randomly either way.
fn evolve_from(
    examples: &[(u16, u16)],
    ops: &[Op],
    max_depth: usize,
    budget: usize,
    seed: u64,
    initial: Vec<Vec<usize>>,
) -> Option<Plan> {
    let mut rng = Rng::new(seed);
    let need = examples.len();
    let mut pop: Vec<Vec<usize>> = initial;
    pop.truncate(POP_SIZE);
    while pop.len() < POP_SIZE {
        pop.push(random_chain(&mut rng, ops.len(), max_depth));
    }
    let mut tested = 0usize;

    loop {
        let mut scored: Vec<(usize, Vec<usize>)> = pop
            .into_iter()
            .map(|c| (matched(ops, &c, examples), c))
            .collect();
        tested += scored.len();
        scored.sort_by_key(|s| std::cmp::Reverse(s.0));
        if scored[0].0 == need {
            let chain = &scored[0].1;
            return Some(Plan {
                steps: chain.iter().map(|&i| ops[i].name.clone()).collect(),
                tested,
                depth: chain.len(),
            });
        }
        if tested >= budget {
            return None;
        }

        let elite: Vec<Vec<usize>> = scored
            .into_iter()
            .take(POP_SIZE / ELITE_FRAC)
            .map(|(_, c)| c)
            .collect();
        let mut next: Vec<Vec<usize>> = elite.clone();
        while next.len() < POP_SIZE {
            if rng.chance(IMMIGRANT_PCT) {
                next.push(random_chain(&mut rng, ops.len(), max_depth));
                continue;
            }
            let a = &elite[rng.below(elite.len())];
            let b = &elite[rng.below(elite.len())];
            let mut child = crossover(&mut rng, a, b, max_depth);
            if rng.chance(MUTATE_PCT) {
                child = mutate(&mut rng, &child, ops.len(), max_depth);
            }
            next.push(child);
        }
        pop = next;
    }
}

fn evolve(
    examples: &[(u16, u16)],
    ops: &[Op],
    max_depth: usize,
    budget: usize,
    seed: u64,
) -> Option<Plan> {
    evolve_from(examples, ops, max_depth, budget, seed, vec![])
}

fn hamming(a: &[u16], b: &[u16]) -> u32 {
    a.iter().zip(b).map(|(x, y)| (x ^ y).count_ones()).sum()
}

/// A small local best-first (greedy-Hamming) search, capped at `sub_budget` expansions, used
/// only to harvest promising *partial* chains to seed the GA's initial population — not a
/// competing synthesizer. `cell80::synth` itself doesn't expose its search frontier, only
/// `Option<Plan>`, so this is a from-scratch (much cheaper, non-exhaustive) stand-in purely
/// for the "what looked promising on the way" signal, not a fork of the real A*. Returns the
/// `top_k` distinct chains seen with the lowest Hamming distance to the targets, plus how many
/// expansions it actually spent (deducted from the caller's total budget for a fair count).
fn harvest_seeds(
    examples: &[(u16, u16)],
    ops: &[Op],
    max_depth: usize,
    sub_budget: usize,
    top_k: usize,
) -> (Vec<Vec<usize>>, usize) {
    use std::cmp::Reverse;
    use std::collections::{BinaryHeap, HashSet};

    let inputs: Vec<u16> = examples.iter().map(|&(i, _)| i).collect();
    let targets: Vec<u16> = examples.iter().map(|&(_, o)| o).collect();

    let mut heap: BinaryHeap<Reverse<(u32, u64, Vec<usize>)>> = BinaryHeap::new();
    let mut seen: HashSet<Vec<usize>> = HashSet::new();
    let mut best: Vec<(u32, Vec<usize>)> = Vec::new();
    let mut seq = 0u64;
    let mut tested = 0usize;

    seen.insert(vec![]);
    let start_state: Vec<u16> = inputs.clone();
    heap.push(Reverse((hamming(&start_state, &targets), seq, vec![])));

    while let Some(Reverse((dist, _, chain))) = heap.pop() {
        best.push((dist, chain.clone()));
        if tested >= sub_budget || chain.len() >= max_depth {
            continue;
        }
        tested += 1;
        let state: Vec<u16> = inputs.iter().map(|&x| run_chain(ops, &chain, x)).collect();
        for (i, op) in ops.iter().enumerate() {
            let mut c2 = chain.clone();
            c2.push(i);
            if seen.insert(c2.clone()) {
                let ns: Vec<u16> = state.iter().map(|&v| op.apply(v)).collect();
                seq += 1;
                heap.push(Reverse((hamming(&ns, &targets), seq, c2)));
            }
        }
    }
    best.sort_by_key(|(d, _)| *d);
    best.dedup_by(|a, b| a.1 == b.1);
    (
        best.into_iter().take(top_k).map(|(_, c)| c).collect(),
        tested,
    )
}

/// Hybrid #2: harvest promising partial chains with a small Hamming-guided search, then run
/// the GA seeded with them (plus random fill) instead of starting from scratch. `tested`
/// covers the whole hybrid (harvest + GA), so it's comparable to the other methods' totals.
fn evolve_seeded(
    examples: &[(u16, u16)],
    ops: &[Op],
    max_depth: usize,
    budget: usize,
    seed: u64,
) -> Option<Plan> {
    const SUB_BUDGET: usize = 2_000;
    const TOP_K: usize = 20;
    let (seeds, harvest_tested) = harvest_seeds(examples, ops, max_depth, SUB_BUDGET, TOP_K);
    let remaining = budget.saturating_sub(harvest_tested);
    match evolve_from(examples, ops, max_depth, remaining, seed, seeds) {
        Some(p) => Some(Plan {
            tested: p.tested + harvest_tested,
            ..p
        }),
        None => None,
    }
}

/// Hybrid #1: the "just run all of them" portfolio — given each method's result for the same
/// problem, report whichever succeeded using the least effort (the outcome of running all
/// three in parallel and taking the first to finish, modeled here as picking the minimum
/// `tested` among the ones that actually found a solution).
fn portfolio(candidates: &[Option<Plan>]) -> Option<Plan> {
    candidates
        .iter()
        .flatten()
        .min_by_key(|p| p.tested)
        .cloned()
}

struct MctsNode {
    chain: Vec<usize>,
    visits: u32,
    reward_sum: f64,
    children: Vec<Option<usize>>,
}

/// Monte Carlo Tree Search over candidate chains: tree-structured like A*, but — like the
/// GA — needs no distance heuristic at all. Each iteration selects down the tree via UCB1,
/// expands one new node, rolls out randomly to `max_depth` (or an early exact match), and
/// backpropagates the *fraction of examples matched* as the reward. Since the branching
/// factor (`ops.len()`) is small, every node's children are eagerly sized upfront rather than
/// tracking "untried moves" separately.
fn mcts(
    examples: &[(u16, u16)],
    ops: &[Op],
    max_depth: usize,
    budget: usize,
    seed: u64,
) -> Option<Plan> {
    const EXPLORE: f64 = 1.4; // ~sqrt(2), the standard UCB1 exploration constant
    let need = examples.len();
    let mut rng = Rng::new(seed);
    let mut arena: Vec<MctsNode> = vec![MctsNode {
        chain: vec![],
        visits: 0,
        reward_sum: 0.0,
        children: vec![None; ops.len()],
    }];

    if matched(ops, &[], examples) == need {
        return Some(Plan {
            steps: vec![],
            tested: 0,
            depth: 0,
        });
    }

    let mut tested = 0usize;
    while tested < budget {
        // Selection + expansion: descend via UCB1, expanding the first unexpanded child found.
        let mut path = vec![0usize];
        let mut node_id = 0usize;
        loop {
            if arena[node_id].chain.len() >= max_depth {
                break;
            }
            let unexpanded: Vec<usize> = (0..ops.len())
                .filter(|&i| arena[node_id].children[i].is_none())
                .collect();
            if !unexpanded.is_empty() {
                let op_i = unexpanded[rng.below(unexpanded.len())];
                let mut chain = arena[node_id].chain.clone();
                chain.push(op_i);
                let cid = arena.len();
                arena.push(MctsNode {
                    chain,
                    visits: 0,
                    reward_sum: 0.0,
                    children: vec![None; ops.len()],
                });
                arena[node_id].children[op_i] = Some(cid);
                path.push(cid);
                node_id = cid;
                break;
            }
            let parent_visits = arena[node_id].visits.max(1) as f64;
            let mut best = (0usize, f64::MIN);
            for i in 0..ops.len() {
                let cid = arena[node_id].children[i].unwrap();
                let v = arena[cid].visits.max(1) as f64;
                let score = arena[cid].reward_sum / v + EXPLORE * (parent_visits.ln() / v).sqrt();
                if score > best.1 {
                    best = (cid, score);
                }
            }
            node_id = best.0;
            path.push(node_id);
        }

        // Rollout from node_id: random ops until max_depth or an exact match.
        let mut rollout = arena[node_id].chain.clone();
        let mut best_matched = matched(ops, &rollout, examples);
        tested += 1;
        let mut solution = (best_matched == need).then(|| rollout.clone());
        while solution.is_none() && rollout.len() < max_depth && tested < budget {
            rollout.push(rng.below(ops.len()));
            let m = matched(ops, &rollout, examples);
            tested += 1;
            best_matched = best_matched.max(m);
            if m == need {
                solution = Some(rollout.clone());
            }
        }
        if let Some(chain) = solution {
            return Some(Plan {
                steps: chain.iter().map(|&i| ops[i].name.clone()).collect(),
                tested,
                depth: chain.len(),
            });
        }

        // Backpropagation.
        let reward = best_matched as f64 / need as f64;
        for &id in &path {
            arena[id].visits += 1;
            arena[id].reward_sum += reward;
        }
    }
    None
}

/// Shared summary for a multi-seed stochastic method's results: found-rate plus averages over
/// only the seeds that succeeded (matching how the GA column was already reported).
fn summarize(results: &[Option<Plan>]) -> String {
    let found = results.iter().filter(|r| r.is_some()).count();
    if found == 0 {
        return format!("0/{}    (exhausted budget)", results.len());
    }
    let tested_sum: usize = results.iter().flatten().map(|p| p.tested).sum();
    let depth_sum: usize = results.iter().flatten().map(|p| p.depth).sum();
    format!(
        "{found}/{}    {:>8}    {:>7.1}",
        results.len(),
        tested_sum / found,
        depth_sum as f64 / found as f64
    )
}

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
