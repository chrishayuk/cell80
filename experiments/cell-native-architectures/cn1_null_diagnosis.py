#!/usr/bin/env python3
"""CN-1 null diagnosis: the 6.73x enrichment used null 0.065 (confusion analysis, random over ALL
790 by stored fingerprint); the bias control found 0.1625 (random SAME-ARITY value pairs by
re-execution). Same nominal quantity, 2.5x apart. Two routes disagree => defect to resolve BEFORE
the enrichment number travels. This isolates: (i) do the two agreement FUNCTIONS agree on identical
pairs? (ii) is the gap POPULATION (all-790 incl. state cells vs same-arity value)? (iii) what is the
confusion population actually made of, so we can pick the RIGHT null and recompute enrichment.

Run: python3 cn1_null_diagnosis.py
"""
from __future__ import annotations

import json
import random
import statistics as st
from pathlib import Path

import torch
import cn1_model_hf
from artifact_paths import checkpoint_input, dataset_input
import cell80_py

HERE = Path(__file__).resolve().parent
CELLS_DIR = HERE.parent.parent / "cell80" / "cells"
DEFAULT_PROBES = [
    [3, 7, 12], [7, 3, 1], [0, 0, 0], [1, 1, 1], [5, 5, 9], [2, 9, 5], [10, 3, 7],
    [255, 1, 128], [100, 4, 50], [12, 12, 12], [1230, 0, 2], [65531, 3, 6], [5, 2, 9],
    [9, 5, 2], [2, 8, 4], [4, 2, 4], [7, 0, 0], [12, 3, 4], [9000, 2500, 40], [2, 0, 1],
]


def agree(a, b):
    return sum(1 for x, y in zip(a, b) if x == y) / len(a)


def main():
    lib = {json.loads(l)["name"]: json.loads(l) for l in (HERE / "cn1_library.jsonl").read_text().splitlines() if l.strip()}
    fp = {n: r["fingerprint"] for n, r in lib.items()}  # STORED fingerprint (confusion-analysis source)
    arity = {n: r["arity"] for n, r in lib.items()}
    names = list(lib)
    held = {h["name"] for h in json.loads((HERE / "cn1_axis_a_heldout.json").read_text())["held_out_cells"]}
    rng = random.Random(0)

    # (i) agreement FUNCTION check: stored fp vs re-executed on DEFAULT_PROBES, same pairs
    host = cell80_py.CellHost()
    handles = {}
    for n in names:
        if arity[n] >= 1:
            try:
                host.add_source(n, next(CELLS_DIR.rglob(f"{n}.rs")).read_text()); handles[n] = host.load(n)
            except Exception:
                pass
    def reexec(n):
        a = arity[n]; v = []
        for p in DEFAULT_PROBES:
            try:
                r = host.run(handles[n], p[:a]); v.append(r["result"] if r.get("halt") == "returned" else None)
            except Exception:
                v.append(None)
        return v
    valnames = [n for n in names if n in handles]
    fn_diffs = []
    for _ in range(300):
        x, y = rng.sample(valnames, 2)
        fn_diffs.append(agree(fp[x], fp[y]) - agree(reexec(x), reexec(y)))
    print(f"(i) agreement FUNCTION: mean(stored - reexec) over 300 pairs = {st.mean(fn_diffs):+.4f} "
          f"(~0 => same function; large => the methods differ)")

    # (ii) POPULATION: for held-out value cells, null over ALL-790 vs SAME-ARITY-VALUE (stored fp)
    heldval = [n for n in names if n in held and arity[n] >= 1]
    null_all, null_sav = [], []
    byar = {}
    for n in valnames:
        byar.setdefault(arity[n], []).append(n)
    for T in heldval:
        null_all.append(st.mean(agree(fp[T], fp[c]) for c in rng.sample([n for n in names if n != T], 40)))
        pool = [n for n in byar[arity[T]] if n != T]
        null_sav.append(st.mean(agree(fp[T], fp[c]) for c in rng.sample(pool, min(40, len(pool)))))
    print(f"(ii) NULL population: all-790 {st.mean(null_all):.4f}  |  same-arity-value {st.mean(null_sav):.4f}")

    # (iii) confusion population + enrichment vs the MATCHED null. Small-N model run for speed.
    hf_map = json.loads(cn1_model_hf.HF_TOKEN_MAP.read_text())
    id_to_name = {v: k[6:-1] for k, v in hf_map.items() if k.startswith("<cell:")}
    cell_ids = sorted(id_to_name)
    m, tok, _, _, _, _ = cn1_model_hf.build_hf("fingerprint")
    ck = torch.load(checkpoint_input("cn1_ckpt_hf_fingerprint_s80.pt"), map_location="cpu")
    with torch.no_grad():
        m.base.get_input_embeddings().weight.copy_(ck["embed"])
    m.w_f.load_state_dict(ck["w_f"])
    for i, blk in enumerate(m.base.model.layers[-16:]):
        blk.load_state_dict(ck[f"block_{i}"])
    m.eval()
    cid = torch.tensor(cell_ids)
    ev = [json.loads(l) for l in dataset_input("cn1_corpus_eval.jsonl").read_text().splitlines() if l.strip()]
    ho = [r for r in ev if r["bucket_cell"] == "novel_cell" and r["bucket_comp"] == "seen_comp"][:40]
    conf_ag, samepack, samearity, isstate, conf_null_matched = [], [], [], [], []
    with torch.no_grad():
        for r in ho:
            T = r["cell"]
            ids = torch.tensor([[0] + tok.encode(r["context"] + " <call>", add_special_tokens=False)])
            order = torch.argsort(m(ids)[0, -1][cid], descending=True).tolist()
            ranked = [id_to_name[cell_ids[i]] for i in order]
            conf = ranked[: ranked.index(T)]
            if not conf:
                continue
            conf_ag.append(st.mean(agree(fp[T], fp[c]) for c in conf))
            samearity.append(sum(arity[c] == arity[T] for c in conf) / len(conf))
            isstate.append(sum(arity[c] == 0 for c in conf) / len(conf))
            samepack.append(sum(lib[c]["pack"] == lib[T]["pack"] for c in conf) / len(conf))
            pool = [n for n in byar[arity[T]] if n != T]
            conf_null_matched.append(st.mean(agree(fp[T], fp[c]) for c in rng.sample(pool, min(len(conf), len(pool)))))
    print(f"\n(iii) confusion set (n={len(conf_ag)} held-out cases):")
    print(f"   mean agreement of confusions to true: {st.mean(conf_ag):.4f}")
    print(f"   confusion composition: same-arity {st.mean(samearity):.3f} | state(arity0) {st.mean(isstate):.3f} | same-pack {st.mean(samepack):.3f}")
    print(f"   enrichment vs all-790 null (0.065-ish):        {st.mean(conf_ag)/st.mean(null_all):.2f}x")
    print(f"   enrichment vs SAME-ARITY-VALUE null (matched):  {st.mean(conf_ag)/st.mean(conf_null_matched):.2f}x")
    print(f"   (matched null = {st.mean(conf_null_matched):.4f}; use whichever population the confusions actually occupy)")


if __name__ == "__main__":
    main()
