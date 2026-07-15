#!/usr/bin/env python3
"""CN-7.2 prerequisite — B9'/B10': the CN-1 fingerprint finetune re-run on unmodified v11 in the
ORIGINAL SP id space (prereg v0.2 §8.2).

Why this exists: CN-1 encoded through the committed-but-wrong tokenizer mapping (~30% of context
tokens hit trained rows — §8.1), so B9/B10 measured a mostly-untrained-embedding substrate.
CN-7.2's P-d1/P-d2 must grade against baselines measured on the FIXED stack: same protocol as the
seeded CN-1 runs (fingerprint arm, W_f three-way tying, unfreeze_top=16, 8000 steps, bs 16,
lr 1e-3 linear-decay, cell-position CE), but text encoded with the SP tokenizer and cell tokens
at the CN-7 ids (cn7_token_map.json: <call>=71261, </call>=71262, cells 71263+).

P-d1' threshold freezes as (B9' + 32) the moment this prints B9'; P-d2' = seed std over 3 seeds.
Eval buckets are RANDOM-sampled (seed 0) before capping — the first-N bug stays dead.

Run: python3 cn7_fp_rebaseline.py --seed 81            # one seed
     for s in 80 81 82; do python3 cn7_fp_rebaseline.py --seed $s; done
"""
from __future__ import annotations

import argparse
import json
import random
import re
import time
from pathlib import Path

import torch
import torch.nn.functional as F

import cn1_model
from cn1_model import CN1Model, resize_embedding, load_fingerprint_features
from cn7_corpus import CALL_ID, CLOSE_ID, CELL_FIRST_ID, SP_MODEL

HERE = Path(__file__).resolve().parent
PAD_ID = 0
_MARK = re.compile(r"(<call>|</call>|<cell:[a-z0-9_]+>)")


class SpEnc:
    def __init__(self, cell_ids):
        import sentencepiece as spm
        self.sp = spm.SentencePieceProcessor(model_file=SP_MODEL)
        self.cell_ids = cell_ids

    def encode(self, text):
        ids = []
        for seg in filter(None, _MARK.split(text)):
            if seg == "<call>":
                ids.append(CALL_ID)
            elif seg == "</call>":
                ids.append(CLOSE_ID)
            elif seg.startswith("<cell:"):
                ids.append(self.cell_ids[seg[6:-1]])
            else:
                ids.extend(self.sp.encode(seg))
        return ids


def build(arm: str, cell_ids: dict):
    from tiny_model_v11.loader import load_from_artifacts
    base, cfg = load_from_artifacts(str(cn1_model.TINY_MODEL / "model" / "v11"), device="cpu")
    vocab = CELL_FIRST_ID + len(cell_ids)
    resize_embedding(base, vocab)
    feats, _, names, held = load_fingerprint_features(kind="fingerprint")
    by_name = {n: feats[i] for i, n in enumerate(names)}
    order = sorted(cell_ids, key=cell_ids.get)
    assert cell_ids[order[0]] == CELL_FIRST_ID and set(order) == set(names)
    feats7 = torch.stack([by_name[n] for n in order])
    model = CN1Model(base, CELL_FIRST_ID, feats7, arm)
    return model, order, held


