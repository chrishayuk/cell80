//! Cell80 Life: a minimal grid-world prototype where organism behaviour is driven by real
//! `.cell` cartridges compiled from the existing cell80 stdlib, run through a `CellHost`.
//! No new cells are authored here — decay, eat, movement, reproduction, and (for predators)
//! hunting all reuse curated library cells, matching the "genes are cards from the existing
//! deck" framing in ../cell80-life.md.
//!
//! Each organism carries its own genome (numeric thresholds plus which cell backs the
//! `hungry_promoter`/`repro_promoter`/`sense_move` roles) and mutates it on reproduction —
//! numeric drift on the thresholds, and a rarer swap to a *discovered* sibling cell. The
//! candidate pool for each role isn't hand-picked: at startup this scans every cell source
//! under `cell80/cells/` and keeps the ones with a matching signature (2 `u16` params + `u16`
//! return for the promoters, 3 for movement) and no `&mut self` state. Nothing filters by
//! whether a candidate actually *behaves* like a boolean gate or a sane movement rule: the
//! promoter check is a plain `== 1`, so a non-boolean cell just almost never fires, and the
//! movement match has a `_ => {}` ("stay") fallback for any out-of-range action code. Whether
//! a swap is any good is left entirely to whether it helps the organism survive to reproduce.
//!
//! Multiple *species* can now coexist in the same world — organisms with structurally
//! different pipelines, not just different parameter values of the same pipeline. A grazer
//! senses/eats food; a predator senses/hunts *other organisms* instead, using the exact same
//! genome roles and cells (`sense_move` targets prey positions instead of food positions,
//! `hungry_promoter` gates an attack instead of eating, `eat` still converts a captured energy
//! value into the attacker's own energy) — no new cells needed for predation, same reuse
//! discipline as everything else here. A successful attack is a clean kill: the victim is
//! removed and the attacker gains its entire energy total via the same `eat` cell a grazer
//! uses on food. Species itself doesn't mutate in this version — a predator's lineage stays
//! predators, a grazer's stays grazers; only the numeric thresholds and role-cell choices
//! within a species evolve, exactly as before.
//!
//! `decay`/`eat`/`split` stay fixed (and shared across every species in a run — mixing genomes
//! with different cells for these roles isn't supported, asserted at startup) for the same
//! reason as always: the stdlib doesn't have a same-scale alternative for them without
//! changing what the numeric parameter even means. The PRNG is a fixed-seed xorshift, not OS
//! randomness, so a run with the same genome files, tick count, and seed is fully
//! reproducible.
use cell80::{Cartridge, CartridgeOpts, CellConfig, CellHost};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const WORLD_LEN: usize = 24;
const FOOD_VALUE: u16 = 40;
const FOOD_REGROW_TICKS: u16 = 8;
const BUDGET: u64 = 10_000;

const DECAY_BOUNDS: (i32, i32) = (1, 6);
const THRESHOLD_BOUNDS: (i32, i32) = (50, 400);
const GIVE_PCT_BOUNDS: (i32, i32) = (10, 90);
const NUMERIC_MUTATE_PCT: u64 = 25;
const SWAP_MUTATE_PCT: u64 = 8;

fn default_species() -> String {
    "grazer".to_string()
}

#[derive(Deserialize)]
struct StartingGenome {
    id: String,
    initial_energy: u16,
    decay_amount: u16,
    repro_threshold: u16,
    repro_give_pct: u16,
    genes: StartingGenes,
    #[serde(default = "default_species")]
    species: String,
}

#[derive(Deserialize)]
struct StartingGenes {
    decay: String,
    hungry_promoter: String,
    eat: String,
    sense_move: String,
    repro_promoter: String,
    split: String,
}

/// A structurally different pipeline, not just different parameter values. Fixed for an
/// organism's whole lineage in this version — species itself doesn't mutate.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Species {
    Grazer,
    Predator,
}

impl Species {
    fn parse(s: &str) -> Self {
        match s {
            "grazer" => Species::Grazer,
            "predator" => Species::Predator,
            other => panic!("unknown species `{other}` (expected \"grazer\" or \"predator\")"),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Species::Grazer => "grazer",
            Species::Predator => "predator",
        }
    }
}

