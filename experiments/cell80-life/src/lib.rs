//! Shared surface between the `cell80-life` binary (the full predator/grazer/mutation
//! world, `main.rs`) and the EX-0 replay-gate harness (`experiments/deterministic-ecology.md`,
//! `ex0`/`genes`/`history`/`rng` below): genome-file loading and the food-grid world.
//! Everything genome/mutation/species-specific that EX-0 doesn't need (per-organism
//! `OrgGenome`, `Species`, cell-swap mutation, predator sensing) stays in `main.rs` — EX-0
//! runs a single homogeneous genome, no species, no mutation-driven genome diversity (see
//! the design doc for why that scope is deliberate, not a shortcut).
use serde::Deserialize;
use std::fs;
use std::path::Path;

pub mod contention;
pub mod ex0;
pub mod ex1;
pub mod ex2;
pub mod genes;
pub mod history;
pub mod lineage;
pub mod pools;
pub mod rng;
pub mod world2d;

fn default_species() -> String {
    "grazer".to_string()
}

#[derive(Deserialize)]
pub struct StartingGenome {
    pub id: String,
    pub initial_energy: u16,
    pub decay_amount: u16,
    pub repro_threshold: u16,
    pub repro_give_pct: u16,
    pub genes: StartingGenes,
    #[serde(default = "default_species")]
    pub species: String,
}

#[derive(Deserialize)]
pub struct StartingGenes {
    pub decay: String,
    pub hungry_promoter: String,
    pub eat: String,
    pub sense_move: String,
    pub repro_promoter: String,
    pub split: String,
}

pub fn load_starting_genome(path: &Path) -> StartingGenome {
    let src = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("reading genome {}: {e}", path.display()));
    serde_json::from_str(&src).unwrap_or_else(|e| panic!("parsing genome {}: {e}", path.display()))
}

/// A 1D food-tile world: `len` tiles, a food tile regrows `regrow_ticks` ticks after being
/// eaten. Identical behaviour to the pre-split hardcoded version when called as
/// `World::new(24, 40, 8)` — the split only parameterizes what were previously the
/// `WORLD_LEN`/`FOOD_VALUE`/`FOOD_REGROW_TICKS` constants in `main.rs`.
pub struct World {
    pub food: Vec<u16>,
    pub regrow_at: Vec<u16>,
    pub food_capacity: Vec<u16>,
    regrow_ticks: u16,
}

impl World {
    pub fn new(len: usize, food_value: u16, regrow_ticks: u16) -> Self {
        let mut food = vec![0u16; len];
        let mut i = 1;
        while i < len {
            food[i] = food_value;
            i += 3;
        }
        let food_capacity = food.clone();
        World {
            food,
            regrow_at: vec![0; len],
            food_capacity,
            regrow_ticks,
        }
    }

    pub fn len(&self) -> usize {
        self.food.len()
    }

    pub fn is_empty(&self) -> bool {
        self.food.is_empty()
    }

    pub fn eat_at(&mut self, pos: usize) {
        self.food[pos] = 0;
        self.regrow_at[pos] = self.regrow_ticks;
    }

    pub fn tick_regrow(&mut self) {
        for i in 0..self.food.len() {
            if self.regrow_at[i] > 0 {
                self.regrow_at[i] -= 1;
                if self.regrow_at[i] == 0 {
                    self.food[i] = self.food_capacity[i];
                }
            }
        }
    }
}
