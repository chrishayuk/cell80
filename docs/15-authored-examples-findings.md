# Agent-authored behavioural examples — findings (v1.0, 2026-07-11)

The question this document answers, and the one it deliberately does not blend with it:

> **Oracle question** (checkpoints 21/22): given *correct* I/O examples, can fused
> retrieval find the right cell? — answered: yes, 0.88–0.96 P@1 at 697 cells.
>
> **Deployed question** (this document): given only a natural-language request, can a
> model *invent* examples good enough for that machinery — and what happens when it
> can't?

The distinction matters because the oracle number is a ceiling. Reading 0.9 deployed
P@1 off checkpoint 22 would be claiming arbitrary text queries are solved; they are
not. This benchmark (`cell-eval authored`) measures the full loop and prices each
failure mode separately.

## Method

```
natural-language request (the retrieval dataset's own queries)
    → model authors 1–3 input→output examples
      (sees the request and the intended arity — caller-side intent — never the target cell)
    → fused retrieval (behaviour ranks, declared-arity match then text break ties)
    → selected cell
```

Per case, the harness records:

* **well_formed** — the reply parsed to 1–3 in-range, arity-correct examples;
* **valid** — the *expected* cell reproduces every authored example (the model had to
  compute the outputs itself; a wrong output is a wrong test);
* **the behavioural equivalence class** the examples actually pin down, over every
  register-driveable value cell — from which:
  * **false_unique** — the class is a *singleton that is not the expected cell*: the
    examples confidently identify the wrong tool. This is the dangerous failure (a
    wrong answer with no ambiguity signal), and the metric that must stay ≈ 0;
  * **ambiguous** — expected is *inside* a >1 class: honest, recoverable ambiguity;
* **P@1 three ways** on the same case: plain text, oracle-equipped (the checkpoint-22
  sidecar), and authored.

**Population honesty.** Only cases whose expected cell is a value cell of arity 1–3
are schema-free-authorable — 474 of 1,445 cases (32.8%) at the time of the runs. A
state cell's field names are not knowable from the request alone; the deployed flow
for those is search → inspect → author *field* examples, a two-step lane registered
as follow-up work, not silently folded into these numbers.

## Results

Two authors, same 474 cases, same library, same day
(`cell-eval/baselines/authored-{granite3b,gemma4e4b}-2026-07-11.json`):

| overall (n=474)      | granite4.1:3b | gemma4:e4b |
|----------------------|--------------:|-----------:|
| well_formed          |          0.80 |       0.85 |
| **valid**            |      **0.35** |   **0.56** |
| false_unique_rate    |         0.095 |      0.074 |
| P@1 plain            |          0.63 |       0.63 |
| P@1 oracle           |          0.96 |       0.96 |
| **P@1 authored**     |      **0.45** |   **0.61** |

Per split, authored vs plain:

| split       | plain | granite authored | gemma authored |
|-------------|------:|-----------------:|---------------:|
| direct      |  0.82 |         0.51 ↓↓ |        0.65 ↓ |
| paraphrase  |  0.42 |         0.39 ↓  |    **0.53 ↑** |
| adversarial |  0.44 |         0.41 ↓  |    **0.70 ↑** |

Three findings:

1. **The router was never the problem.** Oracle P@1 is 0.96 on exactly these cases.
   Every point of deployed loss is authoring loss.
2. **Authoring correctness is capability-bound and it scales.** A 3B model computes
   correct outputs for only a third of its own examples, and raw authored retrieval is
   *net harmful* — the fused ranking's monotone guarantee only protects cells that
   reproduce the examples, so a wrong output actively demotes the right cell (direct:
   0.82 → 0.51). At gemma-class capability validity nearly doubles and the lane turns
   net-positive precisely on the splits where text is weak.
3. **false_unique is the failure to engineer against.** ~7–10% of requests get
   examples that confidently pin the *wrong* cell — no ambiguity to detect, no
   ranking to rescue. It must be handled structurally, which leads to:

## The junk guard

At query time — no ground truth needed — check whether **any cell in the library
reproduces every authored example**. If none does, discard the examples and fall back
to plain text.

Two provable properties (verified over both runs):

* it **never discards a valid example set** — the expected cell reproduces valid
  examples by definition, so a valid set always has a full match;
* on these runs it filtered **56% of invalid sets** (the remainder are wrong examples
  that some *other* cell happens to satisfy — the false-unique residue).

Guarded results:

| split       | plain | granite guarded | gemma guarded |
|-------------|------:|----------------:|--------------:|
| direct      | 0.815 |          0.723  |     **0.823** |
| paraphrase  | 0.424 |      **0.490**  |     **0.631** |
| adversarial | 0.444 |      **0.519**  |     **0.741** |
| overall     | 0.631 |          0.614  |     **0.738** |

**At gemma-class capability, guard→fused strictly dominates plain text on every
split, including direct.** At granite-class it is harm-limited (a wash overall,
positive on the hard splits). The guard costs one full-match check — the same
microsecond-scale scan the fused ranking already does.

## The deployed contract

```
examples authored (or supplied)
    → junk guard: some cell reproduces all of them?
        no  → plain text search (examples discarded, harmless)
        yes → fused search (behaviour ranks, arity match then text break ties)
```

This is deployable today at every surface (`cell_search(examples=…)`, CLI, py). Two
compositions raise the ceiling further and are registered in the baselines ledger:

* **margin-gating** — answer high-text-confidence queries from text alone (the
  existing calibrated tier gate) and spend authored examples only on the escalated
  residue, where they are strongly positive;
* **active disambiguation** — when the guard's full-match set has >1 member, execute
  the members until they diverge and resolve the distinguishing input from context (or
  ask), instead of trusting the author's arithmetic further. This also attacks the
  remaining oracle gap (guarded 0.63 vs oracle 0.94 on paraphrase), which is entirely
  authoring headroom: the system can *generate* discriminating probes deterministically
  where the model must otherwise *compute* correct outputs.

## Equivalence-aware reading (applies to the oracle numbers too)

Some "misses" are the system landing on a cell behaviourally indistinguishable from
the expected one under the case's own examples (`min(a,b) ≡ median3(a,b,0)` under
register zero-fill was the canonical case until the arity tie-break dissolved it;
predicate families remain). The oracle eval now reports both readings: at 697 cells,
equipped paraphrase is 0.880 strict / **0.911 equivalence-aware**, with 8.9% of cases
missing *outside* the known class. The strict number stays the gate; the gap between
them is honest ambiguity, not error.

## Artifacts and reproduction

* Harness: `cell-eval/src/cell_eval/authored.py`; offline tests
  `cell-eval/tests/test_authored.py` (scripted author, no network).
* Run: `cell-eval authored --model <name> [--category paraphrase] [--max-cases N]`
  (OpenAI-compatible / Ollama, like the adoption evals).
* Banked runs + the guard analysis: `cell-eval/baselines/README.md` (round-4 section)
  and `authored-*.json` alongside it.
* Registered follow-ups (ledger, leverage order): schema-aware two-step authoring for
  state cells; margin-gated authored retrieval; active disambiguation; equivalence-
  aware metrics as first-class gate numbers; the 1k/2k/5k scaling checkpoints.