/// The per-organism, heritable part of the genome: the numeric thresholds plus which cell
/// backs each swappable role. Cloned into a child on reproduction, then [`mutate`]d. Role
/// fields are owned `String`s (not `&'static str`) because the candidate pool is discovered
/// at runtime from the stdlib, not a fixed compile-time pair.
#[derive(Clone)]
struct OrgGenome {
    decay_amount: u16,
    repro_threshold: u16,
    repro_give_pct: u16,
    hungry_promoter: String,
    repro_promoter: String,
    sense_move: String,
}

/// A deterministic xorshift64* PRNG — no OS entropy, so a run is reproducible from its seed.
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

    fn step(&mut self, span: i32) -> i32 {
        (self.next_u64() % (2 * span as u64 + 1)) as i32 - span
    }
}

fn clamp_u16(v: i32, bounds: (i32, i32)) -> u16 {
    v.clamp(bounds.0, bounds.1) as u16
}

/// Pick a uniformly random *different* member of `pool` from `current` — the swap mutation
/// over a discovered pool, in place of the old fixed two-way flip. Falls back to `current`
/// unchanged if the pool has nothing else to offer (shouldn't happen at real pool sizes).
fn pick_other(rng: &mut Rng, pool: &[String], current: &str) -> String {
    if pool.len() <= 1 {
        return current.to_string();
    }
    loop {
        let candidate = &pool[rng.below(pool.len())];
        if candidate != current {
            return candidate.clone();
        }
    }
}

struct Pools {
    promoters: Vec<String>,
    movement: Vec<String>,
}

fn mutate(rng: &mut Rng, g: &OrgGenome, pools: &Pools) -> OrgGenome {
    let mut child = g.clone();
    if rng.chance(NUMERIC_MUTATE_PCT) {
        child.decay_amount = clamp_u16(child.decay_amount as i32 + rng.step(1), DECAY_BOUNDS);
    }
    if rng.chance(NUMERIC_MUTATE_PCT) {
        child.repro_threshold = clamp_u16(
            child.repro_threshold as i32 + rng.step(10),
            THRESHOLD_BOUNDS,
        );
    }
    if rng.chance(NUMERIC_MUTATE_PCT) {
        child.repro_give_pct =
            clamp_u16(child.repro_give_pct as i32 + rng.step(5), GIVE_PCT_BOUNDS);
    }
    if rng.chance(SWAP_MUTATE_PCT) {
        child.hungry_promoter = pick_other(rng, &pools.promoters, &child.hungry_promoter);
    }
    if rng.chance(SWAP_MUTATE_PCT) {
        child.repro_promoter = pick_other(rng, &pools.promoters, &child.repro_promoter);
    }
    if rng.chance(SWAP_MUTATE_PCT) {
        child.sense_move = pick_other(rng, &pools.movement, &child.sense_move);
    }
    child
}

struct World {
    food: Vec<u16>,
    regrow_at: Vec<u16>,
    food_capacity: Vec<u16>,
}

impl World {
    fn new() -> Self {
        let mut food = vec![0u16; WORLD_LEN];
        let mut i = 1;
        while i < WORLD_LEN {
            food[i] = FOOD_VALUE;
            i += 3;
        }
        let food_capacity = food.clone();
        World {
            food,
            regrow_at: vec![0; WORLD_LEN],
            food_capacity,
        }
    }

    fn eat_at(&mut self, pos: usize) {
        self.food[pos] = 0;
        self.regrow_at[pos] = FOOD_REGROW_TICKS;
    }

    fn tick_regrow(&mut self) {
        for i in 0..WORLD_LEN {
            if self.regrow_at[i] > 0 {
                self.regrow_at[i] -= 1;
                if self.regrow_at[i] == 0 {
                    self.food[i] = self.food_capacity[i];
                }
            }
        }
    }
}

struct Organism {
    pos: usize,
    energy: u16,
    genome: OrgGenome,
    species: Species,
}

