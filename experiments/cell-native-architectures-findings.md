# Cell-Native Model Architectures — wave 1 findings (CN-0, CN-2 slice-0)

Status: pilot / slice-0 only. Both experiments are defined in
`cell-native-architectures.md`. This note covers the first concrete run of
each, kicked off 2026-07-12. Code lives in `cell-native-architectures/`.

## Infrastructure map (established before any code was written)

The doc's dependency line names LARQL, the Gemma circuit map, and the L30
injection result as available prerequisites. None of that lives inside
`cell80` — it's spread across sibling repos under `~/chris-source/`:

- **LARQL** (`~/chris-source/larql`, Rust, live, daily commits) — loads
  Gemma 3 4B natively, serves an OpenAI-compatible HTTP API
  (`larql serve <vindex> --port 8080`). `chuk-larql` (Python) is its
  superseded predecessor.
- **The "100% P(target)" / seq-exact 1.00 injection result** is real, from
  `chris-experiments/arithmetic_mechanism/a9_residual_alu.py`
  (`A9_VERDICT.md`) — a bespoke per-decode-step `mlx_lm` loop, not built on
  any packaged tool.
- **`chuk-mcp-lazarus`** (`~/chris-source/chuk-ai/mcp-servers/chuk-mcp-lazarus`)
  is prior-art naming precedent for LARQL's mech-interp primitives, not a
  dependency of either LARQL or the A9 result. Its `extract_activations`/
  `prefill_to_layer` tools are usable for readout but weren't used here —
  the already-proven `chris-experiments/arithmetic_mechanism` taps were
  faster to reuse.
- **cell80-py's `CellHost`** (`load`/`run`/`solve`) is solid and required no
  new plumbing — `host.solve(plan_json)` (the M2 plan-IR path used by
  `cell80/examples/m3_gsm8k_smoketest.rs`) verifies `add`/`sub`/`mul`/`div`
  on u32 operands in ~10ms after JIT, cached thereafter.

## CN-0 slice-0 — operand readout

**Script:** `cell-native-architectures/cn0_operand_readout.py`. Vendors the
`mlx_lm` load + per-layer last-position residual capture from
`arithmetic_mechanism/a1_trace.py`, and the Fourier/helix design-matrix
scaffolding (periods 2/5/10/100) from `a2c_helix_rotation.py` — reused in
the **read** direction (residual → operand value) rather than a2c's
**write** direction (value → injection vector).

**Setup:** Gemma 3 4B (`google/gemma-3-4b-it`, local MLX weights), 4 surface-
form families (`digit`, `word`, `mixed`, `narrative`) × 40 addition problems
each (`a, b ∈ [1, 99]`), residual captured at L12–L22 (straddling the doc's
L13–L21 band), three probe families (linear ridge, Fourier/codebook ridge,
small MLX MLP), scored on **held-out-family** exact-pair recovery (train on
3 families, test on the 4th, rotate) plus a pooled 80/20 random-split
baseline for reference.

**Result** (full run, 33s wall-clock, 160 forward passes):

| held out   | best probe | best layer | exact-pair |
|---|---|---|---|
| digit      | fourier | L22 | 0.150 |
| word       | fourier | L21 | 0.225 |
| mixed      | fourier | L22 | 0.250 |
| narrative  | —       | —   | 0.000 |

Linear and MLP probes sit at floor (0.000, one stray 0.025 hit) at every
layer for every held-out family. The **Fourier/codebook probe** is the only
one that shows a real signal, and it's structured, not noise: near-zero
through L12–16, then rising through L17–22, peaking at the top of the swept
band. This is consistent with `a2c_helix_rotation.py`'s prior finding that
Gemma encodes small numbers via small-period Fourier/clock components
rather than linearly — a probe that respects that structure is the one that
picks up signal here. `narrative` (the "Sam had... found... now has"
surface form) is flatly unrecoverable at this sample size — the operands
sit many tokens before the tap position, unlike the other three families
where they're adjacent to it.

