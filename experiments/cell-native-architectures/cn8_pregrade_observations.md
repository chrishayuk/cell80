# CN-8 pre-grade observations — for the grading session, NOT grades

2026-07-17. **UPDATE: battery complete (ALL EVALS DONE), all 7 evals
written.** Started while 5/7 were done; the two remaining (b_s81 trace,
raw trace) have landed and they *resolve* observation 1's check — see the
resolution block below it. Thresholds are frozen in the CN-8 prereg and
grading belongs to the session that owns it; these are eyeball pointers
plus one now-answered check. Numbers below are read from the eval jsons,
not recomputed.

## 1. Arm B's B1 failure is uniform in error class — check for one
## systematic defect before reading it as capability collapse

B s80 trace: B0 exact 1.000 (tf_nll 0.000); B1 exact 0.000 with **all 200
first errors in class `index`** and tf_nll 0.970; B2 exact 0.000, tf_nll
2.453, again index-class 200/200.

Care with the NLL phrasing: 0.970 is low *relative to A-tok's 6.06 on the
same band*, but it is not near-in-distribution in absolute terms — B's own
in-range traces score 0.000. The defensible statement is intermediate:
oracle 5-digit traces are vastly more familiar to B than oracle answers
are to A-tok, while still clearly off B's trained manifold. The uniform
`index` class carries the systematic-defect hypothesis on its own (e.g.
loop/index bookkeeping going off-range beyond the trained band); a handful
of B1 traces from `cn8_eval_b_s80_trace.json` would distinguish
one-mechanical-defect from collapse. The two readings imply different
CN-9/CN-10 follow-ups.

### RESOLUTION (battery complete) — the eval fields already answer it,
### and it replicates across seeds

The trace grader records `index_ok` (operand digits correctly labelled at
every position of the "i" line) and `col_ok`/`col_n` → `col_cond_correct`
(per column, conditional on the model's OWN index labels: add table, carry
propagation, and accumulator copy all internally correct). These separate
"transcribed the operands right" from "ran the algorithm right." What the
files show, both seeds:

| eval | band | exact | index_ok | col_cond_correct | tf_nll | first_err |
|---|---|---|---|---|---|---|
| B s80 | B1 | 0.000 | 0.000 | **1.000** | 0.970 | index 200/200 |
| B s81 | B1 | 0.000 | 0.000 | **1.000** | 0.950 | index 200/200 |
| B s80 | B2 | 0.000 | 0.000 | **1.000** | 2.453 | index 200/200 |
| B s81 | B2 | 0.000 | 0.000 | **1.000** | 2.307 | index 200/200 |
| raw   | B0–B2 | 0.000 | 0.000 | **None** | 9.5–10 | index 200/200 |

Reading (fields only, not a grade): arm B's `index` failure is **not**
capability collapse — `col_cond_correct = 1.000` means every column's
arithmetic (table + carry + accumulate) is flawless; the model runs its
addition algorithm perfectly over whatever operand layout it indexed. The
sole structural break is `index_ok = 0` — it cannot lay out 5/6-digit
operands at their positions beyond the 4-digit trained band. That is a
transcription/length-generalization defect in the operand-binding step,
with the compute step intact — one systematic defect, confirmed, not
collapse; and it replicates across seeds near-identically (col_cond 1.000
both, tf_nll 0.97/0.95).

The "index 200/200" first-error label is **shared with raw v11 but means
the opposite thing**: raw has `col_cond_correct = None` (it never emits a
valid column segment at all; nothing to condition on) and truncates 19–41×
per band. Same label, opposite mechanism — exactly the trap the flag
warned of. Whoever grades should not read B's and raw's shared `index`
class as the same failure.

Tie-in (for the grader to weigh, not decided here): "computation
generalizes, operand-binding does not" is the marshalling-not-fluency
dissociation showing up inside CN-8's own trace grader — the same split
the corpus-atlas resolution adopted today (`corpus-atlas-DRAFT.md`), now
visible as index_ok=0 / col_cond=1 rather than as a corpus statistic.

## 2. A-tok's B2 teacher-forced NLL is WORSE than the raw floor's on the
## same oracle answers under the same harness

A-tok B2 tf_nll: **9.912 (s80), 8.607 (s81)**. Raw v11 B2 tf_nll:
**5.867**. A-ex B2: 8.033 — also above raw. (B1 straddles: A-tok 6.059
(s80) above raw's 5.962, but 4.640 (s81) below; the clean signal is B2,
where both A-tok seeds and A-ex sit well above the raw floor.)

Raw v11, which never saw the answer format, assigns correct 6-digit
answers *higher* likelihood than models trained on ~6M tokens of exactly
this task. If that survives the remaining evals and the frozen grading,
it is anti-generalization: the drill actively suppressed out-of-range
answer likelihood below the untrained baseline — the collapse-basin /
low-diversity-sharpening story arriving as a signed number in CN-8's own
log, and the strongest empirical motivation yet for DIV-1's diversity
curve. Arm B's B2 tf_nll (2.453) sits far *below* the raw floor on its
format, for contrast.

Status of both: eyeball-and-hand-to-the-grader. Nothing here grades a
band, moves a threshold, or reads across arms beyond what the log
literally says.
