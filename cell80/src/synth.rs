//! **Outcome-specified synthesis (experimental)** — the *inverse* of [`CellGraph`](crate::CellGraph).
//!
//! `CellGraph` is graph → output: you author a chain of cells and run it. Synthesis is
//! output → graph: you give input→output **examples** and it *discovers* a short chain of
//! cells that reproduces them. The verifier is the engine in both directions — here, applying
//! an [`Op`] *is* executing a cell, and a candidate chain is accepted only when it reproduces
//! **every** example exactly.
//!
//! This is a deliberately **different mode** from normal tool-calling (`search → inspect →
//! run`). It earns its keep only on the un-foldable regime the rest of cell80 doesn't have:
//! **outcome-specified** (the steps aren't described — they must be found) over **lossy ops**
//! (AND/OR/mask/rotate, where distance-greedy is deceptive so real backtracking search is
//! needed). On smooth/metric ops, or when the prompt already names the steps, composition
//! folds and you don't want this.
//!
//! **Heuristic-first, learned-second (gated).** [`synthesize`] uses a hand heuristic (Hamming
//! distance to the targets) over A*. A learned value heuristic can be plugged via
//! [`synthesize_with`], but it is **not assumed** — whether it beats the hand heuristic at
//! equal budget is a kill gate, not a given (see `examples/composition_eval.rs`, where in a
//! clean-room build the learned value net only *tied* the Hamming heuristic).
use std::collections::{BinaryHeap, HashSet};

use crate::{Cartridge, Runner, DEFAULT_CYCLES};

/// One synthesis op: a cell applied to the running value with a fixed second argument. Built
/// from a real cell, so applying it *is* executing the cell (the verifier is the engine). The
/// per-value results are precomputed once over the u16 domain for fast search.
pub struct Op {
    pub name: String,
    table: Vec<u16>,
}

impl Op {
    /// Build an op from a compiled cell and a fixed second argument (ignored by 1-arg cells).
    pub fn from_cell(name: &str, cart: &Cartridge, arg: u16) -> Self {
        let mut r = Runner::new(cart.z80().expect("synth composes z80-cell bodies"));
        let entry = cart.manifest.entry.clone();
        let table = (0..=u16::MAX)
            .map(|v| {
                r.run_fast(Some(&entry), &[v, arg], DEFAULT_CYCLES)
                    .map(|f| f.result)
                    .unwrap_or(v)
            })
            .collect();
        Op {
            name: name.into(),
            table,
        }
    }

    /// Apply the op to a value (executes the precomputed cell transition).
    pub fn apply(&self, v: u16) -> u16 {
        self.table[v as usize]
    }
}

/// A discovered chain: the op names in order, plus search stats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Op names to apply, in order (the synthesized `CellGraph` chain).
    pub steps: Vec<String>,
    /// Candidates expanded before the solution was found.
    pub tested: usize,
    /// Chain length.
    pub depth: usize,
}

/// Total Hamming distance from the current values to the targets — the hand heuristic.
fn hamming(state: &[u16], targets: &[u16]) -> i64 {
    state
        .iter()
        .zip(targets)
        .map(|(s, t)| (s ^ t).count_ones() as i64)
        .sum()
}

/// Synthesize a chain (≤ `max_depth` ops) mapping every example input to its output, via A*
/// over the verifier ordered by the **hand** Hamming heuristic. `budget` caps node expansions.
/// Returns the first chain found, or `None` if none exists within the budget/depth.
pub fn synthesize(
    examples: &[(u16, u16)],
    ops: &[Op],
    max_depth: usize,
    budget: usize,
) -> Option<Plan> {
    synthesize_with(examples, ops, max_depth, budget, &hamming)
}

