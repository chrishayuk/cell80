#!/usr/bin/env python3
"""CN-1 top-k confusion analysis: on held-out cells, WHAT is beating the true cell?

The plateau shape (held-out median rank ~21, 88% top-10%, top-5 0.18, top-1 0.000) reads as
"the model learned the neighbourhood and can't pick within it", not "almost converging". If the
mechanism is behaviour-as-address, the cells ranked ABOVE the true one should be its behavioural
siblings — because fingerprints place behaviourally-similar cells near each other (the same
property that causes the seen-cell inversion). If so, top-1 is the wrong bar: the model locates a
behavioural *neighbourhood*, and the runtime disambiguates by execution (the shipped fused router,
0.859). If instead the confusions are junk, it's a capacity problem and the top-1 story stands.

Metric: behavioural similarity (cell80's fingerprint *agreement* = fraction of the 20 probes where
two cells return the same value) between the true held-out cell T and the cells the model ranks
above it, vs. a random-cell null; plus same-pack rate vs base rate. Runs on the existing (norm-less
but faithful-geometry) fingerprint checkpoint, on CPU.

Run: python3 cn1_confusion_analysis.py
"""
from __future__ import annotations

import json
import statistics as st
from pathlib import Path

import torch

import cn1_model_hf

HERE = Path(__file__).resolve().parent


def agreement(a, b):
    """cell80-native behavioural similarity: fraction of the 20 probes where both cells return the
    same value (None==None agrees; None vs value disagrees)."""
    return sum(1 for x, y in zip(a, b) if x == y) / len(a)


def main():
    lib = {json.loads(l)["name"]: json.loads(l) for l in (HERE / "cn1_library.jsonl").read_text().splitlines() if l.strip()}
    fp = {n: r["fingerprint"] for n, r in lib.items()}
    pack = {n: r["pack"] for n, r in lib.items()}
    hf_map = json.loads(cn1_model_hf.HF_TOKEN_MAP.read_text())
    id_to_name = {v: k[len("<cell:"):-1] for k, v in hf_map.items() if k.startswith("<cell:")}
    cell_ids = sorted(id_to_name)

    m, tok, names, held, cfi, base_rows = cn1_model_hf.build_hf("fingerprint")
    ck = torch.load(HERE / "cn1_ckpt_hf_fingerprint_s80.pt", map_location="cpu")
    with torch.no_grad():
        m.base.get_input_embeddings().weight.copy_(ck["embed"])
    m.w_f.load_state_dict(ck["w_f"])
    for i, blk in enumerate(m.base.model.layers[-16:]):
        blk.load_state_dict(ck[f"block_{i}"])
    m.eval()
    cell_ids_t = torch.tensor(cell_ids)

    ev = [json.loads(l) for l in (HERE / "cn1_corpus_eval.jsonl").read_text().splitlines() if l.strip()]
    heldout = [r for r in ev if r["bucket_cell"] == "novel_cell" and r["bucket_comp"] == "seen_comp"][:200]

    rng = __import__("random").Random(0)
    agree_conf, agree_null, samepack_conf, base_rate = [], [], [], []
    ranks = []
    examples = []
    with torch.no_grad():
        for r in heldout:
            T = r["cell"]
            ids = torch.tensor([[0] + tok.encode(r["context"] + " <call>", add_special_tokens=False)])
            logits = m(ids)[0, -1][cell_ids_t]
            order = torch.argsort(logits, descending=True).tolist()
            ranked_names = [id_to_name[cell_ids[i]] for i in order]
            r_pos = ranked_names.index(T)
            ranks.append(r_pos)
            confusions = ranked_names[:r_pos]  # cells beating the true cell
            if confusions:
                agree_conf.append(st.mean(agreement(fp[T], fp[c]) for c in confusions))
                samepack_conf.append(sum(pack[c] == pack[T] for c in confusions) / len(confusions))
            # null: random cells (same count as confusions, min 1)
            k = max(1, len(confusions))
            rand = rng.sample([n for n in names if n != T], k)
            agree_null.append(st.mean(agreement(fp[T], fp[c]) for c in rand))
            base_rate.append(sum(1 for n in names if pack[n] == pack[T] and n != T) / (len(names) - 1))
            if len(examples) < 6 and confusions:
                examples.append((T, r_pos, [(c, round(agreement(fp[T], fp[c]), 2), pack[c] == pack[T]) for c in confusions[:5]]))

    ranks.sort()
    print(f"held-out cases: {len(heldout)} | median rank {ranks[len(ranks)//2]}\n")
    print("Behavioural similarity (fingerprint agreement) of the cells BEATING the true cell:")
    print(f"  confusions vs true: mean agreement {st.mean(agree_conf):.3f}")
    print(f"  random  vs true:    mean agreement {st.mean(agree_null):.3f}   (null)")
    print(f"  -> confusions are {st.mean(agree_conf)/max(1e-9,st.mean(agree_null)):.2f}x more behaviourally similar than chance")
    print(f"\nSame-pack (family) rate of confusions: {st.mean(samepack_conf):.3f}  vs base rate {st.mean(base_rate):.3f}")
    print("\nExamples (true cell @ rank; top-5 cells beating it — agreement, same-pack):")
    for T, pos, conf in examples:
        print(f"  {T} @ rank {pos}:")
        for c, ag, sp in conf:
            print(f"      {c:<28} agree {ag:.2f}  {'SAME-PACK' if sp else ''}")


if __name__ == "__main__":
    main()