**Read against the gate:** nowhere near the ≥95% gate, and every held-out
family is also below the 80% kill line. **This is not read as a kill.** At
N=40/family (~120 pooled training examples) against a 2560-dim residual,
the linear and MLP probes are almost certainly underpowered rather than
genuinely floor — ridge with an untuned λ=1.0 and an MLP given only ~120
examples both have plausible headroom a larger N would recover. The
Fourier probe's monotonic-ish rise toward L21–22 is the one piece of signal
worth trusting from this pilot: **the next run should sweep further past
L22** (the doc's own band stops at L21; this pilot's peak sits at its
*upper* edge) and scale N by 5–10× before treating the gate/kill as
actually evaluated.

**Not run this slice:** hyperparameter sweep on ridge λ, longer MLP
training, layers beyond L22, multiplication/subtraction (addition only).

## CN-0 rerun — 5× data, tuned λ, layers extended to L28

Addressed all three of slice-0's open items: `N_PER_FAMILY` 40→200 (800
prompts total), ridge `λ` 1.0→0.3, MLP epochs 300→400, layer sweep extended
from L12–22 to L12–28. Results saved separately from the slice-0 pilot
(`cn0_operand_readout_results_slice0_pilot.json` vs the current
`cn0_operand_readout_results.json`).

| held out   | best probe | best layer | exact-pair (was, N=40) |
|---|---|---|---|
| digit      | fourier | L24 | **0.400** (0.150) |
| word       | fourier | L23 | **0.365** (0.225) |
| mixed      | fourier | L23 | **0.575** (0.250) |
| narrative  | fourier | L25 | **0.030** (0.000) |

Every family roughly doubled or better. Linear and MLP are still at floor
everywhere (0.000–0.010) — this isn't a probe-family question, it's
specifically the Fourier/codebook probe carrying the signal, more strongly
now that it has more data to fit against. The peak layer moved from L21–22
to **L23–25**, past the doc's originally-scoped L13–21 band, confirming
last slice's suspicion that the band needed extending, not just re-running.

**The one genuinely new finding: a random-split vs. held-out-family gap.**
The pooled 80/20 random split (train and test both drawn from all 4
families, so the test set can contain surface forms the probe *has* seen
during training) hits **85.6% exact-pair recovery at L23** — close to the
95% gate, and the Fourier probe crosses 60–70% at several nearby layers
too. That's a large gap from the held-out-family numbers above (max 57.5%,
mixed). Read together, these say the operand information is strongly
present in the residual stream and well-localized (L23, respects the
Fourier/clock encoding) *within* a surface-form distribution, but doesn't
transfer *across* surface forms — most acutely for `narrative` phrasing,
which stays at 0–3% at every layer regardless of probe, even with 5× the
data. The mechanism doesn't generalize the way CN-0's gate wants it to; it
memorizes-per-surface-form rather than reading a form-invariant operand
representation.

**Read against the gate:** still below both the 95% gate and the 80% kill
line for every held-out family — narrowly missing "kill" only because the
trend is still visibly climbing with N (not flat), and `mixed` at 57.5% is
within plausible reach of 80% with more data or a wider probe family (a
proper MLP hyperparameter sweep wasn't done — `MLP_HIDDEN`/`MLP_LR` are
still slice-0 defaults). The random-split/held-out-family gap is the more
interesting result than the gate/kill verdict itself: it reframes the open
question from "can operands be read out at all" (yes, clearly, 85.6%) to
"what would it take to make that readout invariant to surface form"
(unanswered — narrative's near-total failure suggests the gap isn't just
about probe capacity).

