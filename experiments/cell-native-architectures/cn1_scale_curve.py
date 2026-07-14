#!/usr/bin/env python3
"""CN-1 library-scale curve (hypothesis a): does behavioural GEOMETRY hold as the library grows?
Fit rank(N) = 98*(N/790)^alpha on log-log; pass iff alpha < 0.54 (=> rank(1e6) < K_exec GPU 4718).

Honest design: post-hoc mask subsampling is trivially fractional, so at each library size N we
RETRAIN the address. To keep 6-7 retrains cheap AND isolate crowding from training-amount, we hold
the seed-81 TRAINED transformer fixed (it already reads descriptors) and retrain only W_f (tiny) on
the corpus restricted to the subset's seen cells. Nested random subsets always hold the axis-A
held-out value cells in (eval-only at every N). Eval = held-out median rank AMONG the N cells;
chance-median (N/2) recorded alongside for lift-over-chance.

  --validate : just retrain W_f at N=790 and check it reproduces the full-model held-out rank (~98)
               before trusting the curve.
Run: python3 cn1_scale_curve.py            # full sweep
     python3 cn1_scale_curve.py --validate  # N=790 method check
"""
from __future__ import annotations

import argparse
import json
import random
import statistics as st
from pathlib import Path

import torch
import torch.nn.functional as F

import cn1_model
import cn1_decode
from cn1_train import load_tokenizer, PAD_ID

HERE = Path(__file__).resolve().parent
N_POINTS = [114, 175, 270, 415, 640, 790]
WF_STEPS = 3000
LR = 2e-3
SEED = 81


def build_frozen_transformer():
    model, names, held = cn1_model.build("fingerprint")
    ck = torch.load(HERE / f"cn1_ckpt_fingerprint_s{SEED}.pt", map_location="cpu")
    with torch.no_grad():
        model.base.embed.weight.copy_(ck["embed"])
    for i, blk in enumerate(model.base.layers[-16:]):
        blk.load_state_dict(ck[f"block_{i}"])
    if "norm" in ck:
        model.base.norm.load_state_dict(ck["norm"])
    return model, names, held


