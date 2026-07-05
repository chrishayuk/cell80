# Experiment: Cell80 Life (DNA / artificial life)

Status: **speculative, not on the roadmap.** Parked here deliberately rather than in `docs/`,
which is reserved for specs of things that are built or actively being built. Current priority
stays retrieval / the type-led index (see `docs/roadmap.md`); this does not compete with that.
Empirical results from the mutation/selection runs: `cell80-life-findings.md`.

A minimal prototype exists at `experiments/cell80-life/` (workspace member `cell80-life`,
`cargo run -p cell80-life -- <ticks> [genome.json]`): a 1D grid world where organisms sense,
move, eat, and reproduce by calling real `.cell` cartridges through a `CellHost` — not plain
Rust closures. It deliberately reuses existing curated stdlib cells as genes/promoters
(`sub_sat` for decay, `is_gt`/`is_ge` for promoters, `add_sat` for eating, `argmax3` for
movement choice, `discount_percent` for the reproduction split) rather than authoring new
ones, so it's proof that composition-of-the-existing-deck already produces population dynamics
(foraging, starvation, reproduction, local extinction) without inventing any new library
surface.

The genome is data: `genomes/*.json` names which stdlib cell backs each role plus the tunable
numbers (initial energy, decay rate, reproduction threshold/split), loaded at startup — no
recompile to change behaviour. Two genomes exist (`genomes/grazer.json`, the steady default;
`genomes/rapid_reproducer.json`, a cheaper/lower-threshold r-strategy) and produce genuinely
different population curves: the grazer settles into a stable ~8–12, the rapid reproducer
boom-busts between 2 and 6 with roughly 3x the births and deaths over the same run length.

Mutation is now real: each organism carries its own genome (the numeric thresholds plus which
cell backs `hungry_promoter`/`repro_promoter`/`sense_move`), and a child's genome is the
parent's, mutated — numeric drift on the thresholds (~25% chance/field/birth), plus a rarer
(~8%) swap to another cell. A fixed-seed xorshift PRNG keeps every run fully reproducible.
`decay`/`eat`/`split` stay fixed for a whole run — the stdlib has no same-scale alternative for
them without changing what the numeric parameter even means (e.g. a bps-based decay cell), so
swapping those is out of scope for now. Over a 2000-tick run this already shows real selection,
not just drift: a deliberately maladaptive movement swap (seek *away* from food) kept getting
introduced by mutation and purged back out within a few dozen ticks every time, while
`repro_threshold` wandered within a stable band instead of running away. `genome avg: ...` is
printed each stats line so this is directly observable, not just asserted.

The swap mutation no longer picks between two hand-picked alternatives — it picks from every
cell in the real stdlib with a matching signature, discovered at startup (`discover_pools` in
`main.rs`): 2-`u16`-arg/`u16`-return cells for the promoter roles, 3-arg for movement, no
`&mut self` state. On the current library that's 54 promoter candidates and 26 movement
candidates, not 2. Nothing pre-filters for "makes sense as a boolean gate" — the promoter check
is a plain `== 1`, so a non-boolean cell just mostly never fires; a movement cell that returns
an out-of-range action code falls through to "stay." Whether a swap survives is left entirely
to the ecosystem. First run at this scale: the population still converges on the same
essentially-`is_gt`/`is_ge`/`argmax3`-dominated composition as the small hand-picked pool did,
but with visibly richer, noisier dynamics along the way — total births roughly tripled (408 vs.
~150 over the same 2000 ticks on the same genome) from transient population booms that later
got corrected, and at one point `is_coprime` (gcd(a,b)==1 — a number-theoretic check with no
business being a fitness-threshold gate) briefly became the population's plurality choice for
`repro_promoter` before reverting. That's exactly the small-population genetic drift already
described in `cell80-life-findings.md` Finding 4, now visible in a completely different,
open-ended part of the search space rather than the earlier hand-picked one.

Still hardcoded: the *order* and *effect* of each role (decay always runs first, "eat" always
means `energy += food`, ...) — the genome format/mutation picks the cell and the numbers, not
the pipeline shape. 2D/multi-directional worlds are still unbuilt (blocked on the library not
yet having an argmax-over-4/5 cell).

