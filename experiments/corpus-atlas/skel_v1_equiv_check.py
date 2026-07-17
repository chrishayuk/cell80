#!/usr/bin/env python3
"""Equivalence audit: skeleton-v1 vs DIV-0 metrology-M1 on the CN-7 corpus.

Two skeleton definitions now coexist: DIV-0's frozen verdict was graded
with v11-train-plan/div0/metrology.py (M1 = digit->N + deterministic
capitalization-lexicon names->NAME, all else verbatim; M2 = function-word
scaffold), while the atlas's standing instrument is skeleton-v1 (spaCy
NER+PROPN -> N with contiguous collapse, digit-BEARING tokens -> D,
else lowercased). If DIV-1's audit rows get computed by skeleton-v1 while
DIV-0's verdict sits on metrology-M1, they are silently incommensurable.

This audit runs skeleton-v1 over the exact frozen corpus (sha-verified
against DIV-0's provenance) and compares, per frame, the surface
cardinalities against the pinned M1 numbers — S2 = 2 per frame / 12 total
over 25,000 rows — and recomputes the D-B probe coverage with metrology's
own lev_sim/coverage machinery, skeleton-v1 as the renderer. Either the
counts agree and skeleton-v1 inherits the verdict's authority, or the
divergence is documented here BEFORE DIV-1 can trip over it mid-run.
The comparable altitude is M1 (digit/name normalization, content kept);
M2 erases all content words and has no skeleton-v1 analog yet.

Run: .venv-skeleton/bin/python skel_v1_equiv_check.py

VERDICT (first run 2026-07-17, skel_v1_equiv_check.json): NOT EQUIVALENT,
divergence one-sided and fully characterized. S2 comes out 4-6 per frame
(v1) vs pinned 2 (M1), total 31 vs 12; 37 frames diverge, all inflated on
the v1 side. The corpus is exonerated — the raw text has exactly one
warm-up template ("more, so X had", 15,008 rows). Three confirmed
mechanisms, all spaCy-model noise on template text: (1) identical name
surfaces tagged inconsistently across positions ("Lily... Lily" -> N/lily,
reproduced deterministically); (2) entity-span boundary noise absorbing
adjacent words ("gave", "so") for some name fillers; (3) symbol mistags in
call grammar (=, >, cell markers sporadically -> N). Coverage is close but
not identical (D-B2 0.8438 vs 0.8214, D-B5 0.3636 vs 0.3077, rest equal).

CONSEQUENCE: skeleton-v1 does NOT inherit DIV-0's authority on template/
call-bearing corpora — metrology-M1 (deterministic lexicon) remains the
operative normalizer for DIV-1 audit rows and the midtrain wall-checker;
skeleton-v1's domain is the natural-prose pretrain, where its cardinalities
should be read as mild UPPER bounds (splits, never merges). Follow-up
option: an "m1-mode" renderer in the atlas (deterministic lexicon) so DIV
quantities can be computed atlas-side under the frozen definition.
"""

import hashlib
import json
import sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(Path.home() / "chris-source" / "v11-train-plan" / "div0"))

from atlas_skeleton import ENT_N, load_nlp  # noqa: E402
from metrology import coverage, frame_of  # noqa: E402

CORPUS = (HERE.parent / "cell-native-architectures" / "cn7_corpus_train.jsonl")
DIV0_RESULTS = (Path.home() / "chris-source" / "v11-train-plan" / "div0"
                / "results.json")


def render_v1(doc):
    """skeleton-v1 as symbol strings (label-free cardinality comparison)."""
    out = []
    prev_n = False
    for tok in doc:
        if tok.is_space:
            prev_n = False
            continue
        if any(c.isdigit() for c in tok.text):
            out.append("D")
            prev_n = False
        elif tok.pos_ == "PROPN" or tok.ent_type_ in ENT_N:
            if not prev_n:
                out.append("N")
            prev_n = True
        else:
            out.append(tok.text.lower())
            prev_n = False
    return tuple(out)


def main():
    frozen = json.load(open(DIV0_RESULTS))
    sha = hashlib.sha256(CORPUS.read_bytes()).hexdigest()
    assert sha == frozen["provenance"]["sha256"], \
        f"corpus drifted since DIV-0: {sha[:12]} != frozen"
    rows = [json.loads(l) for l in CORPUS.read_text().splitlines() if l.strip()]
    assert len(rows) == frozen["provenance"]["rows"]
    print(f"[equiv] corpus sha OK, {len(rows):,} rows")

    nlp = load_nlp()
    v1_sets = defaultdict(set)
    frames = [frame_of(r) for r in rows]
    for f, doc in zip(frames, nlp.pipe((r["text"] for r in rows),
                                         batch_size=256, n_process=4)):
        v1_sets[f].add(render_v1(doc))
    print(f"[equiv] rendered {len(rows):,} rows into "
          f"{sum(len(s) for s in v1_sets.values())} distinct surfaces")

    pinned = frozen["cardinality_per_frame"]
    table, diverging = [], []
    for f in sorted(v1_sets):
        m1 = pinned[f]["M1"]
        v1 = len(v1_sets[f])
        table.append({"frame": f, "rows": pinned[f]["rows"],
                       "M1": m1, "V1": v1})
        if v1 != m1:
            diverging.append(f)

    s2 = [t for t in table if t["frame"].startswith("s2:")]
    s2_v1_total = len(set().union(*(v1_sets[t["frame"]] for t in s2)))
    print(f"\n{'frame':<28} {'rows':>7} {'M1':>4} {'V1':>4}")
    for t in table:
        mark = "  <-- DIVERGES" if t["frame"] in diverging else ""
        if t["frame"].startswith(("s1:", "s2:")) or mark:
            print(f"{t['frame']:<28} {t['rows']:>7,} {t['M1']:>4} "
                  f"{t['V1']:>4}{mark}")
    print(f"\nS2 pinned: 2/frame, 12 total over 25,000 rows")
    print(f"S2 v1:     {[t['V1'] for t in s2]} -> total {s2_v1_total}")

    # coverage on the frozen D-B probes, metrology's own machinery,
    # skeleton-v1 renderings on both sides
    all_v1 = set().union(*v1_sets.values())
    cov_v1 = {}
    for pid, ptext in frozen["probes"].items():
        cov_v1[pid] = round(coverage(render_v1(nlp(ptext)), all_v1), 4)
    print(f"\n{'probe':<7} {'M1 cov':>8} {'V1 cov':>8}")
    for pid in frozen["coverage"]["M1"]:
        print(f"{pid:<7} {frozen['coverage']['M1'][pid]:>8} "
              f"{cov_v1[pid]:>8}")

    # receipts for divergent frames: the distinct renderings, if few
    examples = {}
    for f in diverging:
        if len(v1_sets[f]) <= 30:
            examples[f] = sorted(" ".join(s) for s in v1_sets[f])

    out = {"corpus_sha256": sha, "rows": len(rows),
            "per_frame": table, "diverging_frames": diverging,
            "s2_v1_total": s2_v1_total,
            "coverage_M1_frozen": frozen["coverage"]["M1"],
            "coverage_V1": cov_v1,
            "divergent_renderings": examples}
    with open(HERE / "skel_v1_equiv_check.json", "w") as f:
        json.dump(out, f, indent=1, ensure_ascii=False)
    print("\n-> skel_v1_equiv_check.json")
    sys.stdout.flush()  # os._exit skips buffer flush; results above matter
    import os
    os._exit(0)  # spaCy n_process leaves workers that stall exit


if __name__ == "__main__":
    main()
