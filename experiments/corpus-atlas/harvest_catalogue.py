#!/usr/bin/env python3
"""corpus-atlas harvest (spec §6): the frame catalogue + the DIV-1
calibration histogram + distance scores for CN-8's frozen eval bands.

Three products in one deterministic pass:

1. v1 FRAME CATALOGUE — sentence-level skeletons cut from the built
   skeleton stream (split at ./!/? symbols, never across chunk
   sentinels), deduped, with occurrence counts per phase, distinct
   D/N-filler-tuple counts (via the alignment map back to original
   pieces), frame type (declarative/exclamative/interrogative from the
   final punct), and a first-occurrence receipt. Rendered under
   skeleton-v1, so counts read as mild upper bounds per the equivalence
   audit (v1 splits, never merges).

2. m1-MODE CROSS-CHECK — the same harvest under DIV-0's deterministic
   metrology-M1 normalizer (imported from v11-train-plan/div0, name
   lexicon rebuilt from the pretrain text itself), so the calibration
   histogram's SHAPE is confirmed by the normalizer that holds authority
   on template text. Two renderers, one distribution — the two-routes
   rule applied to the harvest.

3. CN-8 BAND DISTANCES — two-distance profiles (surface + skeleton) for
   the frozen B0/B1/B2 prompts ("{a} + {b} ="), committed BEFORE the
   CN-8 grading verdict exists so the eventual exact-vs-distance curve
   is demonstrably untuned. Scoring prompts, not grading generations.

The histogram is the empirical answer to "where do DIV-1's 1/8/64/~512
diversity levels sit relative to what the pretrain actually contains" —
per-frame occurrence and filler-diversity distributions with those levels
located as percentiles.

Run: .venv-skeleton/bin/python harvest_catalogue.py
"""

import json
import sys
from collections import Counter, defaultdict
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(Path.home() / "chris-source" / "v11-train-plan" / "div0"))

from atlas_skeleton import (D_SYM, N_SYM, SkeletonIndex,  # noqa: E402
                             chunk_to_text_spans)
from atlas_surface import CHUNK, PHASES, SENTINEL, V11_SP  # noqa: E402

CN_DIR = HERE.parent / "cell-native-architectures"
DIV1_LEVELS = [1, 8, 64, 512]


def pctile_of(sorted_counts, level):
    """Percentile of `level` within the distribution of counts."""
    import bisect
    return round(100.0 * bisect.bisect_right(sorted_counts, level)
                 / len(sorted_counts), 1)


def hist_summary(counts):
    cs = sorted(counts)
    n = len(cs)
    q = lambda p: cs[min(n - 1, int(p * n))]
    return {"frames": n,
            "hapax_share": round(sum(1 for c in cs if c == 1) / n, 4),
            "quantiles": {"p50": q(.50), "p90": q(.90), "p99": q(.99),
                           "max": cs[-1]},
            "div1_levels_as_percentile": {str(l): pctile_of(cs, l)
                                            for l in DIV1_LEVELS}}


def harvest_v1(idx):
    frames = {}  # tuple -> [count, filler-set, phase-counter, receipt]
    punct = {idx.vocab.get(p) for p in (".", "!", "?")} - {None}
    all_ids = {p: np.fromfile(HERE / PHASES[p], dtype=np.uint32
                                ).reshape(-1, CHUNK) for p in PHASES}
    for p in PHASES:
        stream, chunk_of, pspan = idx.stream[p], idx.chunk_of[p], idx.pspan[p]
        sent_start = 0
        for i in range(len(stream)):
            s = int(stream[i])
            if s == int(SENTINEL) or s in punct:
                end = i + (0 if s == int(SENTINEL) else 1)
                if end - sent_start >= 2:
                    frame = tuple(int(x) for x in stream[sent_start:end])
                    fillers = []
                    for k in range(sent_start, end):
                        if stream[k] in (D_SYM, N_SYM):
                            ci = int(chunk_of[k])
                            p0, p1 = (int(x) for x in pspan[k])
                            fillers.append(idx.sp.decode(
                                [int(t) for t in all_ids[p][ci][p0:p1]]))
                    rec = frames.get(frame)
                    if rec is None:
                        ci = int(chunk_of[sent_start])
                        frames[frame] = [1, {tuple(fillers)}, Counter([p]),
                                          (p, ci)]
                    else:
                        rec[0] += 1
                        rec[1].add(tuple(fillers))
                        rec[2][p] += 1
                sent_start = i + 1
    return frames


def harvest_m1(sp):
    from metrology import SENT_END, build_name_lexicon, norm_m1, tokens
    texts = []
    for p in PHASES:
        ids = np.fromfile(HERE / PHASES[p], dtype=np.uint32).reshape(-1, CHUNK)
        texts.extend(chunk_to_text_spans(sp, row)[0] for row in ids)
    names = build_name_lexicon(texts)
    frames = Counter()
    fillers = defaultdict(set)
    for text in texts:
        toks = tokens(text)
        norm = norm_m1(text, names)
        start = 0
        for i, t in enumerate(toks):
            if t in SENT_END:
                if i + 1 - start >= 2:
                    fr = norm[start:i + 1]
                    frames[fr] += 1
                    fillers[fr].add(tuple(
                        toks[k] for k in range(start, i + 1)
                        if norm[k] in ("N", "NAME")))
                start = i + 1
    return frames, fillers, len(names)