/// The other organisms' `(position, energy, species)` as of the *start* of this tick,
/// snapshotted before anyone acts — so a predator's sensing and attacking are based on a
/// consistent view of the world, not on whatever partial state earlier-processed organisms
/// this same tick happen to have left behind.
///
/// Only `Species::Grazer` counts as prey. An earlier version matched *any* other organism by
/// position alone, so two co-located predators would sense — and attack — each other: a
/// generic "who's on my tile" lookup, reused for predation without adding the one check
/// predation actually needs. That produced exactly the "predators confusedly sense each other
/// as prey" finding in `../cell80-life.md`: predators killing each other collapsed the
/// predator population down to a single survivor, which then had no prey left (grazers wiped
/// out the same way) and starved alone. Filtering to `Species::Grazer` here is the fix — a
/// predator can still be *attacked* by another predator's stray sensing, but no longer
/// identified as valid prey, so the attack promoter never fires on it.
fn prey_at(
    snapshot: &[(usize, u16, Species)],
    pos: usize,
    self_idx: usize,
) -> Option<(usize, u16)> {
    snapshot
        .iter()
        .enumerate()
        .find(|&(j, &(p, _, sp))| j != self_idx && p == pos && sp == Species::Grazer)
        .map(|(j, &(_, e, _))| (j, e))
}

/// Compile one stdlib cell source (by filename stem, from `cell80/cells/`) and load it into
/// the host under that same id, so it can be reused each tick as a gene/promoter.
fn load_gene(host: &mut CellHost, cells_dir: &Path, name: &str) -> usize {
    let path = cells_dir.join(format!("{name}.rs"));
    let src =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    let cart = Cartridge::compile(
        &src,
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some(name.to_string()),
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("compiling {name}: {e}"));
    host.add(cart);
    host.load(name)
        .unwrap_or_else(|e| panic!("loading {name}: {e}"))
}

fn load_starting_genome(path: &Path) -> StartingGenome {
    let src = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("reading genome {}: {e}", path.display()));
    serde_json::from_str(&src).unwrap_or_else(|e| panic!("parsing genome {}: {e}", path.display()))
}

