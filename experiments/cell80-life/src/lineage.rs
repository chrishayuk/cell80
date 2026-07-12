//! Instrumentation over EX-1/2/3 runs (`experiments/deterministic-ecology.md`'s EX-4):
//! content-addressed genome hashing, a lineage tree built from a run's birth log, and
//! detection of "sustained plurality change" events for the three swappable roles — the
//! mechanical version of what `cell80-life-findings.md`'s Finding 3 did by hand (reading
//! the `argmin3` purge story off printed diversity stats sampled every 20 ticks). No new
//! world, no new tick engine: everything here is a pure function of data `ex2::run`/
//! `ex2::run_with_overrides` already produce.
use std::collections::{BTreeMap, HashMap, HashSet};

use sha2::{Digest, Sha256};

use crate::ex2::FieldOverride;
use crate::history::{
    BirthEvent, BirthEventEco, OrgSnapshot2DEco, OrgSnapshot2DGenome, Species, TickRecord2DEco,
    TickRecord2DGenome,
};

/// The 6 heritable fields every genome-carrying type in this codebase shares, in the same
/// order — hashed once here rather than duplicating the byte-layout logic per carrier type
/// (`BirthEvent`, `OrgSnapshot2DGenome`, `StartingGenome2` all carry these identically).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenomeFields {
    pub decay_amount: u16,
    pub repro_threshold: u16,
    pub repro_give_pct: u16,
    pub hungry_promoter: u16,
    pub repro_promoter: u16,
    pub sense_move: u16,
}

impl GenomeFields {
    pub fn from_birth(b: &BirthEvent) -> Self {
        GenomeFields {
            decay_amount: b.decay_amount,
            repro_threshold: b.repro_threshold,
            repro_give_pct: b.repro_give_pct,
            hungry_promoter: b.hungry_promoter,
            repro_promoter: b.repro_promoter,
            sense_move: b.sense_move,
        }
    }

    pub fn from_snapshot(o: &OrgSnapshot2DGenome) -> Self {
        GenomeFields {
            decay_amount: o.decay_amount,
            repro_threshold: o.repro_threshold,
            repro_give_pct: o.repro_give_pct,
            hungry_promoter: o.hungry_promoter,
            repro_promoter: o.repro_promoter,
            sense_move: o.sense_move,
        }
    }

    pub fn from_starting(s: &crate::ex2::StartingGenome2) -> Self {
        GenomeFields {
            decay_amount: s.decay_amount,
            repro_threshold: s.repro_threshold,
            repro_give_pct: s.repro_give_pct,
            hungry_promoter: s.hungry_promoter,
            repro_promoter: s.repro_promoter,
            sense_move: s.sense_move,
        }
    }

    /// EX-3's `BirthEventEco` carries the same 6 fields as `BirthEvent` plus `species`
    /// (ignored here — species is orthogonal to genome content, never part of the hash).
    pub fn from_birth_eco(b: &BirthEventEco) -> Self {
        GenomeFields {
            decay_amount: b.decay_amount,
            repro_threshold: b.repro_threshold,
            repro_give_pct: b.repro_give_pct,
            hungry_promoter: b.hungry_promoter,
            repro_promoter: b.repro_promoter,
            sense_move: b.sense_move,
        }
    }

    /// EX-3's `OrgSnapshot2DEco` counterpart to `from_snapshot`.
    pub fn from_snapshot_eco(o: &OrgSnapshot2DEco) -> Self {
        GenomeFields {
            decay_amount: o.decay_amount,
            repro_threshold: o.repro_threshold,
            repro_give_pct: o.repro_give_pct,
            hungry_promoter: o.hungry_promoter,
            repro_promoter: o.repro_promoter,
            sense_move: o.sense_move,
        }
    }

