# Evolved cells: findings from the first pre-registered run

Companion to `evolved-cells-preregistration.md` (the committed hypothesis/method/success bar)
and `evolved-cells/` (the code). That doc says what would count as a win *before* running;
this one reports what actually happened, including the two real mistakes the run itself caught
— on the theory that a pre-registration only means something if deviations get reported, not
quietly smoothed over.

## Result

**4 of 4 reachable targets passed; the negative control correctly found nothing.** That beats
the pre-registered placeholder bar (2 of 5, is_semiprime excluded from the denominator as the
calibration target) outright.

| target | chain found | full-domain check | fingerprint vs. existing library |
|---|---|---|---|
| `digital_root` | `digit_sum → digit_sum → digit_sum` | PASS (65,536/65,536) | closest = `digit_sum`, agreement 0.833 → **novel** |
| `low_byte_popcount` | `low_byte → popcount` | PASS | closest = `popcount`, agreement 0.833 → **novel** |
| `high_byte_popcount` | `high_byte → popcount` | PASS | closest = `lcm3`, agreement 0.833 → **novel** |
| `rotated_low_byte_popcount` | `rotl12 → high_byte → popcount` | PASS | closest = `popcount`, agreement 0.667 → **novel** |

## Follow-up: the real admission gate, not just the fingerprint check

The result above only tested `Fingerprint`/`DEFAULT_PROBES` in isolation — necessary but, as
the pre-registration said up front, not sufficient, since the real gate also needs retrieval
rows and walks the *whole* library at once. Closed that gap directly: `evolved-cells` now
writes each passing candidate to `evolved-cells/candidates/*.rs` with a proper `//!
summary`/`//! tags:` header (parsed into the manifest by `library_cartridge`, same as any real
cell). A scratch copy of the actual 209-cell `cell80/cells/` plus these 4 candidates, and a
scratch copy of `cell-eval/datasets/retrieval.jsonl` plus 2 retrieval rows per candidate (never
touching the real library or dataset), was fed to the **actual `cell80` CLI binary**:

```
cargo run -p cell80 --bin cell80 -- index <scratch-cells> --gate <scratch-retrieval.jsonl> --json
```

Result: `{"admitted": [...all 213 cells including all 4 candidates...], "refused": []}`. Every
candidate was admitted; nothing was refused — not "would pass the fingerprint check" but the
real `admission::admit` function, on the real current library, via the real CLI, with zero
special-casing. This is the strongest available evidence for the pre-registration's central
claim without literally submitting a PR: the three named caveats (no retrieval rows, no real
gate run, hand-templated codegen) are down to one.
| `is_semiprime` (negative control) | *no chain found* (budget 500,000, depth 5) | — | — |

`agreement` is fraction-of-12-probes-matching; `1.0` is the real admission gate's own
duplicate threshold (`cell80/src/admission.rs::DUPLICATE_AGREEMENT`). Every candidate here
landed at 0.667–0.833 — close enough to be genuinely related to an existing cell, far enough
to not be a behavioral copy of it.

## Two real mistakes, caught by the method working as designed

**1. The first run's `digital_root` chain was wrong, and the full-domain check caught it.**
With the original probe set, A* found `digit_sum → digit_sum` (only 2 applications) — it
satisfied every probe, but the full-domain sweep found 8,075/65,536 mismatches. Reasoning
through it afterward: `digit_sum`'s output range over all `u16` inputs is 0–41 (from `59999`,
not `65535` as a quick guess would suggest), and *some* values in that range — like the `39`
produced by `digit_sum(39999) = 39` — still need a second reduction (`digit_sum(39) = 12`,
still two digits) before a third pass gets to a true single digit (`digit_sum(12) = 3`). Two
passes isn't always enough. The original probe set just didn't happen to include an input that
exposed the gap. Fixed by adding `39999` to the probes (chosen specifically because it forces
the 3-pass requirement, not decoratively) and giving `digital_root` more depth headroom (4→5).
This is the pre-registration's step 4 (full-domain validation) doing exactly the job it was
written for — catching a plausible-looking wrong answer before it could be reported as a win.