/// Like [`synthesize`] but with a pluggable heuristic `h(state, targets) -> cost` (lower =
/// closer) — the seam where a **learned** value net rides in *as a gated falsifier*, never an
/// assumption: it must beat the hand heuristic at equal budget to earn its place.
pub fn synthesize_with(
    examples: &[(u16, u16)],
    ops: &[Op],
    max_depth: usize,
    budget: usize,
    h: &dyn Fn(&[u16], &[u16]) -> i64,
) -> Option<Plan> {
    let inputs: Vec<u16> = examples.iter().map(|&(i, _)| i).collect();
    let targets: Vec<u16> = examples.iter().map(|&(_, o)| o).collect();
    if inputs == targets {
        return Some(Plan {
            steps: vec![],
            tested: 0,
            depth: 0,
        });
    }

    // Max-heap on negated A* cost (g + h); a unique sequence number breaks ties cheaply (so
    // the state/path vectors are never compared).
    let mut heap: BinaryHeap<(i64, u64, Vec<u16>, Vec<usize>)> = BinaryHeap::new();
    let mut seen: HashSet<Vec<u16>> = HashSet::new();
    let mut seq: u64 = 0;
    seen.insert(inputs.clone());
    heap.push((-h(&inputs, &targets), seq, inputs, vec![]));
    let mut tested = 0usize;

    while let Some((_, _, state, path)) = heap.pop() {
        if state == targets {
            return Some(Plan {
                steps: path.iter().map(|&i| ops[i].name.clone()).collect(),
                tested,
                depth: path.len(),
            });
        }
        if tested >= budget || path.len() >= max_depth {
            continue; // don't expand, but keep checking already-generated goal nodes
        }
        tested += 1;
        for (i, op) in ops.iter().enumerate() {
            let ns: Vec<u16> = state.iter().map(|&v| op.apply(v)).collect();
            if seen.insert(ns.clone()) {
                let mut np = path.clone();
                np.push(i);
                let cost = np.len() as i64 + h(&ns, &targets); // g + h
                seq += 1;
                heap.push((-cost, seq, ns, np));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CartridgeOpts, CellConfig};

    fn cell(id: &str, src: &str) -> Cartridge {
        Cartridge::compile(
            src,
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some(id.into()),
                ..Default::default()
            },
        )
        .unwrap()
    }

    fn ops() -> Vec<Op> {
        let xor = cell("mask_xor", "fn run(a: u16, b: u16) -> u16 { a ^ b }");
        let add = cell("add_sat", "fn run(a: u16, b: u16) -> u16 { let s = a.wrapping_add(b); let mut r = s; if s < a { r = 65535u16; } r }");
        let swap = cell(
            "swap_bytes",
            "fn run(x: u16) -> u16 { (x << 8u16) | (x >> 8u16) }",
        );
        vec![
            Op::from_cell("xor_00ff", &xor, 0x00FF),
            Op::from_cell("xor_ff00", &xor, 0xFF00),
            Op::from_cell("add_5", &add, 5),
            Op::from_cell("swap", &swap, 0),
        ]
    }

    /// Apply a named chain to a value (independent re-derivation for assertions).
    fn run_chain(ops: &[Op], steps: &[String], mut v: u16) -> u16 {
        for s in steps {
            let op = ops.iter().find(|o| &o.name == s).unwrap();
            v = op.apply(v);
        }
        v
    }

    #[test]
    fn recovers_a_chain_that_satisfies_the_examples() {
        let ops = ops();
        // Hidden target program: add_5 then swap. Generate examples from it.
        let truth = ["add_5".to_string(), "swap".to_string()];
        let examples: Vec<(u16, u16)> = [3u16, 40, 100, 7]
            .iter()
            .map(|&x| (x, run_chain(&ops, &truth, x)))
            .collect();

        let plan = synthesize(&examples, &ops, 4, 50_000).expect("a chain exists");
        // Not necessarily the same chain — but it MUST reproduce every example (verifier).
        for &(x, y) in &examples {
            assert_eq!(
                run_chain(&ops, &plan.steps, x),
                y,
                "synthesized chain must satisfy the spec"
            );
        }
        assert!(plan.depth <= 4);
    }

    #[test]
    fn identity_spec_is_the_empty_chain() {
        let ops = ops();
        let examples = vec![(5u16, 5u16), (9, 9)];
        assert_eq!(
            synthesize(&examples, &ops, 4, 1000).unwrap().steps,
            Vec::<String>::new()
        );
    }

    #[test]
    fn unsolvable_within_budget_returns_none() {
        let ops = ops();
        // No chain of these ops maps both 0->1 and 0->2 (a single input can't reach two outputs).
        let examples = vec![(0u16, 1u16), (0u16, 2u16)];
        assert!(synthesize(&examples, &ops, 4, 5000).is_none());
    }

    #[test]
    fn budget_is_enforced() {
        let ops = ops();
        let examples = vec![(3u16, run_chain(&ops, &["add_5".to_string()], 3))]; // 1-step solvable
        assert!(
            synthesize(&examples, &ops, 4, 0).is_none(),
            "budget 0 can't expand a node"
        );
        let plan = synthesize(&examples, &ops, 4, 100).expect("enough budget");
        assert!(plan.tested <= 100, "never expands past the budget");
    }

    #[test]
    fn pluggable_heuristic_still_solves() {
        let ops = ops();
        let truth = ["add_5".to_string(), "swap".to_string()];
        let examples: Vec<(u16, u16)> = [10u16, 20]
            .iter()
            .map(|&x| (x, run_chain(&ops, &truth, x)))
            .collect();
        // A trivial heuristic (constant 0) degrades A* to uniform-cost search; must still solve
        // (this is the seam a learned value net plugs into — gated, never assumed).
        let plan = synthesize_with(&examples, &ops, 4, 200_000, &|_, _| 0).expect("found");
        for &(x, y) in &examples {
            assert_eq!(run_chain(&ops, &plan.steps, x), y);
        }
    }
}