def set_trainable(model, arm, unfreeze_top):
    for p in model.parameters():
        p.requires_grad_(False)
    trainable = []
    if arm in ("fingerprint", "shuffled", "description"):
        for p in model.w_f.parameters():
            p.requires_grad_(True)
            trainable.append(p)
    for blk in model.base.layers[-unfreeze_top:]:
        for p in blk.parameters():
            p.requires_grad_(True)
            trainable.append(p)
    for p in model.base.norm.parameters():
        p.requires_grad_(True)
        trainable.append(p)
    emb = model.base.embed.weight
    emb.requires_grad_(True)
    grad_mask = torch.zeros(emb.shape[0], 1, device=emb.device)
    if arm == "random":
        grad_mask[CALL_ID:] = 1.0            # delimiters + all cell rows
    else:
        grad_mask[CALL_ID:CELL_FIRST_ID] = 1.0  # the two delimiter rows only
    emb.register_hook(lambda g: g * grad_mask)
    trainable.append(emb)
    return trainable


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--arm", default="fingerprint", choices=["fingerprint", "shuffled", "random"])
    ap.add_argument("--seed", type=int, default=81)
    ap.add_argument("--steps", type=int, default=8000)
    ap.add_argument("--bs", type=int, default=16)
    ap.add_argument("--lr", type=float, default=1e-3)
    ap.add_argument("--unfreeze-top", type=int, default=16)
    ap.add_argument("--max-len", type=int, default=128)
    ap.add_argument("--smoke", action="store_true")
    ap.add_argument("--device", default=None)
    args = ap.parse_args()
    device = args.device or ("mps" if torch.backends.mps.is_available() else "cpu")
    rng = random.Random(args.seed)
    torch.manual_seed(args.seed)
    t0 = time.time()

    ckpt_path = HERE / f"cn7_ckpt_fp_{args.arm}_s{args.seed}.pt"
    if ckpt_path.exists() and not args.smoke:
        raise SystemExit(f"REFUSING to run: {ckpt_path.name} already exists — a result-bearing "
                         f"checkpoint is never overwritten (CN-6 §8.3 lesson). Rename or move it first.")

    cell_ids = json.load(open(HERE / "cn7_token_map.json"))["cells"]
    print(f"== CN-7 fp re-baseline (arm {args.arm}, seed {args.seed}) on {device}, SP id space ==", flush=True)
    model, names, held = build(args.arm, cell_ids)
    model = model.to(device)
    trainable = set_trainable(model, args.arm, args.unfreeze_top)
    print(f"  trainable params: {sum(p.numel() for p in trainable if p.requires_grad):,}", flush=True)

    enc = SpEnc(cell_ids)
    rows = [json.loads(l) for l in (HERE / "cn1_corpus_train.jsonl").read_text().splitlines() if l.strip()]
    if args.smoke:
        rows = rows[:1500]
    data = []
    for r in rows:
        ids = [2] + enc.encode(r["text"]) + [3]
        cid = cell_ids[r["cell"]]
        if len(ids) <= args.max_len and cid in ids:
            data.append((ids, ids.index(cid)))
    print(f"  train sequences: {len(data)} (from {len(rows)} rows)", flush=True)

    opt = torch.optim.Adam([p for p in trainable if p.requires_grad], lr=args.lr)
    sched = torch.optim.lr_scheduler.LambdaLR(opt, lambda s: max(0.0, 1.0 - s / max(1, args.steps)))

    step, losses, accs = 0, [], []
    while step < args.steps:
        order = list(range(len(data)))
        rng.shuffle(order)
        for i in range(0, len(order), args.bs):
            chunk = [data[j] for j in order[i:i + args.bs]]
            m = max(len(s) for s, _ in chunk)
            ids = torch.full((len(chunk), m), PAD_ID, dtype=torch.long)
            for k, (s, _) in enumerate(chunk):
                ids[k, :len(s)] = torch.tensor(s)
            ids = ids.to(device)
            cpos = torch.tensor([c for _, c in chunk], device=device)
            logits = model(ids)
            b = torch.arange(ids.shape[0], device=device)
            pred = logits[b, cpos - 1]
            tgt = ids[b, cpos]
            loss = F.cross_entropy(pred, tgt)
            opt.zero_grad(); loss.backward(); opt.step(); sched.step()
            losses.append(float(loss)); accs.append(float((pred.argmax(-1) == tgt).float().mean()))
            step += 1
            if step % 100 == 0:
                print(f"  step {step:>5}/{args.steps}  loss {sum(losses[-100:])/100:.4f}  "
                      f"cell-acc {sum(accs[-100:])/100:.3f}  ({time.time()-t0:.0f}s)", flush=True)
            if step >= args.steps:
                break

    ckpt = ckpt_path
    while ckpt.exists():  # never overwrite, even in a race or smoke rerun
        ckpt = ckpt.with_name(ckpt.stem + "+.pt")
    state = {"arm": args.arm, "seed": args.seed, "embed": model.base.embed.weight.detach().cpu(),
             "unfreeze_top": args.unfreeze_top}
    if args.arm in ("fingerprint", "shuffled"):
        state["w_f"] = model.w_f.state_dict()
    for i, blk in enumerate(model.base.layers[-args.unfreeze_top:]):
        state[f"block_{i}"] = {k: v.detach().cpu() for k, v in blk.state_dict().items()}
    state["norm"] = {k: v.detach().cpu() for k, v in model.base.norm.state_dict().items()}
    torch.save(state, ckpt)
    print(f"  saved {ckpt.name}", flush=True)

    # eval: random-sampled buckets (seed 0 shuffle BEFORE cap — first-N bug stays dead)
    cell_ids_t = torch.tensor(sorted(cell_ids.values()), device=device)
    model.eval()
    ev = [json.loads(l) for l in (HERE / "cn1_corpus_eval.jsonl").read_text().splitlines() if l.strip()]
    by_bucket = {}
    for r in ev:
        by_bucket.setdefault((r["bucket_cell"], r["bucket_comp"]), []).append(r)

    @torch.no_grad()
    def metrics(items, cap=200):
        items = list(items)
        random.Random(0).shuffle(items)
        items = items[:cap]
        top1 = top5 = 0
        ranks = []
        for r in items:
            ids = torch.tensor([[2] + enc.encode(r["context"] + " <call>")], device=device)
            lg = model(ids)[0, -1]
            cl = lg[cell_ids_t]
            ranked = cell_ids_t[torch.argsort(cl, descending=True)]
            true = cell_ids[r["cell"]]
            pos = int((ranked == true).nonzero().flatten()[0])
            ranks.append(pos); top1 += pos == 0; top5 += pos < 5
        ranks.sort()
        return {"top1": round(top1 / len(items), 4), "top5": round(top5 / len(items), 4),
                "median_rank": ranks[len(ranks) // 2], "n": len(items)}

    results = {"arm": args.arm, "seed": args.seed, "steps": args.steps,
               "unfreeze_top": args.unfreeze_top, "id_space": "sp-v11-original",
               "final_train_acc": round(sum(accs[-40:]) / min(40, len(accs)), 4), "buckets": {}}
    print(f"== eval (random-sampled buckets, chance median ~395/790) ==", flush=True)
    for bucket in [("seen_cell", "seen_comp"), ("seen_cell", "novel_comp"),
                   ("novel_cell", "seen_comp"), ("novel_cell", "novel_comp")]:
        if bucket not in by_bucket:
            continue
        m = metrics(by_bucket[bucket])
        results["buckets"]["|".join(bucket)] = m
        tag = "  <- B9' signal" if bucket == ("novel_cell", "seen_comp") else ""
        print(f"  {'|'.join(bucket):<24} top1 {m['top1']:.3f}  top5 {m['top5']:.3f}  "
              f"med.rank {m['median_rank']:>3}/790  (n={m['n']}){tag}", flush=True)

    out = HERE / f"cn7_fp_rebaseline_{args.arm}_s{args.seed}.json"
    out.write_text(json.dumps(results, indent=2))
    print(f"wrote {out.name} ({time.time()-t0:.0f}s total)")


if __name__ == "__main__":
    main()
