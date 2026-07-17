# CN-8 pre-grade observations — for the grading session, NOT grades

2026-07-17, written while the eval battery was still running (5/7 done:
b_s80 trace, atok_s80/s81 answer, aex_s80 answer, raw answer; b_s81 trace
in flight, raw trace queued). Thresholds are frozen in the CN-8 prereg and
grading belongs to the session that owns it; these are two observations
from the raw log worth an eyeball before the verdict is written, plus
pointers. Numbers below are copied from `cn8_eval_run.log`, not recomputed.

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