**Not run this rerun either:** a real hyperparameter sweep (λ and MLP
architecture were single fixed choices, not searched), multiplication/
subtraction, or a dedicated narrative-vs-others contrastive probe design
(the current setup treats all 4 families symmetrically, but narrative's
failure mode looks qualitatively different from the other three's).

## CN-2 slice-0 — verified decoding, real result obtained

**Script:** `cell-native-architectures/cn2_verified_decoding.py`. Sends a
15-problem hand-authored GSM8K-style battery to a running LARQL server,
asks the model to show each arithmetic step as `A op B = C`, extracts every
such span by regex, and independently re-derives it via
`cell80_py.CellHost.solve()` (the same plan-IR path
`m3_gsm8k_smoketest.rs` uses). Verified against synthetic text end-to-end
(extraction + `add`/`sub`/`mul`/`div` verification all correct) before
touching a real server.

**Initial blocker, since fixed in the sibling `larql` repo** (see next
section for the full account): `/v1/chat/completions` hung indefinitely and
`/v1/completions` hung on anything past a trivial 3-token request. Root
causes found, fixed, and verified: a missing request timeout (ported an
existing fix from `/v1/infer`) and an unguarded raw-pointer read in the
default Q4_K hand-asm matvec kernel. A third, deeper concurrency bug in the
custom `spin_pool` thread pool remains — worked around via
`LARQL_SPIN_POOL=0` (routes through `rayon` instead; the file's own docs
call the two paths numerically identical), which the actual run below used.

**Result** (full 15-problem run, `LARQL_SPIN_POOL=0`, zero crashes):

```json
{
  "n_problems": 15, "n_spans": 8,
  "n_match": 8, "n_mismatch": 0, "n_escalated": 0,
  "agreement_rate": 1.0, "wrong_number_rate": 0.0,
  "final_answer_accuracy": 1.0
}
```

Every arithmetic span the model wrote in `A op B = C` form (8 of 15
problems produced at least one — the rest solved without showing intermediate
steps, still landing on a correct final answer) matched cell80's exact
computation. 15/15 final answers were correct. N is small (a slice-0 pilot,
not the full pre-registered GSM8K battery) — read this as "the measurement
pipeline works and the first real numbers are clean," not as a settled
wrong-number-rate baseline. The real CN-2 signal (does injection/resampling
move the needle) needs a larger battery and, per the gate's own design, some
genuinely wrong model arithmetic to correct — this pilot's model happened to
get everything right unaided.

## CN-2 rerun — 60-problem battery, and two real harness bugs found along the way

Extended `BATTERY` from 15 hand-authored problems to 60 (15 + 45
programmatically generated, larger numbers and 3–4-step chains, ground
truth computed in Python rather than by hand — `_gen_battery()` in the
script). Goal: give the model, and CN-2's measurement, something to
actually get wrong — the slice-0 pilot's 15 problems were easy enough that
the model went 15/15.

**That surfaced two real bugs in the harness itself, both worth recording
since they'd silently corrupt any future rerun that didn't catch them:**

1. **Regex span extraction matched chain fragments, not real claims.**
   `SPAN_RE` finds any `A op B = C` substring, so a model line like
   `"437 + 127 + 207 = 771"` (a genuine 3-operand sum) partially matches as
   the 2-operand substring `"127 + 207 = 771"` — real arithmetic (127+207
   = 334) that was never actually claimed to equal 771, so verifying it
   produced a **false-positive mismatch**. Same failure mode, worse, on a
   self-verification decomposition (`"6 * 578 = 6 * (500+70+8) = ... =
   3468"`, correct, flagged wrong) and on a degenerate repetition loop
   (`"359 + 144 = 499 + 1 = 500 + 1 = 500 + 1 = ..."` repeating to the
   token limit). Fixed by rejecting any match with an arithmetic operator
   or `=` immediately adjacent (prefix *or* suffix, skipping whitespace) —
   a genuine standalone equation has neither; a chain fragment has one or
   both. A label prefix like `"Total shirts sold = 45 + 38 = 83"` still
   matches correctly (the character before `45` is `=`, not an operator).
2. **The `SYSTEM` prompt's "A op B = C" phrasing was read as literal text,
   not a placeholder.** On 55/60 completions (including some of the exact
   same problems that worked cleanly in the slice-0 pilot) the model wrote
   lines like `"12 op 7 = 19"` — literally copying the word "op" instead
   of substituting `+`. Fixed by rewording the instruction and adding a
   concrete example (`"write '12 + 7 = 19', not '12 op 7 = 19'"`). Span
   coverage went from 9 verifiable spans (60 problems) to 127 after the
   fix — more than a 10× improvement in what the measurement could
   actually see.

**Final result** (60 problems, `LARQL_SPIN_POOL=0`, fixed harness + fixed
prompt, offline-reprocessed against the saved deterministic completions
once the extraction fix landed so the model didn't need to be re-queried
twice):

```json
{
  "n_problems": 60, "n_spans": 127,
  "n_match": 122, "n_mismatch": 2, "n_escalated": 3,
  "agreement_rate": 0.961, "wrong_number_rate": 0.016,
  "final_answer_accuracy": 0.883
}
```

- **2 genuine caught arithmetic errors** (real signal, not extraction
  artifacts): `68 * 31 = 2088` (correct: 2108) and `1569 + 299 = 1888`
  (correct: 1868).
- **3 escalations, and they're not model errors either** — all three are
  `636 - 710 = -74`-shaped (subtraction producing a negative
  intermediate). The model's arithmetic is actually correct on all three;
  cell80's plan IR is unsigned (`u32`-based) and has no representation for
  a negative intermediate, so it escalates (`needs_wider_math`) rather
  than silently producing a wrong answer. Real, honest coverage gap in the
  verifier, not a wrong-number-rate data point either way — worth fixing
  before CN-2 scales further (a battery with more subtraction-into-negative
  steps would just keep escalating instead of verifying).
- **Final-answer accuracy improved from 80% → 88.3%** between the
  "op"-placeholder run and the fixed-prompt run on the *identical* 60
  problems — forcing genuinely explicit intermediate arithmetic (rather
  than a broken placeholder the model partially ignored) measurably helped
  the model get more final answers right, a secondary but real finding
  about prompting for verified decoding.

**Read against the gate:** `wrong_number_rate = 0.016` is now a real,
trustworthy first baseline number (previously: 0.0 on too-easy problems,
then an artifact-inflated 0.1 before the extraction fix). It's the number
CN-2's eventual injection/resampling build should be compared against —
this slice still doesn't do injection, just measurement.

## LARQL fixes — three real bugs found in the sibling repo, two fixed

Reproducing CN-2's server hang required going into `~/chris-source/larql`
(a separate, actively-developed sibling repo, not part of `cell80`). Full
account, since these are real defects future sessions (in either repo)
should know about:

**Bug 1 — missing request timeout, FIXED.** `/v1/completions` and
`/v1/chat/completions` (`larql-server/src/routes/openai/{completions,chat}.rs`)
did a bare `spawn_blocking(...).await` with no deadline. A slow/stuck
generation call holds `LoadedModel.weights`'s write guard for as long as the
spawned thread runs; with no timeout, every subsequent request queues on
that guard forever — one slow request wedges the whole server. `/v1/infer`
already had this exact fix (`run_infer_with_timeout`, commit `660e6afb`,
"BUG-infer-deadlock §5.6") — it was never ported to the OpenAI-compat
routes. Ported the same `tokio::time::timeout(state.infer_timeout, handle)`
pattern to both. Verified: rebuilt, reinstalled, confirmed clean 200s where
the old binary hung indefinitely.

**Bug 2 — unguarded raw-pointer read in the default Q4_K asm kernel, FIXED.**
The hand-written `asm!` kernel (`q4k_q8k_matvec_asm_v3` and 8 sibling
functions in `larql-compute/src/cpu/ops/q4k_q8k_dot.rs`) is the **default**
matvec path (`LARQL_Q4K_ASM=0` opts out, not in). Its only guard against a
caller passing an activation buffer (`q8k_x.qs`) shorter than the `cols` it's
about to read was `debug_assert_eq!(q8k_x.qs.len(), cols)` — compiled to
nothing in this workspace's release profile (`[profile.release]` never sets
`debug-assertions`). The asm kernel takes `q8k_x.qs.as_ptr()` as a bare
pointer with no length of its own and no per-iteration bound in the asm
block itself — a real unguarded OOB read in production, not a debug-only
guard. Added a real runtime check (`q8k_shape_ok`, zero-fills and returns
early on mismatch, matching the file's own existing convention for the
`w.len() < rows*row_bytes` check right next to it) across all 9 call sites
carrying this pattern (Q4_K scalar/neon/neon_2row/asm/asm_v2/asm_v3, the
fused gate-up neon/asm pair, plus the Q6_K family for consistency, though
Q6_K's own `qs` access turned out to already be bounds-checked). Verified:
full `larql-compute`/`larql-inference`/`larql-server` test suite green
(1240+744+other passed, the two apparent `larql-inference` failures were
pre-existing test-parallelism flakiness on the shared `spin_pool::global()`
singleton, confirmed by re-running in isolation and by a clean full re-run),
all 28 `q4k_q8k_dot` tests including every scalar/neon/asm bit-exact parity
check still pass.

**Bug 3 — a real concurrency bug in `spin_pool`, NOT root-caused despite
extensive effort, hardened + worked around.** Even after both fixes above,
the server still crashed (SIGSEGV) on repeated requests — 3 separate crash
reports, fault addresses `0x2800` (10240, matches `features_per_layer`),
`0xa00` (2560, matches `hidden_size`), and `0x1`, all localizing to
`larql_compute::cpu::spin_pool`'s own dispatch closure
(`par_chunks_mut`/`run_chunks`), not the matvec kernels. Hardened one real,
provable defect found in that file — `chunk.min(total - start)` at
`spin_pool.rs:349`/`:384` is an **unchecked subtraction** that silently
wraps to a huge `usize` in a release build if `start` ever exceeds `total`,
feeding a wild length into `from_raw_parts_mut`; changed to `saturating_sub`
with a zero-length early-return.

Went further to try to actually root-cause it: wrote two new stress tests
(`stress_realistic_decode_shape_no_corruption`,
`stress_concurrent_realistic_decode_shape_no_corruption`) that exercise the
exact public `par_chunks_mut` entry point at the real gemma-3-4b-it
dimensions — sequential at production scale (108K+ dispatches), then
genuinely concurrent (6 threads, varying dispatch shapes, ~98K dispatches),
then again with artificial per-element busy-work to match the real asm
kernel's timing (ruling out a spin/yield/park timing dependency). All
clean, zero corruption, across every variant. A companion audit (separate
agent) traced the full mmap ownership chain from vindex file on disk to
the `&[u8]` slices the kernels read and found no unmap/reload/UAF hazard —
the vindex loads once at startup before the listener binds, and the
timeout-drop pattern from bug 1 doesn't dangle any reference (the abandoned
thread owns real `Arc` clones). Also audited every other `par_chunks_mut`
call site in the codebase for the same unguarded-pointer shape as bug 2 —
found none. **Conclusion: the bug resists both static analysis and
synthetic reproduction; finding it needs a live debugger session on an
actual crash (`lldb` attach + repeated requests until it faults), which
this session didn't have set up.** Root cause is open.