def retrain_wf_and_eval(model, tok, cell_id, id_to_name, train_rows, subset_ids, heldout_rows, device):
    """Reset W_f, freeze everything else, retrain W_f on train_rows (subset's seen cells), then eval
    held-out median rank AMONG subset_ids."""
    for p in model.parameters():
        p.requires_grad_(False)
    # fresh W_f
    model.w_f = cn1_model.Wf(model.fp_feats.shape[1], model.dim)
    model = model.to(device)
    for p in model.w_f.parameters():
        p.requires_grad_(True)
    opt = torch.optim.Adam(model.w_f.parameters(), lr=LR)
    sched = torch.optim.lr_scheduler.LambdaLR(opt, lambda s: max(0.0, 1 - s / WF_STEPS))
    data = []
    for r in train_rows:
        ids = [2] + tok.encode(r["text"])
        try:
            cp = ids.index(r["cell_id"])
        except ValueError:
            continue
        if len(ids) <= 128:
            data.append((ids, cp))
    rng = random.Random(0)
    model.train()
    step = 0
    while step < WF_STEPS and data:
        order = list(range(len(data)))
        rng.shuffle(order)
        for i in range(0, len(data), 16):
            chunk = [data[j] for j in order[i:i + 16]]
            m = max(len(s) for s, _ in chunk)
            ids = torch.full((len(chunk), m), PAD_ID, dtype=torch.long)
            cps = []
            for k, (s, cp) in enumerate(chunk):
                ids[k, :len(s)] = torch.tensor(s)
                cps.append(cp)
            ids = ids.to(device)
            cps = torch.tensor(cps, device=device)
            b = torch.arange(len(chunk), device=device)
            logits = model(ids)
            loss = F.cross_entropy(logits[b, cps - 1], ids[b, cps])
            opt.zero_grad(); loss.backward(); opt.step(); sched.step()
            step += 1
            if step >= WF_STEPS:
                break
    # eval: rank among subset_ids
    model.eval()
    sub = torch.tensor(sorted(subset_ids), device=device)
    ranks = []
    with torch.no_grad():
        for r in heldout_rows:
            ids = torch.tensor([[2] + tok.encode(r["context"] + " <call>")], device=device)
            order = torch.argsort(model(ids)[0, -1][sub], descending=True)
            pos = int((sub[order] == r["cell_id"]).nonzero().flatten()[0])
            ranks.append(pos)
    ranks.sort()
    return ranks[len(ranks) // 2], len(ranks)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--validate", action="store_true")
    args = ap.parse_args()
    device = "mps" if torch.backends.mps.is_available() else "cpu"
    tok = load_tokenizer()
    tok_map = json.loads((HERE / "cn1_cell_token_map.json").read_text())
    lib = {json.loads(l)["name"]: json.loads(l) for l in (HERE / "cn1_library.jsonl").read_text().splitlines() if l.strip()}
    held = {h["name"] for h in json.loads((HERE / "cn1_axis_a_heldout.json").read_text())["held_out_cells"]}
    held_val = [n for n in lib if n in held and lib[n]["arity"] >= 1]           # 24 eval targets
    others = [n for n in lib if n not in set(held_val)]                          # 766 (held into subsets by size)
    rng = random.Random(3); rng.shuffle(others)
    name_to_id = {k[len("<cell:"):-1]: v for k, v in tok_map.items() if k.startswith("<cell:")}
    id_to_name = {v: k for k, v in name_to_id.items()}

    train_all = [json.loads(l) for l in (HERE / "cn1_corpus_train.jsonl").read_text().splitlines() if l.strip()]
    eval_all = [json.loads(l) for l in (HERE / "cn1_corpus_eval.jsonl").read_text().splitlines() if l.strip()]
    heldout_rows = [r for r in eval_all if r["bucket_cell"] == "novel_cell" and r["bucket_comp"] == "seen_comp" and r["cell"] in held_val]
    random.Random(0).shuffle(heldout_rows)
    heldout_rows = heldout_rows[:200]

    model, names, _ = build_frozen_transformer()

    points = [790] if args.validate else N_POINTS
    results = []
    for N in points:
        subset_names = set(held_val) | set(others[: N - len(held_val)])
        subset_ids = [name_to_id[n] for n in subset_names]
        seen_in_subset = {n for n in subset_names if n not in held and lib[n]["arity"] >= 1}
        train_rows = [r for r in train_all if r["cell"] in seen_in_subset]
        med, n = retrain_wf_and_eval(model, tok, name_to_id, id_to_name, train_rows, subset_ids, heldout_rows, device)
        chance = N / 2
        results.append({"N": N, "median_rank": med, "chance_median": chance, "lift": round(chance / max(med, 1), 2),
                        "n_seen_trained": len(seen_in_subset), "n_train_rows": len(train_rows)})
        print(f"N={N:>4}  held-out median rank {med:>4}  chance {chance:>4.0f}  lift x{chance/max(med,1):>4.1f}  "
              f"(seen trained {len(seen_in_subset)}, rows {len(train_rows)})", flush=True)

    if not args.validate and len(results) >= 3:
        import math
        xs = [math.log(r["N"] / 790) for r in results]
        ys = [math.log(max(r["median_rank"], 1) / 98) for r in results]
        n = len(xs); sx = sum(xs); sy = sum(ys); sxx = sum(x * x for x in xs); sxy = sum(x * y for x, y in zip(xs, ys))
        alpha = (n * sxy - sx * sy) / (n * sxx - sx * sx)
        print(f"\nlog-log fit: rank(N) ~ 98*(N/790)^alpha  =>  alpha = {alpha:.3f}")
        print(f"pass threshold alpha < 0.54 (rank(1e6) < K_exec GPU 4718):  {'PASS' if alpha < 0.54 else 'FAIL'}")
        print(f"extrapolated rank(1e6) = {98 * (1e6/790)**alpha:,.0f}")
        (HERE / "cn1_scale_curve_results.json").write_text(json.dumps({"points": results, "alpha": alpha}, indent=2))

    if args.validate:
        print(f"\nVALIDATION: W_f-only retrain at N=790 gave held-out median {results[0]['median_rank']} "
              f"(full-model faithful was ~98). Method {'OK' if abs(results[0]['median_rank']-98)<40 else 'SUSPECT'}.")


if __name__ == "__main__":
    main()