def score_cn8_bands(idx):
    from atlas_surface import SurfaceIndex
    surf = SurfaceIndex()
    bands = json.loads((CN_DIR / "cn8_eval_problems.json").read_text())
    out = {}
    for band, probs in bands.items():
        rows = []
        for a, b in probs:
            text = f"{a} + {b} ="
            sp_prof = surf.profile(text, receipts_for_max=False)
            sk_prof_syms, _ = idx.encode_probe(text)
            sk_max = {p: 0 for p in PHASES}
            for i in range(len(sk_prof_syms)):
                for p in PHASES:
                    l, _, _ = idx.longest_match_at(p, sk_prof_syms, i)
                    sk_max[p] = max(sk_max[p], l)
            rows.append({"a": a, "b": b,
                          "surface_max": sp_prof["max_match"],
                          "skeleton_max": sk_max})
        out[band] = {
            "n": len(rows),
            "surface_max_mean": {p: round(float(np.mean(
                [r["surface_max"][p] for r in rows])), 2) for p in PHASES},
            "skeleton_max_mean": {p: round(float(np.mean(
                [r["skeleton_max"][p] for r in rows])), 2) for p in PHASES},
            "rows": rows,
        }
    return out


def main():
    idx = SkeletonIndex()

    print("[harvest] v1 frame catalogue…", flush=True)
    v1 = harvest_v1(idx)
    v1_counts = [rec[0] for rec in v1.values()]
    v1_fillers = [len(rec[1]) for rec in v1.values()]
    print(f"  {len(v1):,} distinct v1 frames")

    inv = idx.inv
    top = sorted(v1.items(), key=lambda kv: -kv[1][0])[:10]
    catalogue = HERE / "harvest_catalogue_v1.jsonl"
    with open(catalogue, "w") as f:
        for frame, (count, fill, phases, receipt) in v1.items():
            f.write(json.dumps({
                "skeleton": " ".join(inv.get(s, "?") for s in frame),
                "count": count, "distinct_fillers": len(fill),
                "phase_counts": dict(phases),
                "frame_type": {"." : "decl", "!": "excl", "?": "interr"}.get(
                    inv.get(frame[-1], ""), "unpunct"),
                "provenance": {"harvested": True, "phase": receipt[0],
                                "chunk": receipt[1]},
            }, ensure_ascii=False) + "\n")
    print(f"  -> {catalogue.name} (untracked; derived)")

    print("[harvest] m1-mode cross-check…", flush=True)
    m1_frames, m1_fillers, lex = harvest_m1(idx.sp)
    m1_counts = list(m1_frames.values())
    m1_fill = [len(m1_fillers[f]) for f in m1_frames]
    print(f"  {len(m1_frames):,} distinct m1 frames (name lexicon {lex:,})")

    print("[harvest] CN-8 band distances…", flush=True)
    cn8 = score_cn8_bands(idx)
    with open(HERE / "cn8_band_distances.json", "w") as f:
        json.dump({"note": ("two-distance scores of the frozen CN-8 eval "
                              "prompts, committed before any grading verdict; "
                              "scores prompts, grades nothing"),
                    "bands": cn8}, f, indent=1)
    for band in cn8:
        print(f"  {band}: surface_max_mean {cn8[band]['surface_max_mean']} "
              f"skeleton_max_mean {cn8[band]['skeleton_max_mean']}")

    summary = {
        "v1": {"occurrences": hist_summary(v1_counts),
                "distinct_fillers": hist_summary(v1_fillers),
                "head_frames": [{
                    "skeleton": " ".join(inv.get(s, "?") for s in fr)[:120],
                    "count": rec[0], "distinct_fillers": len(rec[1])}
                    for fr, rec in top]},
        "m1": {"occurrences": hist_summary(m1_counts),
                "distinct_fillers": hist_summary(m1_fill),
                "name_lexicon_size": lex},
        "reading": ("div1_levels_as_percentile locates DIV-1's seeded "
                     "frame-cardinality levels against the pretrain's own "
                     "frame-diversity distribution; v1 counts are mild upper "
                     "bounds (equivalence audit), m1 is the "
                     "authority-holding cross-check"),
    }
    with open(HERE / "harvest_summary.json", "w") as f:
        json.dump(summary, f, indent=1)
    print(json.dumps({k: summary[k]["occurrences"] for k in ("v1", "m1")},
                      indent=1))
    print("-> harvest_summary.json, cn8_band_distances.json")
    sys.stdout.flush()
    import os
    os._exit(0)


if __name__ == "__main__":
    main()
