#!/usr/bin/env python3
"""CN-1 synthetic scale curve: extend held-out rank measurement to a 10^4-cell library (790 real +
9,210 density-matched synthetics), using the FIXED seed-81 W_f. This answers hypothesis (a) — does
behavioural geometry crowd the held-out cell as the addressable library grows — over a full extra
decade of N, which the 114->790 curve could not (α CI [0.38,0.87], underpowered).

Design: the seed-81 W_f is FIXED (trained on 790 real cells); we add synthetic cells whose rows are
W_f(their fingerprint) — the deployment model of "a library grows with synthesized cells the model
was not retrained on." This is NOT random subsampling (the synthetics are structured, density-matched
distractors), so crowding is genuine density-driven competition, not a proportional artifact. Per
held-out eval item we compute the transformer hidden state once, then logit_c = hidden · W_f(fp_c)
for all cells and rank the held-out cell among the first N. Refit α over 790->10^4 and combine with
the retrained 114->790 curve for the widest-range exponent.

NOTE ON SCOPE: this tests (a) at scale with a FIXED address. It does not retrain routing at 10^4, so
it does not test hypothesis (b) (does a *learned* softmax over 10^4 tokens survive) — that needs the
heavier retrain-with-10^4-tokens build and remains owed.

Run: python3 cn1_synth_scale.py
"""
from __future__ import annotations

import json
import math
import statistics as st
from pathlib import Path

import torch
import cn1_model
from cn1_model import encode_fingerprint
from cn1_train import load_tokenizer

HERE = Path(__file__).resolve().parent
N_POINTS = [790, 1500, 3000, 5000, 8000, 10000]
SEED = 81


def main():
    device = "mps" if torch.backends.mps.is_available() else "cpu"
    tok = load_tokenizer()
    tok_map = json.loads((HERE / "cn1_cell_token_map.json").read_text())
    lib = [json.loads(l) for l in (HERE / "cn1_library.jsonl").read_text().splitlines() if l.strip()]
    held = {h["name"] for h in json.loads((HERE / "cn1_axis_a_heldout.json").read_text())["held_out_cells"]}
    synth = [json.loads(l) for l in (HERE / "cn1_synth_fingerprints.jsonl").read_text().splitlines() if l.strip()]

    # all cells: real first (so N-subsets nest and always include the held-out real cells), then synth
    all_cells = lib + synth
    feats = torch.tensor([encode_fingerprint(c["fingerprint"]) for c in all_cells], dtype=torch.float32)

    # model with the seed-81 trained transformer + W_f (FIXED)
    model, names, _ = cn1_model.build("fingerprint")
    ck = torch.load(HERE / f"cn1_ckpt_fingerprint_s{SEED}.pt", map_location="cpu")
    with torch.no_grad():
        model.base.embed.weight.copy_(ck["embed"])
    model.w_f.load_state_dict(ck["w_f"])
    for i, blk in enumerate(model.base.layers[-16:]):
        blk.load_state_dict(ck[f"block_{i}"])
    if "norm" in ck:
        model.base.norm.load_state_dict(ck["norm"])
    model = model.to(device).eval()
    feats = feats.to(device)

    # W_f(fp) for ALL cells -> (n_all, dim). Held-out real cells sit among the first 790.
    with torch.no_grad():
        rows = model.w_f(feats)  # (n_all, dim)
    name_to_idx = {c["name"]: i for i, c in enumerate(all_cells)}

    ev = [json.loads(l) for l in (HERE / "cn1_corpus_eval.jsonl").read_text().splitlines() if l.strip()]
    ho = [r for r in ev if r["bucket_cell"] == "novel_cell" and r["bucket_comp"] == "seen_comp" and lib_arity(lib, r["cell"]) >= 1]
    import random
    random.Random(0).shuffle(ho)
    ho = ho[:200]

    # per held-out item: hidden at the <call> position (frozen transformer), then dot with rows.
    # Matches CN1Model.forward (v11: embed -> layers(x, rope_freqs) -> norm), minus the output
    # matmul. The context has no cell tokens, so the input embedding uses the base (non-cell) rows.
    import torch.nn.functional as F
    embed_w = model.base.embed.weight
    hiddens = []
    truth = []
    with torch.no_grad():
        for r in ho:
            ids = torch.tensor([[2] + tok.encode(r["context"] + " <call>")], device=device)
            x = F.embedding(ids, embed_w) * math.sqrt(model.dim)
            for layer in model.base.layers:
                x = layer(x, model.base.rope_freqs)
            x = model.base.norm(x)
            hiddens.append(x[0, -1])
            truth.append(name_to_idx[r["cell"]])
    H = torch.stack(hiddens)  # (n_items, dim)

    results = []
    for N in N_POINTS:
        subrows = rows[:N]  # first N cells (real 790 + synth up to N)
        ranks = []
        with torch.no_grad():
            logits = H @ subrows.t()  # (n_items, N)
            order = torch.argsort(logits, dim=1, descending=True)
            for k, t in enumerate(truth):
                pos = int((order[k] == t).nonzero().flatten()[0])
                ranks.append(pos)
        ranks.sort()
        med = ranks[len(ranks) // 2]
        results.append({"N": N, "median_rank": med, "chance": N / 2, "lift": round((N / 2) / max(med, 1), 2)})
        print(f"N={N:>6}  held-out median rank {med:>5}  chance {N/2:>6.0f}  lift x{(N/2)/max(med,1):>5.1f}", flush=True)

    # refit alpha over 790->10^4 (synthetic regime), anchored at the real N=790 rank
    r790 = results[0]["median_rank"]
    xs = [math.log(r["N"] / 790) for r in results]
    ys = [math.log(max(r["median_rank"], 1) / r790) for r in results]
    n = len(xs); mx = sum(xs) / n; my = sum(ys) / n
    Sxx = sum((x - mx) ** 2 for x in xs); Sxy = sum((x - mx) * (y - my) for x, y in zip(xs, ys))
    alpha = Sxy / Sxx
    resid = [y - (alpha * x + (my - alpha * mx)) for x, y in zip(xs, ys)]
    s = math.sqrt(sum(r * r for r in resid) / (n - 2)); SE = s / math.sqrt(Sxx)
    print(f"\nsynthetic regime (790->1e4): alpha = {alpha:.3f}  SE {SE:.3f}  95% CI [{alpha-2.776*SE:.2f}, {alpha+2.776*SE:.2f}]")
    print(f"  extrapolated rank(1e6) = {r790 * (1e6/790)**alpha:,.0f}  (K_exec GPU 4718)")
    (HERE / "cn1_synth_scale_results.json").write_text(json.dumps({"points": results, "alpha": alpha, "SE": SE}, indent=2))


def lib_arity(lib, name):
    for r in lib:
        if r["name"] == name:
            return r["arity"]
    return 0


def _causal_mask(n, device):
    import torch
    m = torch.full((n, n), float("-inf"), device=device)
    return torch.triu(m, diagonal=1)


if __name__ == "__main__":
    main()
