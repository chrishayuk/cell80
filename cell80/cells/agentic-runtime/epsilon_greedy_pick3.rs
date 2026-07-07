//! Epsilon-greedy selection: returns alt_idx (explore) if rand_bps < epsilon_bps, else best_idx (exploit) — composes with the already-shipped lcg_next/xorshift16 (for rand_bps, via safe_mod against 10000) and epsilon_bps as a basis-points exploration rate (e.g. 1000 = 10% exploration).
//! tags: epsilon-greedy, explore, exploit, bandit, random, selection, agentic, state
//! entry: EpsilonGreedyPick3::run
struct EpsilonGreedyPick3 { rand_bps: u16, epsilon_bps: u16, best_idx: u16, alt_idx: u16 }
impl EpsilonGreedyPick3 {
    fn run(&mut self) -> u16 {
        if self.rand_bps < self.epsilon_bps { self.alt_idx } else { self.best_idx }
    }
}