/// Scan every `.rs` cell source under `cells_dir`, compile each, and bucket it by arity into
/// the promoter pool (2 `u16` params, `u16` return) or the movement pool (3 params) — skipping
/// anything that fails to compile, has `&mut self` state (a plain fn call can't read its
/// fields back), or returns/takes anything other than `u16`. Names are sorted before
/// filtering: directory iteration order isn't guaranteed, and pool order feeds the
/// deterministic PRNG's index choices, so an unsorted pool would silently break run-to-run
/// reproducibility across platforms/filesystems.
fn discover_pools(cells_dir: &Path) -> Pools {
    let mut names: Vec<String> = fs::read_dir(cells_dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", cells_dir.display()))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
        .filter_map(|e| {
            e.path()
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
        })
        .collect();
    names.sort();

    let mut promoters = Vec::new();
    let mut movement = Vec::new();
    for name in names {
        let path = cells_dir.join(format!("{name}.rs"));
        let Ok(src) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(cart) = Cartridge::compile(
            &src,
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some(name.clone()),
                ..Default::default()
            },
        ) else {
            continue;
        };
        let sig = &cart.manifest.signature;
        if !sig.state.is_empty()
            || sig.ret != "u16"
            || !sig.params.iter().all(|(_, ty)| ty == "u16")
        {
            continue;
        }
        match sig.params.len() {
            2 => promoters.push(name),
            3 => movement.push(name),
            _ => {}
        }
    }
    Pools {
        promoters,
        movement,
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let ticks: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(200);
    let genome_arg = args.next();
    let genome_paths: Vec<PathBuf> = match &genome_arg {
        Some(s) => s.split(',').map(PathBuf::from).collect(),
        None => {
            let default = Path::new(env!("CARGO_MANIFEST_DIR")).join("genomes/grazer.json");
            vec![default.clone(), default]
        }
    };
    let seed: u64 = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x5eed_1234_c311_80ff);

    let startings: Vec<StartingGenome> = genome_paths
        .iter()
        .map(|p| load_starting_genome(p))
        .collect();
    let cells_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cell80/cells");
    let mut host = CellHost::new();

    // decay/eat/split are shared across every species in a run — asserted consistent rather
    // than made per-organism, since nothing yet needs them to differ.
    let (decay_name, eat_name, split_name) = (
        &startings[0].genes.decay,
        &startings[0].genes.eat,
        &startings[0].genes.split,
    );
    for (s, path) in startings.iter().zip(&genome_paths) {
        assert_eq!(
            &s.genes.decay,
            decay_name,
            "{}: decay cell `{}` differs from the run's shared `{decay_name}` — mixing decay \
             cells across genomes in one run isn't supported",
            path.display(),
            s.genes.decay
        );
        assert_eq!(
            &s.genes.eat,
            eat_name,
            "{}: eat cell `{}` differs from the run's shared `{eat_name}`",
            path.display(),
            s.genes.eat
        );
        assert_eq!(
            &s.genes.split,
            split_name,
            "{}: split cell `{}` differs from the run's shared `{split_name}`",
            path.display(),
            s.genes.split
        );
    }
    let decay = load_gene(&mut host, &cells_dir, decay_name);
    let eat = load_gene(&mut host, &cells_dir, eat_name);
    let split = load_gene(&mut host, &cells_dir, split_name);

    let pools = discover_pools(&cells_dir);
    let mut cells: HashMap<String, usize> = HashMap::new();
    for name in pools.promoters.iter().chain(pools.movement.iter()) {
        cells.insert(name.clone(), load_gene(&mut host, &cells_dir, name));
    }

    for (s, path) in startings.iter().zip(&genome_paths) {
        assert!(
            pools.promoters.contains(&s.genes.hungry_promoter),
            "{}: hungry_promoter `{}` isn't in the discovered promoter pool",
            path.display(),
            s.genes.hungry_promoter
        );
        assert!(
            pools.promoters.contains(&s.genes.repro_promoter),
            "{}: repro_promoter `{}` isn't in the discovered promoter pool",
            path.display(),
            s.genes.repro_promoter
        );
        assert!(
            pools.movement.contains(&s.genes.sense_move),
            "{}: sense_move `{}` isn't in the discovered movement pool",
            path.display(),
            s.genes.sense_move
        );
    }

    let mut world = World::new();
    let mut organisms: Vec<Organism> = startings
        .iter()
        .enumerate()
        .map(|(i, s)| Organism {
            pos: i * WORLD_LEN / startings.len(),
            energy: s.initial_energy,
            genome: OrgGenome {
                decay_amount: s.decay_amount,
                repro_threshold: s.repro_threshold,
                repro_give_pct: s.repro_give_pct,
                hungry_promoter: s.genes.hungry_promoter.clone(),
                repro_promoter: s.genes.repro_promoter.clone(),
                sense_move: s.genes.sense_move.clone(),
            },
            species: Species::parse(&s.species),
        })
        .collect();
    let mut rng = Rng::new(seed);

    println!(
        "genomes: {}  seed={seed:#x}  pools: {} promoters, {} movement",
        startings
            .iter()
            .zip(&genome_paths)
            .map(|(s, p)| format!("{} [{}] ({})", s.id, s.species, p.display()))
            .collect::<Vec<_>>()
            .join(", "),
        pools.promoters.len(),
        pools.movement.len()
    );

    let mut births = 0u32;
    let mut starved = 0u32;
    let mut eaten = 0u32;
    let mut last_tick = 0u32;

    for tick in 0..ticks {
        last_tick = tick;
        let snapshot: Vec<(usize, u16, Species)> = organisms
            .iter()
            .map(|o| (o.pos, o.energy, o.species))
            .collect();
        let mut killed: HashSet<usize> = HashSet::new();
        let mut next_gen = Vec::new();
        let mut tagged_survivors: Vec<(usize, Organism)> = Vec::with_capacity(organisms.len());

        for (idx, mut org) in organisms.into_iter().enumerate() {
            if killed.contains(&idx) {
                // Eaten by an earlier-processed predator this same tick — already counted
                // in `eaten` at the point of the kill, below.
                continue;
            }

            // gene: energy_decay (species-agnostic)
            org.energy = host
                .run_fast(decay, &[org.energy, org.genome.decay_amount], BUDGET)
                .unwrap()
                .result;

            match org.species {
                Species::Grazer => {
                    let food_here = world.food[org.pos];
                    let food_left = if org.pos > 0 {
                        world.food[org.pos - 1]
                    } else {
                        0
                    };
                    let food_right = if org.pos + 1 < WORLD_LEN {
                        world.food[org.pos + 1]
                    } else {
                        0
                    };

                    let sense_move = cells[&org.genome.sense_move];
                    let action = host
                        .run_fast(sense_move, &[food_here, food_left, food_right], BUDGET)
                        .unwrap()
                        .result;

                    match action {
                        0 => {
                            let hungry = cells[&org.genome.hungry_promoter];
                            let is_hungry_here = host
                                .run_fast(hungry, &[food_here, 0], BUDGET)
                                .unwrap()
                                .result;
                            if is_hungry_here == 1 {
                                org.energy = host
                                    .run_fast(eat, &[org.energy, food_here], BUDGET)
                                    .unwrap()
                                    .result;
                                world.eat_at(org.pos);
                            }
                        }
                        1 if org.pos > 0 => org.pos -= 1,
                        2 if org.pos + 1 < WORLD_LEN => org.pos += 1,
                        _ => {}
                    }
                }
                Species::Predator => {
                    let prey_here = prey_at(&snapshot, org.pos, idx).map_or(0, |(_, e)| e);
                    let prey_left = if org.pos > 0 {
                        prey_at(&snapshot, org.pos - 1, idx).map_or(0, |(_, e)| e)
                    } else {
                        0
                    };
                    let prey_right = if org.pos + 1 < WORLD_LEN {
                        prey_at(&snapshot, org.pos + 1, idx).map_or(0, |(_, e)| e)
                    } else {
                        0
                    };
                    // A predator with zero prey sensed never moves (argmax3(0,0,0) == 0,
                    // "stay"), and since prey mostly camp at food tiles rather than roam, an
                    // idle predator can sit frozen for its entire life without ever getting a
                    // real encounter (observed directly: a predator's position never changed
                    // once across a 500-tick run). EXPLORE_BIAS breaks that tie toward a
                    // direction that holds for `EXPLORE_HALF_PERIOD` ticks before flipping — a
                    // deterministic sweep, not a random walk. Flipping every single tick was
                    // tried first and just oscillates the predator between two adjacent tiles
                    // (recompute position -> recompute bias -> immediately reverse), never
                    // actually covering ground; caught by checking the position trace, not by
                    // assuming the fix worked because it compiled and ran. The bias is
                    // negligible next to any genuine prey signal (energy is routinely 50+).
                    const EXPLORE_BIAS: u16 = 1;
                    const EXPLORE_HALF_PERIOD: u32 = 20;
                    let (prey_left, prey_right) = if (tick / EXPLORE_HALF_PERIOD) % 2 == 0 {
                        (prey_left + EXPLORE_BIAS, prey_right)
                    } else {
                        (prey_left, prey_right + EXPLORE_BIAS)
                    };

                    let sense_move = cells[&org.genome.sense_move];
                    let action = host
                        .run_fast(sense_move, &[prey_here, prey_left, prey_right], BUDGET)
                        .unwrap()
                        .result;

                    match action {
                        0 => {
                            // promoter: attack_here (reuses the `hungry_promoter` role/cell)
                            let attack = cells[&org.genome.hungry_promoter];
                            let can_attack = host
                                .run_fast(attack, &[prey_here, 0], BUDGET)
                                .unwrap()
                                .result;
                            if can_attack == 1 {
                                if let Some((victim_idx, victim_energy)) =
                                    prey_at(&snapshot, org.pos, idx)
                                {
                                    killed.insert(victim_idx);
                                    eaten += 1;
                                    // gene: eat — same cell a grazer uses on food, fed the
                                    // victim's whole energy total (a clean kill, not a wound)
                                    org.energy = host
                                        .run_fast(eat, &[org.energy, victim_energy], BUDGET)
                                        .unwrap()
                                        .result;
                                }
                            }
                        }
                        1 if org.pos > 0 => org.pos -= 1,
                        2 if org.pos + 1 < WORLD_LEN => org.pos += 1,
                        _ => {}
                    }
                }
            }

            if org.energy == 0 {
                starved += 1;
                continue;
            }

            // promoter: if_energy_high (species-agnostic)
            let repro_promoter = cells[&org.genome.repro_promoter];
            let ready = host
                .run_fast(
                    repro_promoter,
                    &[org.energy, org.genome.repro_threshold],
                    BUDGET,
                )
                .unwrap()
                .result;
            if ready == 1 {
                let parent_keep = host
                    .run_fast(split, &[org.energy, org.genome.repro_give_pct], BUDGET)
                    .unwrap()
                    .result;
                let child_energy = org.energy - parent_keep;
                org.energy = parent_keep;
                let child_pos = if org.pos + 1 < WORLD_LEN {
                    org.pos + 1
                } else {
                    org.pos - 1
                };
                let child_genome = mutate(&mut rng, &org.genome, &pools);
                next_gen.push(Organism {
                    pos: child_pos,
                    energy: child_energy,
                    genome: child_genome,
                    species: org.species,
                });
                births += 1;
            }

            tagged_survivors.push((idx, org));
        }

        // Removes anyone eaten by a predator processed *after* they'd already been pushed
        // to survivors this same tick.
        tagged_survivors.retain(|(idx, _)| !killed.contains(idx));
        let mut survivors: Vec<Organism> = tagged_survivors.into_iter().map(|(_, o)| o).collect();
        survivors.extend(next_gen);
        organisms = survivors;
        world.tick_regrow();

        if tick % 20 == 0 || organisms.is_empty() {
            println!("{}", render(tick, &world, &organisms));
        }
        if organisms.is_empty() {
            println!("-- extinction at tick {tick} --");
            break;
        }
    }

    println!(
        "\nfinal: {} organisms, {births} births, {starved} starved, {eaten} eaten over {} ticks",
        organisms.len(),
        last_tick + 1
    );
    println!("{}", genome_stats(&organisms));
}

