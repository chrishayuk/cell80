//! A toroidal 2D food-tile world for EX-1 (`experiments/deterministic-ecology.md`) — the
//! 2D sibling of `World` (the 1D, non-toroidal world `ex0.rs`/the original `cell80-life`
//! binary use). Kept as a separate type rather than a generalization of `World`, so neither
//! of those needs to change.
use crate::rng;

/// Food placement's dedicated stream — distinct from `rng::MUTATION_STREAM` and
/// `contention::EAT_CONTENTION_STREAM`. Reuses `rng::draw`'s third parameter (normally an
/// organism id) as a **tile index** instead — `draw` doesn't care what the id represents,
/// only that the whole `(seed, tick, id, stream)` tuple is a stable, order-independent key,
/// which a tile index is just as much as an organism id is.
pub const WORLD_INIT_STREAM: u8 = 2;

/// A square-or-rectangular, row-major, wraparound (toroidal) world. Food is placed
/// **probabilistically** at construction — tile `i` gets food iff a deterministic draw
/// keyed by `(seed, i)` falls under `food_density` — rather than a fixed lattice stride, so
/// density is a real, continuous sweep knob under the same `f(seed, ...)` determinism
/// contract as everything else.
pub struct World2D {
    pub width: usize,
    pub height: usize,
    pub food: Vec<u16>,
    pub regrow_at: Vec<u16>,
    pub food_capacity: Vec<u16>,
    regrow_ticks: u16,
}

impl World2D {
    pub fn new(
        seed: u64,
        width: usize,
        height: usize,
        food_density: f64,
        food_value: u16,
        regrow_ticks: u16,
    ) -> Self {
        let n = width * height;
        let threshold = (food_density.clamp(0.0, 1.0) * u32::MAX as f64) as u32;
        let food: Vec<u16> = (0..n)
            .map(|i| {
                let draw = rng::draw(seed, 0, i as u32, WORLD_INIT_STREAM);
                if draw < threshold {
                    food_value
                } else {
                    0
                }
            })
            .collect();
        let food_capacity = food.clone();
        World2D {
            width,
            height,
            food,
            regrow_at: vec![0; n],
            food_capacity,
            regrow_ticks,
        }
    }

    pub fn len(&self) -> usize {
        self.width * self.height
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    pub fn xy(&self, idx: usize) -> (usize, usize) {
        (idx % self.width, idx / self.width)
    }

    /// Toroidal neighbor lookup: `(dx, dy)` each typically in `{-1, 0, 1}`; always defined,
    /// wrapping both axes — no boundary-clamp branches needed, unlike the 1D `World`.
    pub fn neighbor_index(&self, idx: usize, dx: i32, dy: i32) -> usize {
        let (x, y) = self.xy(idx);
        let nx = (x as i32 + dx).rem_euclid(self.width as i32) as usize;
        let ny = (y as i32 + dy).rem_euclid(self.height as i32) as usize;
        self.idx(nx, ny)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idx_xy_round_trip() {
        let w = World2D::new(1, 12, 7, 0.3, 40, 8);
        for y in 0..7 {
            for x in 0..12 {
                assert_eq!(w.xy(w.idx(x, y)), (x, y));
            }
        }
    }

    #[test]
    fn neighbor_wraps_both_axes() {
        let w = World2D::new(1, 10, 10, 0.3, 40, 8);
        let top_left = w.idx(0, 0);
        // one step north/west from (0,0) wraps to the far edge, not a clamp.
        assert_eq!(w.neighbor_index(top_left, -1, 0), w.idx(9, 0));
        assert_eq!(w.neighbor_index(top_left, 0, -1), w.idx(0, 9));
        let bottom_right = w.idx(9, 9);
        assert_eq!(w.neighbor_index(bottom_right, 1, 0), w.idx(0, 9));
        assert_eq!(w.neighbor_index(bottom_right, 0, 1), w.idx(9, 0));
    }

    #[test]
    fn deterministic_food_layout_for_same_seed() {
        let a = World2D::new(0x5eed, 20, 20, 0.25, 40, 8);
        let b = World2D::new(0x5eed, 20, 20, 0.25, 40, 8);
        assert_eq!(a.food, b.food);
    }

    #[test]
    fn different_seeds_usually_differ() {
        let a = World2D::new(1, 20, 20, 0.25, 40, 8);
        let b = World2D::new(2, 20, 20, 0.25, 40, 8);
        assert_ne!(
            a.food, b.food,
            "two different seeds produced an identical food layout"
        );
    }

    #[test]
    fn density_roughly_matches_requested_fraction() {
        let side = 200; // 40,000 tiles — large enough for a stable empirical fraction
        for &density in &[0.0, 0.1, 0.33, 0.6, 1.0] {
            let w = World2D::new(7, side, side, density, 40, 8);
            let observed = w.food.iter().filter(|&&f| f > 0).count() as f64 / w.len() as f64;
            assert!(
                (observed - density).abs() < 0.02,
                "density {density}: observed {observed}, want within 0.02"
            );
        }
    }

    #[test]
    fn eat_and_regrow_round_trip() {
        let mut w = World2D::new(1, 4, 4, 1.0, 40, 3);
        let pos = w.idx(1, 1);
        assert_eq!(w.food[pos], 40);
        w.eat_at(pos);
        assert_eq!(w.food[pos], 0);
        w.tick_regrow();
        w.tick_regrow();
        assert_eq!(
            w.food[pos], 0,
            "regrow_ticks=3, should still be empty after 2 ticks"
        );
        w.tick_regrow();
        assert_eq!(w.food[pos], 40, "should regrow on the 3rd tick");
    }
}
