# NE-0c execution interruption and recovery

The sealed evaluation began after commit `effb845`. On 2026-09-17 UTC,
the driver stopped during secondary continuation B for seeds 4004–4006.
All primary phases, probes, scores and persistence checks for seeds 4001–4006
had completed; seeds 4001–4003 had also completed their secondary branches
and archival.

The filesystem reported only 135 MiB free when inspected after the stop.
The three continuation subprocesses exited with code 1 at approximately the
same time. The final terminal output was truncated, so the precise subprocess
error text was not retained: disk exhaustion is the supported explanation,
not a directly captured ENOSPC error. The journal also contains queued A starts
for seeds 4007–4009 with no exits or output directories. No NE-0c worker remained
in the process list. No completed phase summary reported a backend mismatch.

## Preserving the attempts

[interruption-record.json](interruption-record.json) captures the original
journal hash, unmatched starts, completed primary cohort and every partial
continuation file's size and hash. Each incomplete continuation directory was
renamed to `diagnostics/interrupted-continuation-attempt-1` within its seed.
No primary result or incomplete output was overwritten.

The inherited lossless archiver ran on seeds 4004–4006 after their primary
scoring and persistence checks. This recovered approximately 1.3 GB. It retained
original/compressed hashes and checked a decompression roundtrip for each file,
including interrupted outputs. Free space subsequently increased to about
371 GiB, without this task deleting unrelated files.

## Exact-configuration completion

The preregistration permits explicitly logged reruns after interruption.
The [recovery driver](recovery-driver.py), written after interruption and outside the sealed scientific implementation, calls the original sealed command wrapper, binary,
probe, archiver and analysis. It completes continuation B/fit/C/score/archive
for 4004–4006 without repeating primary phases or fitting primary probes.
It compares every complete preserved B tick-log line against the rerun prefix.
It then invokes the sealed `one_seed` procedure for 4007–4010 and the sealed
cohort analysis. Seed workers run sequentially during recovery, within the
registered maximum of three independent workers. A free-space guard runs before
each seed. This changes scheduling and recovery orchestration only.

[recovery-progress.json](recovery-progress.json) records the recovery script
hash, completed stages and interrupted-prefix checks. The original attempt
journal retains all failed and unmatched entries alongside the successful
reruns. No source, binary, protocol, scientific parameter, seed, outcome rule
or probe-fitting procedure was changed. The final mechanical audit must check
these records explicitly rather than asserting that the execution had no retries.