fn render(tick: u32, world: &World, organisms: &[Organism]) -> String {
    let mut line: Vec<char> = world
        .food
        .iter()
        .map(|&f| if f > 0 { '*' } else { '.' })
        .collect();
    for org in organisms {
        line[org.pos] = match (org.species, org.energy >= 100) {
            (Species::Predator, true) => 'X',
            (Species::Predator, false) => 'x',
            (Species::Grazer, true) => '@',
            (Species::Grazer, false) => 'o',
        };
    }
    let n = organisms.len();
    let grazers = organisms
        .iter()
        .filter(|o| o.species == Species::Grazer)
        .count();
    let predators = n - grazers;
    let avg_energy = if organisms.is_empty() {
        0
    } else {
        organisms.iter().map(|o| o.energy as u32).sum::<u32>() / n as u32
    };
    let strip: String = line.into_iter().collect();
    format!(
        "t={tick:>4}  [{strip}]  n={n:<3} (grazers={grazers} predators={predators})  \
         avg_energy={avg_energy}  {}",
        genome_stats(organisms)
    )
}

/// For a role with a large discovered pool, "X/N use is_ge" doesn't generalize — report
/// diversity (how many distinct cells are currently in the population) and the current
/// plurality winner instead.
fn mode_and_diversity<'a>(names: impl Iterator<Item = &'a String>) -> (usize, &'a str, usize) {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for name in names {
        *counts.entry(name.as_str()).or_insert(0) += 1;
    }
    let distinct = counts.len();
    let (top_name, top_count) = counts
        .into_iter()
        .max_by_key(|(_, c)| *c)
        .unwrap_or(("none", 0));
    (distinct, top_name, top_count)
}