**2. The first codegen attempt didn't compile — the restricted dialect rejects block
expressions as values.** `let x = { ...; result };` is not legal in cell80's dialect
("blocks-as-values aren't supported — bind with `let` first"); every multi-statement op
(`digit_sum`, `popcount`, etc.) needs its internal loop *inlined as flat statements* in the
outer function body, not nested in a block bound to a variable. Fixed by rewriting the codegen
templates to emit statement sequences with per-step-unique temp variable names (`v_0`/`s_0`,
`v_1`/`s_1`, ...) instead of one expression-or-block per step. This was caught by actually
trying to compile the first real candidate, not by reasoning about the dialect in advance —
worth naming since the pre-registration described codegen as "real, non-trivial engineering,"
and this is exactly the kind of thing that phrase was hedging against.

## What's genuinely interesting, not just "it worked"

- **`digital_root`'s fingerprint agreement (0.833) is with `digit_sum` itself** — sensible,
  since the candidate *is* digit_sum applied three times. The fingerprint check correctly
  recognizes "closely related to an existing cell" without calling it a duplicate, which is
  exactly the discrimination the real gate needs to make (related ≠ identical).
- **`high_byte_popcount`'s closest match by fingerprint is `lcm3`, not `popcount`** — the
  cell it's actually built from. The 12-probe bank isn't testing "does this look like it's
  related to X," it's testing "does this happen to output the same values as X on these 12
  specific inputs" — a behavioral coincidence, not a structural one. That's a useful reminder
  that "closest fingerprint match" isn't the same question as "what is this composition made
  of," and shouldn't be read as one.
- **`rotated_low_byte_popcount` found a genuinely different composition than the one it was
  designed around.** The target was built from the mental model "rotate left 4, mask the low
  byte, popcount" — A* instead found `rotl12 → high_byte → popcount` (rotating left 12 and
  taking the high byte, rather than rotating left 4 and taking the low byte — the same
  bits end up in the same place either way, just reached differently). The full-domain check
  doesn't care which path got there, only whether the output is right for all 65,536 inputs —
  which is the point: the search found *its own* solution, not a rediscovery of the one already
  in the experimenter's head.

## What this does *not* show

- **The real-admission-gate run used *hand-written* retrieval rows and a *scratch copy* of the
  library, not a real contribution.** The rows were authored by the experimenter (2 per
  candidate, direct + paraphrase, matching the real convention) rather than independently
  reviewed, and the run happened in a throwaway directory, never the real `cell80/cells/` or
  `cell-eval/datasets/retrieval.jsonl`. "The real gate accepts this artifact" is now a checked
  claim; "a human maintainer would merge this" still isn't — aliasing judgment and doc/tag
  quality review are separate, human steps `library-growth.md` describes as part of a real
  contribution, not mechanically gated.
- **Codegen was hand-templated for these specific ops, not general.** Adding a 20th op to the
  pool means writing a 20th `op_stmts` match arm by hand. A general AST-substitution codegen
  system is still unbuilt.
- **No cycle-cost tracking or cost-aware acceptance was used**, per the pre-registration's
  stated scope reduction — these are "smooth" targets, so A*'s natural shortest-chain
  preference stood in for a real cost objective. Whether that substitution holds for harder
  (lossy) targets is untested.
- **n=5 targets, all arity-1, all constructed by the experimenter** (not literally pulled from
  a real backlog — the actual "Next waves" list in `library-growth.md` turned out to have no
  remaining un-duplicated arity-1 gaps when checked). A clean pass here is evidence the
  mechanism works on a small, controlled draw, not a claim about how much of real library
  growth this route could cover.
- **"Evolved" here means "found via search over existing cells," not "grown from nothing."**
  Every candidate's actual logic is built entirely out of pieces that already exist in the
  library — the open philosophical question the pre-registration flagged (efficient
  rediscovery vs. genuine novel authorship) is exactly as open as it was before this run.

## Reproduce it

```
cargo run -p evolved-cells   # writes candidates/*.rs for every passing target

# Real admission-gate check, entirely in a scratch directory:
SCRATCH=/tmp/admission-test
mkdir -p "$SCRATCH/cells"
cp cell80/cells/*.rs "$SCRATCH/cells/"
cp experiments/evolved-cells/candidates/*.rs "$SCRATCH/cells/"
cp cell-eval/datasets/retrieval.jsonl "$SCRATCH/retrieval.jsonl"
# append 1-2 retrieval rows per candidate to $SCRATCH/retrieval.jsonl, then:
cargo run -p cell80 --bin cell80 -- index "$SCRATCH/cells" --gate "$SCRATCH/retrieval.jsonl" --json
```

Targets, the op pool, probes, and budgets are constants/data at the top of `main.rs`.
