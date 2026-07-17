#!/usr/bin/env python3
"""corpus-atlas cross-instrument check: retrodict CN-10's digit-prior ranks.

CN-10's readout smoke (cn10_readout_smoke_raw_v11.json, gate machinery on
raw v11) found answer-digit ranks at the final position tracking digit
identity and attributed this to register statistics. Two-routes check: the
atlas counts each bare digit piece's unigram frequency in the materialized
24M-token pretrain stream and asks whether corpus log-frequency retrodicts
the smoke's median digit rank (Spearman). Agreement = two independent
instruments (behavioral readout, corpus enumeration) measuring the same
prior; the delta-from-prior correction then rests on a corpus-grounded
prior, not a purely behavioral one.

Ranks in the smoke are of the BARE digit pieces (sp.PieceToId(d)). Two
statistics are computed:

1. Global unigram count (the naive prior) — pre-specified check.
2. NUMBER-INITIAL count (occurrences whose previous token is not a digit
   piece) — the conditioning that matches what the smoke measured: its ref
   token is the FIRST digit-bearing piece of the answer, i.e. a
   numeral-run-initial position. Receipts motivated this: '1' lives
   number-initially (10/12/100/911 counting register), '0' number-finally
   (10/20/30 — only 3 initial occurrences in 24M tokens), '3' is inflated
   by the frame-bound "a 3 year old" idiom.

First run 2026-07-17 (tie-averaged Spearman): unigram rho = -0.37 (weak,
right sign); number-initial rho = -0.77 all digits, -0.87 excluding '3'.
The '3' residual is itself informative: 1,992 idiom-bound occurrences do
not transfer to the post-'=' slot — the behavioral prior is frame-
sensitive, not unigram.

DESIGN RULING for CN-10 (settled by the '3' outlier): the behavioral prior
(shuffled-rank control) stays the OPERATIVE delta-from-prior correction;
the corpus count is its validating second route, not its replacement. The
model's prior at the post-'=' position is conditioned on more context than
any position-binned corpus count can capture — delta-from-corpus-prior
would import exactly the residual the idiom exposes. Corpus count as
receipt, shuffled rank as instrument.
"""

import json
from pathlib import Path

import numpy as np
import sentencepiece as spm

HERE = Path(__file__).resolve().parent
CN_DIR = HERE.parent / "cell-native-architectures"
V11_SP = (Path.home() / "chris-source" / "chris-experiments" / "compilation"
          / "15_v11_model" / "v11_tokenizer" / "v11.model")


def _avg_ranks(x):
    x = np.asarray(x, dtype=float)
    order = np.argsort(x)
    ranks = np.empty(len(x))
    i = 0
    while i < len(x):
        j = i
        while j + 1 < len(x) and x[order[j + 1]] == x[order[i]]:
            j += 1
        ranks[order[i:j + 1]] = (i + j) / 2.0  # ties get the average rank
        i = j + 1
    return ranks


def spearman(x, y):
    return float(np.corrcoef(_avg_ranks(x), _avg_ranks(y))[0, 1])


def main():
    sp = spm.SentencePieceProcessor()
    sp.load(str(V11_SP))

    counts = np.zeros(71261, dtype=np.int64)
    for f in ["v11_stream_phase1_run1.u32", "v11_stream_phase3_run1.u32"]:
        ids = np.fromfile(HERE / f, dtype=np.uint32)
        counts += np.bincount(ids, minlength=71261)

    smoke = json.load(open(CN_DIR / "cn10_readout_smoke_raw_v11.json"))
    tables = [r["digit_ranks_final"] for r in smoke["rows"]]

    digits = list("0123456789")
    med_rank, bare_ct, sp_ct = {}, {}, {}
    for d in digits:
        med_rank[d] = float(np.median([t[d] for t in tables]))
        bare_ct[d] = int(counts[sp.PieceToId(d)])
        pid = sp.PieceToId("▁" + d)
        sp_ct[d] = int(counts[pid]) if pid != sp.unk_id() else None

    ids = np.concatenate([np.fromfile(HERE / f, dtype=np.uint32)
                           for f in ["v11_stream_phase1_run1.u32",
                                     "v11_stream_phase3_run1.u32"]])
    pids = {d: sp.PieceToId(d) for d in digits}
    is_digit = np.isin(ids, list(pids.values()))
    prev_digit = np.roll(is_digit, 1)
    prev_digit[0] = False
    initial = is_digit & ~prev_digit
    init_ct = {d: int((initial & (ids == p)).sum()) for d, p in pids.items()}

    # higher corpus count should mean better (lower) rank -> negative rho
    rho = spearman([bare_ct[d] for d in digits], [med_rank[d] for d in digits])
    rho_init = spearman([init_ct[d] for d in digits],
                         [med_rank[d] for d in digits])
    no3 = [d for d in digits if d != "3"]
    rho_init_no3 = spearman([init_ct[d] for d in no3],
                              [med_rank[d] for d in no3])

    print(f"{'digit':>5} {'unigram count':>13} {'number-initial':>14} "
          f"{'median smoke rank':>18}")
    for d in sorted(digits, key=lambda d: med_rank[d]):
        print(f"{d:>5} {bare_ct[d]:>13,} {init_ct[d]:>14,} {med_rank[d]:>18.1f}")
    print(f"\nSpearman(unigram, rank)        = {rho:+.3f}")
    print(f"Spearman(number-initial, rank) = {rho_init:+.3f}  "
          f"(excluding '3' idiom: {rho_init_no3:+.3f})")

    out = {
        "stream_files": ["v11_stream_phase1_run1.u32", "v11_stream_phase3_run1.u32"],
        "smoke_source": "cn10_readout_smoke_raw_v11.json",
        "per_digit": {d: {"bare_count": bare_ct[d], "space_count": sp_ct[d],
                            "number_initial_count": init_ct[d],
                            "median_smoke_rank": med_rank[d]} for d in digits},
        "spearman_unigram_vs_rank": rho,
        "spearman_initial_vs_rank": rho_init,
        "spearman_initial_vs_rank_excl_3": rho_init_no3,
    }
    with open(HERE / "retro_digit_prior.json", "w") as f:
        json.dump(out, f, indent=2)
    print(f"-> retro_digit_prior.json")


if __name__ == "__main__":
    main()