**Empirically confirmed mitigation:** `LARQL_SPIN_POOL=0` (documented in the
file's own header comment as routing through `rayon` instead, "either way
the arithmetic is identical — only *which threads run which chunks*
differs") ran the full 15-request CN-2 battery with zero crashes.

**State of the sibling repo: committed and pushed** (explicit request,
2026-07-12), three scoped commits on `origin/main`:
- `600dcc66` — bug 1 (timeout)
- `addbc267` — bug 2 (unguarded asm pointer)
- `7e5b84b8` — bug 3 hardening + the two new stress tests, commit message
  is explicit that root cause remains open

## Wave 1 status: both experiments have real, well-powered first results

CN-0's 5× rerun and CN-2's 60-problem rerun (above) are both done. Neither
was a one-shot — CN-0 needed the sample-size/λ/layer-range tuning from the
slice-0 pilot's own recommendations, and CN-2 needed two real harness bugs
(regex chain-matching, the "op" placeholder prompt) fixed mid-flight before
its numbers were trustworthy. Worth internalizing for whoever runs CN-1/
CN-3 next: a slice-0 pilot's job is exactly this — finding the bugs in the
measurement apparatus itself before treating its numbers as science.

## Immediate next steps (not yet done)

1. Root-cause `spin_pool`'s remaining concurrency bug (bug 3) for real —
   needs a live debugger session on an actual crash, not more static or
   synthetic work (both were tried extensively and came up clean).
2. CN-0: a real hyperparameter sweep (λ and MLP architecture were single
   fixed choices even in the rerun), multiplication/subtraction beyond
   addition, and a dedicated narrative-vs-others contrastive probe design —
   narrative's near-total failure (0–3% at every layer, both runs) looks
   qualitatively different from the other three families', not just a
   smaller-N version of the same gap.
3. CN-2: fix the plan-IR's unsigned-only limitation (the 3 escalations
   above are the model being *right* and cell80 being unable to verify a
   negative intermediate) before scaling the battery further, or the
   escalation rate will just grow with it. Then this is ready for the real
   G2 build (verified decoding *with* resampling on mismatch) — slice-0/
   rerun only measured, never corrected.
4. Wave 2 (CN-1's H1 factory, CN-3's prosthetic) hasn't been scoped yet.
