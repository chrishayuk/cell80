# NE-1b: causal weight reversions

**Specification stage. No intervention neural measurements have been run.**

NE-1a found strong temporal representations before adaptation and reader-dependent
changes after adaptation. This follow-up asks what reverting those persistent
weight changes does to two selected forms of accessibility:

* q/global decoding of the symbol two steps back;
* q/conditioned decoding of the next symbol.

The [intervention specification](intervention-spec.md) freezes the experiment.
The [selection inventory](selection-inventory.json) verifies the archived inputs
and deterministically constructs the planned edge subsets **without simulation**.
The specification seal covers that inventory and its generator; it is not an
implementation seal or evidence that an intervention worked.

Each Plastic/Yoked endpoint has 35 branches: unchanged, all changes reverted,
three top-movement fractions, and five replicates of each of two random control
families at each fraction. One shared Frozen reference per seed gives **2,840
planned evaluation score rows**, including both refitted and zero-time transported
readers. Five random draws are technical controls; the experimental unit remains
the seed (ten reused seeds).

Exact movement matching largely forces the largest changes to be selected again.
The inventory reports overlap and how many selections are possible. Count-matched
controls permit wider perturbations but do not control the amount of movement
removed. Neither limitation is hidden behind the phrase “matched random.”

The static evaluation inventory confirms substantial overlap for exact movement
matching (ranges across seeds, arms and draws):

| Fraction restored | Count-control overlap with top set | Movement-control overlap |
|---|---:|---:|
| 1% | 0.0–3.7% | 58.5–97.6% |
| 5% | 5.1–9.4% | 69.6–95.5% |
| 10% | 11.0–15.7% | 72.5–96.9% |

These are subset-overlap statistics, not intervention outcomes. All 418 inherited
sealed files match; the inventory pins 934 imported files and 770 logical endpoint
subsets, including development. Five selector tests pass.

## Verify the specification

From the repository root:

```sh
python3 experiments/neural-ecology-ne1b/select_interventions.py --check
python3 -m unittest discover -s experiments/neural-ecology-ne1b/tests
```

The first command checks the specification seal, inherited code seal, source
checkpoints/readers, and regenerated selection inventory. It does not run neural
trajectories or refit readers. Creating an initial inventory uses `--write`;
existing files cannot be overwritten.

## Next execution stages

Implement and seal a separate runner, validate the complete development matrix
on seed 3001, then seal evaluation code and evidence. Finish every evaluation
fit before generating intervention test trajectories, complete the entire matrix,
and independently audit it before interpretation. Existing NE-1a test data and
findings have already been seen; the barrier does not make this confirmatory.

NE-0c and NE-1a stay unchanged. An internal locally trained downstream population
is a possible NE-2 study, outside this specification.