fn genome_stats(organisms: &[Organism]) -> String {
    let n = organisms.len();
    if n == 0 {
        return "(no organisms)".to_string();
    }
    let n32 = n as u32;
    let avg_decay = organisms
        .iter()
        .map(|o| o.genome.decay_amount as u32)
        .sum::<u32>()
        / n32;
    let avg_thresh = organisms
        .iter()
        .map(|o| o.genome.repro_threshold as u32)
        .sum::<u32>()
        / n32;
    let avg_give = organisms
        .iter()
        .map(|o| o.genome.repro_give_pct as u32)
        .sum::<u32>()
        / n32;
    let (hungry_distinct, hungry_top, hungry_top_n) =
        mode_and_diversity(organisms.iter().map(|o| &o.genome.hungry_promoter));
    let (repro_distinct, repro_top, repro_top_n) =
        mode_and_diversity(organisms.iter().map(|o| &o.genome.repro_promoter));
    let (move_distinct, move_top, move_top_n) =
        mode_and_diversity(organisms.iter().map(|o| &o.genome.sense_move));
    format!(
        "genome avg: decay={avg_decay} thresh={avg_thresh} give={avg_give}%  \
         hungry: {hungry_distinct} distinct (top {hungry_top}={hungry_top_n}/{n})  \
         repro: {repro_distinct} distinct (top {repro_top}={repro_top_n}/{n})  \
         move: {move_distinct} distinct (top {move_top}={move_top_n}/{n})"
    )
}

impl std::fmt::Display for Species {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}