Multiple *species* — structurally different pipelines, not just parameter drift — now coexist:
a predator (`genomes/predator.json`) senses/hunts *other organisms* instead of food, reusing
the exact same genome roles and cells a grazer uses on food (`eat` converts a captured energy
value into the attacker's own energy either way; a successful attack is a clean kill). First
run: a predator with only 1-tile sensing and no fallback movement just sat frozen at its start
tile for its whole life and starved, because prey camp at food tiles rather than roam — an
honest finding in its own right, not a bug. Giving the predator a small "explore when idle"
bias (alternate a tie-breaking nudge left/right every 20 ticks, negligible next to any real
prey signal) got it moving, and it swept into a grazer cluster, killed 2, reproduced once —
then two co-located predators appear to confusedly sense *each other* as prey near the same
tile, and the lineage starved out rather than establishing a stable population. Real, working
predation; not yet a stable predator-prey equilibrium — that's open, not chased further yet.

## Idea

A Cell80 organism is not a model — it's a bundle of tiny executable genes. Each gene is a
`.cell`: bounded, deterministic, inspectable, cheap to run, capable of one tiny behaviour.

```
one cell   = one gene
genome     = many cells (a deck)
body       = runtime state (energy, position, memory)
world      = environment
fitness    = survival / task performance / energy
mutation   = edit, replace, reorder, disable, or recombine genes
```

Cell80 is unusually suited to this because the VM already gives inspectable execution: a
report includes result, cycles, halt reason, code size, and touched memory. That means an
organism's behaviour is explainable in terms of *which named genes ran*, not an opaque weight
blob.

## Gene families (starter set)

- **Sensor** — `sense_food_north`, `sense_wall_ahead`, `sense_energy_low`, `sense_neighbor`
- **Metabolism** — `energy_decay`, `eat_food`, `store_energy`, `hunger_threshold`
- **Movement** — `move_forward`, `turn_left`, `turn_towards_food`, `avoid_wall`
- **Decision** — `choose_if_hungry`, `choose_if_blocked`, `argmax_signal`
- **Reproduction** — `can_reproduce`, `split_energy`, `copy_genome`, `mutation_rate`
- **Social** — `emit_signal`, `follow_signal`, `share_energy`, `attack_neighbor`

Genes output *proposals*, not direct world mutations (`move_north`, `eat`, `reproduce`, ...);
the host/world arbitrates legality. This keeps cells pure and matches the existing moat: no
strings, no I/O, no network inside a cell — anything needing that escalates to the host rather
than expanding the ISA.

## Promoters (regulation)

Real DNA isn't just genes, it's also regulation — when does a gene express? Model this as a
promoter: a tiny predicate cell gating a gene.

```
if_hungry.cell      -> seek_food.cell
if_blocked.cell     -> turn_left.cell
if_energy_high.cell -> reproduce.cell
```

A genome is a list of `(promoter, gene)` pairs — regulation without a new VM concept, since a
promoter is just another cell.

## Genome format (sketch)

```json
{
  "id": "grazer.v1",
  "body": { "energy": 800, "max_genes": 32, "max_cycles_per_tick": 5000 },
  "genes": [
    { "promoter": "always.cell",         "gene": "energy_decay.cell" },
    { "promoter": "if_food_near.cell",   "gene": "move_towards_food.cell" },
    { "promoter": "if_on_food.cell",     "gene": "eat.cell" },
    { "promoter": "if_energy_high.cell", "gene": "reproduce.cell" }
  ]
}
```

`.cell` = gene. `.genome` = chromosome (a deck). `.world` = environment.

## Mutation model

Start at gene-composition level, not raw bytecode (raw mutation mostly produces garbage):

- add / remove / replace / reorder / duplicate / disable a gene
- change a gene's declared parameters within its own safe bounds, e.g.

```json
{ "gene": "reproduce_if_energy",
  "params": { "threshold": { "type": "u16", "min": 100, "max": 2000, "default": 800 } } }
```

Lower-level mutation (constants, branch thresholds, instruction substitution/deletion,
crossover at basic-block boundaries) is a later, harder step — only worth it once there's a
corpus of working genomes to mutate.

## Metabolism = cycles

Charging energy per cycle a gene costs means evolution naturally favours cheap genes unless
extra complexity earns its keep. That's a direct reuse of the VM's existing cycle accounting,
not a new mechanic.

## Relation to the trading-card pillar

```
algorithm card  -> individual gene   (.cell)
deck            -> genome            (.genome)
pack            -> gene family
booster         -> mutation / recombination source
tournament      -> ecology / evolution run
```

"Organisms are decks that play themselves."

## Minimal demo shape (if ever prototyped)

```
world:  32x32 grid, food spawns randomly, organisms have energy, one action/tick
genes:  always, if_hungry, if_food_adjacent, if_energy_high,
        move_random, move_towards_food, eat, reproduce, energy_decay

cell80 life run worlds/pond.json genomes/starter_grazer.json --ticks 10000
cell80 life inspect-species 0x91fa
```

## Why it might matter (if pursued later)

- Tests whether complex behaviour emerges from composed tiny deterministic procedures —
  a non-agent version of the core Cell80 thesis.
- Gives an explainable alternative to opaque-policy benchmarks: "it survives because it has
  food-gradient sensing, poison avoidance, and delayed reproduction," not a weight dump.
- Evolved genomes could become curriculum/benchmark generators for other agents.

## Open questions / why this stays fenced

- A world runtime, a data-driven genome file format, and parameter/gene-swap mutation on
  reproduction all exist (1D, single species per run); no multi-species coexistence yet.
- Risks re-litigating the "Z80 for agents" framing the pitch deliberately moved away from —
  needs to stay a showcase, not the headline.
- Should not preempt the type-led index or stdlib growth work.