    /// Content-address this genome: fixed-width LE bytes into a fresh SHA-256, exactly
    /// `HistoryHasher`'s existing discipline. Use the full 32 bytes as a map key; truncate
    /// only for human-readable printing.
    pub fn hash(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(self.decay_amount.to_le_bytes());
        h.update(self.repro_threshold.to_le_bytes());
        h.update(self.repro_give_pct.to_le_bytes());
        h.update(self.hungry_promoter.to_le_bytes());
        h.update(self.repro_promoter.to_le_bytes());
        h.update(self.sense_move.to_le_bytes());
        h.finalize().into()
    }

    pub fn short_hash(&self) -> String {
        self.hash()[..8]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}

/// One of the three swappable roles EX-2 mutates by cell-swap. Numeric fields are
/// deliberately out of scope for this detector — `cell80-life-findings.md`'s own Finding 3
/// already frames them as continuous "stabilizing selection near a starting value," never a
/// discrete winner; a different mechanism, worth naming as future work, not folded in here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    Hungry,
    Repro,
    Sense,
}

impl Role {
    fn value(self, g: &GenomeFields) -> u16 {
        match self {
            Role::Hungry => g.hungry_promoter,
            Role::Repro => g.repro_promoter,
            Role::Sense => g.sense_move,
        }
    }

    /// The `FieldOverride` that forces this role's mutation to be skipped for one birth —
    /// EX-4's counterfactual mechanism, applied to exactly the one field under test.
    pub fn skip_override(self) -> FieldOverride {
        let mut o = FieldOverride::default();
        match self {
            Role::Hungry => o.skip_hungry_swap = true,
            Role::Repro => o.skip_repro_swap = true,
            Role::Sense => o.skip_sense_swap = true,
        }
        o
    }
}

/// One node in the lineage tree — a genesis organism (`parent_id: None`) or a birth.
#[derive(Clone, Debug)]
pub struct LineageNode {
    pub child_id: u32,
    pub parent_id: Option<u32>,
    pub tick: u32,
    pub genome: GenomeFields,
}

/// `hash(child genome) -> hash(parent genome)`, keyed on `(genome_hash, child_id)` rather
/// than `genome_hash` alone: two organisms can independently mutate to the exact same
/// 6-field genome at different times (real and expected given pool sizes), and merging on
/// hash alone would force an arbitrary, silent choice about which parent is "the" parent.
/// Keeping every birth (and every genesis organism) a distinct node makes convergent
/// origins a visible, queryable case instead of hiding one.
pub struct LineageTree {
    by_child_id: HashMap<u32, LineageNode>,
    by_hash: HashMap<[u8; 32], Vec<u32>>,
}

impl LineageTree {
    /// `initial_organisms` genesis ids, always `0..initial_organisms` — correct for a
    /// single-species run (EX-1/EX-2) where genesis ids start at 0. A thin wrapper over
    /// `build_from_genesis_ids`.
    pub fn build(starting: &GenomeFields, initial_organisms: usize, births: &[BirthEvent]) -> Self {
        let genesis_ids: Vec<u32> = (0..initial_organisms as u32).collect();
        Self::build_from_genesis_ids(&genesis_ids, starting, births)
    }

    /// EX-3's counterpart to `build`: a second species' genesis ids don't start at 0 (they
    /// come after the first species' full initial population in `ex3.rs`'s shared,
    /// monotonic id counter), so the caller supplies the exact genesis id set instead of a
    /// count.
    pub fn build_from_genesis_ids(
        genesis_ids: &[u32],
        starting: &GenomeFields,
        births: &[BirthEvent],
    ) -> Self {
        let mut by_child_id = HashMap::new();
        let mut by_hash: HashMap<[u8; 32], Vec<u32>> = HashMap::new();

        for &id in genesis_ids {
            by_hash.entry(starting.hash()).or_default().push(id);
            by_child_id.insert(
                id,
                LineageNode {
                    child_id: id,
                    parent_id: None,
                    tick: 0,
                    genome: *starting,
                },
            );
        }
        for b in births {
            let genome = GenomeFields::from_birth(b);
            by_hash.entry(genome.hash()).or_default().push(b.child_id);
            by_child_id.insert(
                b.child_id,
                LineageNode {
                    child_id: b.child_id,
                    parent_id: Some(b.parent_id),
                    tick: b.tick,
                    genome,
                },
            );
        }
        LineageTree {
            by_child_id,
            by_hash,
        }
    }

