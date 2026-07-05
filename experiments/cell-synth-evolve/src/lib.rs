//! The reusable search methods from the cell-synth-evolve experiment (see `../README`-style
//! doc comment in `main.rs` and `../../cell-synth-evolve.md`): a from-scratch genetic search,
//! an MCTS (UCT), and two hybrids (portfolio, A*-seeded GA) over chains of `cell80::synth::Op`.
//! Split into a lib so other experiments (`evolved-cells`) can reuse the actual code, not a
//! duplicated copy of it.
use cell80::{Op, Plan};

const POP_SIZE: usize = 150;
const ELITE_FRAC: usize = 5; // top 1/POP_SIZE*ELITE_FRAC survive unmutated each generation
const MUTATE_PCT: u64 = 70;
const IMMIGRANT_PCT: u64 = 10; // fraction of each new generation that's fresh random blood

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn chance(&mut self, pct: u64) -> bool {
        self.next_u64() % 100 < pct
    }
}

pub fn run_chain(ops: &[Op], chain: &[usize], mut v: u16) -> u16 {
    for &i in chain {
        v = ops[i].apply(v);
    }
    v
}

pub fn matched(ops: &[Op], chain: &[usize], examples: &[(u16, u16)]) -> usize {
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
pub fn evolve_from(
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

pub fn evolve(
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
pub fn harvest_seeds(
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
pub fn evolve_seeded(
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
pub fn portfolio(candidates: &[Option<Plan>]) -> Option<Plan> {
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
pub fn mcts(
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
pub fn summarize(results: &[Option<Plan>]) -> String {
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