    pub fn get(&self, child_id: u32) -> Option<&LineageNode> {
        self.by_child_id.get(&child_id)
    }

    pub fn with_genome(&self, hash: &[u8; 32]) -> &[u32] {
        self.by_hash.get(hash).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

/// EX-3's species-filtering adapter: reduce one species' view of a two-species run down to
/// the exact shapes `detect_plurality_events`/`find_origins`/`LineageTree::build_from_genesis_ids`
/// already consume, so those functions run **unmodified** on a second species — no `_eco`
/// variant of any of them. `food`/`births`/`starved`/`contention_losses`/`total_ir_steps`
/// aren't reproduced (they're run-global counts, not species-partitioned, and none of the
/// three functions above read them — see `plurality_at`/`find_origins`, which only touch
/// `tick` and `organisms`); this mirrors the test helpers' own `tick_rec(..)` convention of
/// leaving them at their zero value when irrelevant to the analysis.
pub fn eco_ticks_to_genome(ticks: &[TickRecord2DEco], species: Species) -> Vec<TickRecord2DGenome> {
    ticks
        .iter()
        .map(|rec| TickRecord2DGenome {
            tick: rec.tick,
            organisms: rec
                .organisms
                .iter()
                .filter(|o| o.species == species)
                .map(|o| OrgSnapshot2DGenome {
                    id: o.id,
                    x: o.x,
                    y: o.y,
                    energy: o.energy,
                    decay_amount: o.decay_amount,
                    repro_threshold: o.repro_threshold,
                    repro_give_pct: o.repro_give_pct,
                    hungry_promoter: o.hungry_promoter,
                    repro_promoter: o.repro_promoter,
                    sense_move: o.sense_move,
                })
                .collect(),
            food: Vec::new(),
            births: 0,
            starved: 0,
            contention_losses: 0,
            total_ir_steps: 0,
        })
        .collect()
}

/// `BirthEventEco` -> `BirthEvent`, filtered to one species — the birth-log counterpart to
/// `eco_ticks_to_genome`, feeding `LineageTree::build_from_genesis_ids` unmodified.
pub fn eco_births_to_genome(births: &[BirthEventEco], species: Species) -> Vec<BirthEvent> {
    births
        .iter()
        .filter(|b| b.species == species)
        .map(|b| BirthEvent {
            child_id: b.child_id,
            parent_id: b.parent_id,
            tick: b.tick,
            decay_amount: b.decay_amount,
            repro_threshold: b.repro_threshold,
            repro_give_pct: b.repro_give_pct,
            hungry_promoter: b.hungry_promoter,
            repro_promoter: b.repro_promoter,
            sense_move: b.sense_move,
        })
        .collect()
}

/// One sampled tick's plurality winner for a role, among the living population.
#[derive(Clone, Copy, Debug)]
struct Sample {
    tick: u32,
    winner: u16,
    share: f64,
}

/// The plurality (most common) value for `role` among a tick's living organisms —
/// `BTreeMap` for ascending-key iteration, tie-break = lowest index wins (strict `>`, never
/// `>=`), avoiding the `HashMap`-tie-break risk `history.rs`'s own doc comment already
/// flags elsewhere in this codebase.
fn plurality_at(rec: &TickRecord2DGenome, role: Role) -> Option<Sample> {
    if rec.organisms.is_empty() {
        return None;
    }
    let mut counts: BTreeMap<u16, u32> = BTreeMap::new();
    for o in &rec.organisms {
        let g = GenomeFields::from_snapshot(o);
        *counts.entry(role.value(&g)).or_insert(0) += 1;
    }
    let total = rec.organisms.len() as u32;
    let mut best: Option<(u16, u32)> = None;
    for (&idx, &count) in &counts {
        let is_better = match best {
            None => true,
            Some((_, best_count)) => count > best_count,
        };
        if is_better {
            best = Some((idx, count));
        }
    }
    best.map(|(winner, count)| Sample {
        tick: rec.tick,
        winner,
        share: count as f64 / total as f64,
    })
}

/// A detected, sustained change in which pool member is the population's plurality choice
/// for `role`. Deliberately named a "plurality shift," not "fixation" — see
/// `detect_plurality_events`'s doc comment for why claiming population-genetics fixation
/// would overclaim what this codebase's population scale can actually show.
#[derive(Clone, Debug, PartialEq)]
pub struct PluralityEvent {
    pub role: Role,
    pub from: u16,
    pub to: u16,
    pub shift_tick: u32,
    pub share_at_shift: f64,
    pub peak_share_in_window: f64,
}

/// Detect every "sustained plurality change" for `role`: the plurality winner changes
/// between two consecutive samples (`sample_every`-tick cadence) and the new winner remains
/// the plurality winner for at least `sustain_k` subsequent samples. A transient blip (the
/// winner reverting before `sustain_k` samples elapse) is not reported — that's ordinary
/// birth/death churn, not a real event.
///
/// This reports a *plurality* shift, not population-genetics fixation (~100%) or even an
/// outright majority (>50%): EX-2's own receipts show dispatch-count-per-role climbing to
/// 7-30 distinct values in active use at n~150-250, so single-value dominance is
/// implausible at this scale within a few thousand ticks. `share_at_shift`/
/// `peak_share_in_window` are recorded precisely so a report can state the real share
/// honestly rather than borrow the word "fixation" for something smaller.
pub fn detect_plurality_events(
    ticks: &[TickRecord2DGenome],
    role: Role,
    sample_every: u32,
    sustain_k: usize,
) -> Vec<PluralityEvent> {
    let samples: Vec<Sample> = ticks
        .iter()
        .filter(|r| r.tick % sample_every == 0)
        .filter_map(|r| plurality_at(r, role))
        .collect();

    let mut events = Vec::new();
    for i in 1..samples.len() {
        if samples[i].winner == samples[i - 1].winner {
            continue;
        }
        let to = samples[i].winner;
        if samples.len() - i >= sustain_k
            && samples[i..i + sustain_k].iter().all(|s| s.winner == to)
        {
            let peak = samples[i..i + sustain_k]
                .iter()
                .map(|s| s.share)
                .fold(0.0_f64, f64::max);
            events.push(PluralityEvent {
                role,
                from: samples[i - 1].winner,
                to,
                shift_tick: samples[i].tick,
                share_at_shift: samples[i].share,
                peak_share_in_window: peak,
            });
        }
    }
    events
}

/// Where a winning genome value traces back to.
#[derive(Clone, Debug, PartialEq)]
pub enum OriginKind {
    /// A genuine mutation-in birth: the child's role value differs from its immediate
    /// parent's — the causally-responsible event.
    Mutated {
        origin_child_id: u32,
        origin_parent_id: u32,
        origin_tick: u32,
        parent_genome: GenomeFields,
        child_genome: GenomeFields,
    },
    /// The ancestry walk reached a genesis organism without ever finding a transition — the
    /// winning value was the *starting* genome's value all along. This event is a
    /// reversion to baseline after a transient displacement by some other variant, not a
    /// novel fixation, and must be labeled distinctly rather than silently reported as "no
    /// origin found."
    Genesis { genesis_id: u32 },
}

/// Walk `child_id`'s ancestry upward until the nearest ancestor where `role` flipped INTO
/// its current value (a genuine mutation-in, not an inherited copy) — or a genesis
/// organism, if the value traces all the way back to the run's starting genome.
fn trace_origin(tree: &LineageTree, role: Role, child_id: u32) -> OriginKind {
    let mut current = child_id;
    loop {
        let node = tree
            .get(current)
            .expect("lineage tree missing a live organism's node");
        match node.parent_id {
            None => {
                return OriginKind::Genesis {
                    genesis_id: current,
                }
            }
            Some(parent_id) => {
                let parent = tree
                    .get(parent_id)
                    .expect("lineage tree missing a parent node");
                if role.value(&parent.genome) != role.value(&node.genome) {
                    return OriginKind::Mutated {
                        origin_child_id: node.child_id,
                        origin_parent_id: parent_id,
                        origin_tick: node.tick,
                        parent_genome: parent.genome,
                        child_genome: node.genome,
                    };
                }
                current = parent_id;
            }
        }
    }
}

/// Every distinct origin (deduplicated by `child_id`/`genesis_id`, not genome hash —
/// convergent origins may share a hash but remain separately meaningful events) among the
/// organisms actually carrying `event.to` for `event.role` at the event's shift tick. A
/// backward walk from the *living* holders, not a forward scan of the whole birth log — a
/// forward scan would surface historical mutations whose lineage has since died out
/// entirely, misreporting an extinct branch as "the cause."
pub fn find_origins(
    tree: &LineageTree,
    ticks: &[TickRecord2DGenome],
    event: &PluralityEvent,
) -> Vec<OriginKind> {
    let Some(rec) = ticks.iter().find(|r| r.tick == event.shift_tick) else {
        return Vec::new();
    };
    let mut seen: HashSet<u32> = HashSet::new();
    let mut origins = Vec::new();
    for o in &rec.organisms {
        let g = GenomeFields::from_snapshot(o);
        if event.role.value(&g) != event.to {
            continue;
        }
        let origin = trace_origin(tree, event.role, o.id);
        let key = match &origin {
            OriginKind::Mutated {
                origin_child_id, ..
            } => *origin_child_id,
            OriginKind::Genesis { genesis_id } => *genesis_id,
        };
        if seen.insert(key) {
            origins.push(origin);
        }
    }
    origins
}

#[cfg(test)]
mod tests {
    use super::*;

    fn genome(hungry: u16) -> GenomeFields {
        // Only `hungry_promoter` varies across these tests; the other 5 fields are fixed
        // constants so the hash still uniquely reflects the field under test.
        GenomeFields {
            decay_amount: 1,
            repro_threshold: 200,
            repro_give_pct: 50,
            hungry_promoter: hungry,
            repro_promoter: 0,
            sense_move: 0,
        }
    }

    fn birth(child_id: u32, parent_id: u32, tick: u32, hungry: u16) -> BirthEvent {
        let g = genome(hungry);
        BirthEvent {
            child_id,
            parent_id,
            tick,
            decay_amount: g.decay_amount,
            repro_threshold: g.repro_threshold,
            repro_give_pct: g.repro_give_pct,
            hungry_promoter: g.hungry_promoter,
            repro_promoter: g.repro_promoter,
            sense_move: g.sense_move,
        }
    }

    fn org(id: u32, hungry: u16) -> OrgSnapshot2DGenome {
        let g = genome(hungry);
        OrgSnapshot2DGenome {
            id,
            x: 0,
            y: 0,
            energy: 100,
            decay_amount: g.decay_amount,
            repro_threshold: g.repro_threshold,
            repro_give_pct: g.repro_give_pct,
            hungry_promoter: g.hungry_promoter,
            repro_promoter: g.repro_promoter,
            sense_move: g.sense_move,
        }
    }

    fn tick_rec(tick: u32, orgs: Vec<OrgSnapshot2DGenome>) -> TickRecord2DGenome {
        TickRecord2DGenome {
            tick,
            organisms: orgs,
            food: Vec::new(),
            births: 0,
            starved: 0,
            contention_losses: 0,
            total_ir_steps: 0,
        }
    }

    #[test]
    fn genesis_traces_to_genesis_organism() {
        let starting = genome(0);
        let tree = LineageTree::build(&starting, 2, &[]);
        assert_eq!(
            trace_origin(&tree, Role::Hungry, 0),
            OriginKind::Genesis { genesis_id: 0 }
        );
        assert_eq!(
            trace_origin(&tree, Role::Hungry, 1),
            OriginKind::Genesis { genesis_id: 1 }
        );
    }

    #[test]
    fn single_origin_mutation_is_found_even_through_a_descendant() {
        let starting = genome(0);
        let births = vec![
            birth(2, 0, 10, 1), // genuine mutation: parent(0)=0 -> child(2)=1
            birth(3, 2, 20, 1), // inherited, unchanged: parent(2)=1 -> child(3)=1
        ];
        let tree = LineageTree::build(&starting, 2, &births);

        // Tracing from the direct mutant and from its unchanged descendant both find the
        // same origin — the walk doesn't stop at the first ancestor, it finds the *nearest*
        // genuine transition.
        let want = OriginKind::Mutated {
            origin_child_id: 2,
            origin_parent_id: 0,
            origin_tick: 10,
            parent_genome: genome(0),
            child_genome: genome(1),
        };
        assert_eq!(trace_origin(&tree, Role::Hungry, 2), want);
        assert_eq!(trace_origin(&tree, Role::Hungry, 3), want);
    }

    #[test]
    fn convergent_origins_are_both_reported_not_merged() {
        let starting = genome(0);
        // Two independent mutations to the same value (1) from two different genesis
        // parents.
        let births = vec![birth(2, 0, 10, 1), birth(3, 1, 15, 1)];
        let tree = LineageTree::build(&starting, 2, &births);

        let ticks = vec![tick_rec(15, vec![org(2, 1), org(3, 1)])];
        let event = PluralityEvent {
            role: Role::Hungry,
            from: 0,
            to: 1,
            shift_tick: 15,
            share_at_shift: 1.0,
            peak_share_in_window: 1.0,
        };
        let origins = find_origins(&tree, &ticks, &event);
        assert_eq!(
            origins.len(),
            2,
            "expected two distinct convergent origins, got {origins:?}"
        );
        let ids: HashSet<u32> = origins
            .iter()
            .map(|o| match o {
                OriginKind::Mutated {
                    origin_child_id, ..
                } => *origin_child_id,
                OriginKind::Genesis { genesis_id } => *genesis_id,
            })
            .collect();
        assert_eq!(ids, HashSet::from([2, 3]));
    }

    #[test]
    fn plurality_tie_break_is_lowest_index_not_hash_order() {
        let rec = tick_rec(0, vec![org(0, 5), org(1, 5), org(2, 3), org(3, 3)]);
        // 2-2 tie between hungry=3 and hungry=5; lowest index (3) must win, deterministically.
        let sample = plurality_at(&rec, Role::Hungry).unwrap();
        assert_eq!(sample.winner, 3);
        assert_eq!(sample.share, 0.5);
    }

    #[test]
    fn transient_blip_is_not_a_sustained_event() {
        // Winner flips to 1 for exactly one sample, then reverts — must not be reported at
        // sustain_k=3.
        let ticks = vec![
            tick_rec(0, vec![org(0, 0), org(1, 0), org(2, 0)]),
            tick_rec(20, vec![org(0, 1), org(1, 1), org(2, 0)]),
            tick_rec(40, vec![org(0, 0), org(1, 0), org(2, 1)]),
            tick_rec(60, vec![org(0, 0), org(1, 0), org(2, 1)]),
        ];
        let events = detect_plurality_events(&ticks, Role::Hungry, 20, 3);
        assert!(
            events.is_empty(),
            "a one-sample blip should not register: {events:?}"
        );
    }

    #[test]
    fn sustained_change_is_detected_with_correct_shares() {
        let ticks = vec![
            tick_rec(0, vec![org(0, 0), org(1, 0), org(2, 0), org(3, 0)]),
            tick_rec(20, vec![org(0, 1), org(1, 1), org(2, 1), org(3, 0)]),
            tick_rec(40, vec![org(0, 1), org(1, 1), org(2, 1), org(3, 0)]),
            tick_rec(60, vec![org(0, 1), org(1, 1), org(2, 1), org(3, 1)]),
        ];
        let events = detect_plurality_events(&ticks, Role::Hungry, 20, 3);
        assert_eq!(events.len(), 1);
        let e = &events[0];
        assert_eq!((e.from, e.to, e.shift_tick), (0, 1, 20));
        assert!((e.share_at_shift - 0.75).abs() < 1e-9);
        assert!((e.peak_share_in_window - 1.0).abs() < 1e-9);
    }

    #[test]
    fn genome_hash_is_stable_and_content_sensitive() {
        assert_eq!(genome(1).hash(), genome(1).hash());
        assert_ne!(genome(1).hash(), genome(2).hash());
    }

    fn birth_eco(
        child_id: u32,
        parent_id: u32,
        tick: u32,
        species: Species,
        hungry: u16,
    ) -> BirthEventEco {
        let g = genome(hungry);
        BirthEventEco {
            child_id,
            parent_id,
            tick,
            species,
            decay_amount: g.decay_amount,
            repro_threshold: g.repro_threshold,
            repro_give_pct: g.repro_give_pct,
            hungry_promoter: g.hungry_promoter,
            repro_promoter: g.repro_promoter,
            sense_move: g.sense_move,
        }
    }

    fn org_eco(id: u32, species: Species, hungry: u16) -> OrgSnapshot2DEco {
        let g = genome(hungry);
        OrgSnapshot2DEco {
            id,
            x: 0,
            y: 0,
            energy: 100,
            species,
            decay_amount: g.decay_amount,
            repro_threshold: g.repro_threshold,
            repro_give_pct: g.repro_give_pct,
            hungry_promoter: g.hungry_promoter,
            repro_promoter: g.repro_promoter,
            sense_move: g.sense_move,
        }
    }

    fn tick_rec_eco(tick: u32, orgs: Vec<OrgSnapshot2DEco>) -> TickRecord2DEco {
        TickRecord2DEco {
            tick,
            organisms: orgs,
            food: Vec::new(),
            births: 0,
            starved: 0,
            contention_losses: 0,
            predation_kills: 0,
            total_ir_steps: 0,
        }
    }

    #[test]
    fn genome_fields_from_eco_matches_non_eco_for_the_same_values() {
        let b = birth_eco(2, 0, 10, Species::Predator, 1);
        let b_plain = birth(2, 0, 10, 1);
        assert_eq!(
            GenomeFields::from_birth_eco(&b),
            GenomeFields::from_birth(&b_plain)
        );

        let o = org_eco(5, Species::Grazer, 3);
        let o_plain = org(5, 3);
        assert_eq!(
            GenomeFields::from_snapshot_eco(&o),
            GenomeFields::from_snapshot(&o_plain)
        );
    }

    #[test]
    fn build_from_genesis_ids_supports_a_non_zero_starting_range() {
        // A second species' genesis ids start after the first species' population, e.g.
        // predators occupying ids 40..48 in a run with 40 grazers.
        let starting = genome(0);
        let genesis_ids = [40u32, 41, 42];
        let births = vec![birth(43, 41, 5, 1)]; // genuine mutation: parent(41)=0 -> child(43)=1
        let tree = LineageTree::build_from_genesis_ids(&genesis_ids, &starting, &births);

        assert_eq!(
            tree.get(40).unwrap().parent_id,
            None,
            "genesis organism, not a birth"
        );
        assert_eq!(tree.get(41).unwrap().parent_id, None);
        assert_eq!(tree.get(42).unwrap().parent_id, None);
        assert_eq!(
            trace_origin(&tree, Role::Hungry, 43),
            OriginKind::Mutated {
                origin_child_id: 43,
                origin_parent_id: 41,
                origin_tick: 5,
                parent_genome: genome(0),
                child_genome: genome(1),
            }
        );
    }

    #[test]
    fn eco_adapters_filter_by_species_and_preserve_values() {
        let ticks = vec![tick_rec_eco(
            0,
            vec![
                org_eco(0, Species::Grazer, 5),
                org_eco(1, Species::Predator, 9),
            ],
        )];
        let births = vec![
            birth_eco(2, 0, 10, Species::Grazer, 6),
            birth_eco(3, 1, 10, Species::Predator, 8),
        ];

        let grazer_ticks = eco_ticks_to_genome(&ticks, Species::Grazer);
        assert_eq!(grazer_ticks.len(), 1);
        assert_eq!(
            grazer_ticks[0].organisms.len(),
            1,
            "predator must be filtered out"
        );
        assert_eq!(grazer_ticks[0].organisms[0].id, 0);
        assert_eq!(grazer_ticks[0].organisms[0].hungry_promoter, 5);

        let predator_ticks = eco_ticks_to_genome(&ticks, Species::Predator);
        assert_eq!(
            predator_ticks[0].organisms.len(),
            1,
            "grazer must be filtered out"
        );
        assert_eq!(predator_ticks[0].organisms[0].id, 1);

        let grazer_births = eco_births_to_genome(&births, Species::Grazer);
        assert_eq!(grazer_births.len(), 1);
        assert_eq!(grazer_births[0].child_id, 2);
        assert_eq!(grazer_births[0].hungry_promoter, 6);

        let predator_births = eco_births_to_genome(&births, Species::Predator);
        assert_eq!(predator_births.len(), 1);
        assert_eq!(predator_births[0].child_id, 3);
    }

    #[test]
    fn eco_adapters_feed_detect_plurality_events_and_find_origins_unmodified() {
        // Two species sharing one run; only the predator's `hungry_promoter` plurality
        // actually shifts. The species-filtered view must isolate that shift without the
        // grazer population (constant at 0) interfering.
        let ticks = vec![
            tick_rec_eco(
                0,
                vec![
                    org_eco(0, Species::Grazer, 0),
                    org_eco(1, Species::Grazer, 0),
                    org_eco(10, Species::Predator, 0),
                    org_eco(11, Species::Predator, 0),
                ],
            ),
            tick_rec_eco(
                20,
                vec![
                    org_eco(0, Species::Grazer, 0),
                    org_eco(1, Species::Grazer, 0),
                    org_eco(12, Species::Predator, 1),
                    org_eco(13, Species::Predator, 1),
                ],
            ),
            tick_rec_eco(
                40,
                vec![
                    org_eco(0, Species::Grazer, 0),
                    org_eco(1, Species::Grazer, 0),
                    org_eco(12, Species::Predator, 1),
                    org_eco(13, Species::Predator, 1),
                ],
            ),
            tick_rec_eco(
                60,
                vec![
                    org_eco(0, Species::Grazer, 0),
                    org_eco(1, Species::Grazer, 0),
                    org_eco(12, Species::Predator, 1),
                    org_eco(13, Species::Predator, 1),
                ],
            ),
        ];
        let births = vec![
            birth_eco(12, 10, 20, Species::Predator, 1),
            birth_eco(13, 11, 20, Species::Predator, 1),
        ];

        let predator_ticks = eco_ticks_to_genome(&ticks, Species::Predator);
        let predator_births = eco_births_to_genome(&births, Species::Predator);
        let grazer_ticks = eco_ticks_to_genome(&ticks, Species::Grazer);

        // No grazer event: hungry_promoter never changes for that species.
        let grazer_events = detect_plurality_events(&grazer_ticks, Role::Hungry, 20, 3);
        assert!(grazer_events.is_empty());

        let predator_events = detect_plurality_events(&predator_ticks, Role::Hungry, 20, 3);
        assert_eq!(predator_events.len(), 1);
        let event = &predator_events[0];
        assert_eq!((event.from, event.to, event.shift_tick), (0, 1, 20));

        let genesis_ids = [10u32, 11];
        let starting = genome(0);
        let tree = LineageTree::build_from_genesis_ids(&genesis_ids, &starting, &predator_births);
        let origins = find_origins(&tree, &predator_ticks, event);
        assert_eq!(
            origins.len(),
            2,
            "two independent predator mutations, both live at the shift tick"
        );
    }
}
