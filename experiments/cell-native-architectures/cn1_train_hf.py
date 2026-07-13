#!/usr/bin/env python3
"""CN-1 base-swap training loop (pre-registration amendment): train the three-way-tied
fingerprint model on the SmolLM2-135M base (`cn1_model_hf.py`) over the same descriptor corpus.
The honest prior-vs-capacity test — same arms, same corpus, same loss (cell-token position), same
rank eval as `cn1_train.py`, only the base changes from TinyStories v11 to a code/math-pretrained
model.

Run: python3 cn1_train_hf.py --arm fingerprint --steps 8000 --unfreeze-top 16
"""
from __future__ import annotations

import argparse
import json
import time
from pathlib import Path

import torch
import torch.nn.functional as F

import cn1_model_hf
import cn1_decode

HERE = Path(__file__).resolve().parent
TRAIN = HERE / "cn1_corpus_train.jsonl"
BOS = 0  # SmolLM2 bos/eos


def set_trainable(model, arm, base_rows, cell_first_id, unfreeze_top=0):
    for p in model.parameters():
        p.requires_grad_(False)
    trainable = []
    if arm in ("fingerprint", "shuffled", "description"):
        for p in model.w_f.parameters():
            p.requires_grad_(True); trainable.append(p)
    if unfreeze_top > 0:
        for blk in model.base.model.layers[-unfreeze_top:]:
            for p in blk.parameters():
                p.requires_grad_(True); trainable.append(p)
        for p in model.base.model.norm.parameters():
            p.requires_grad_(True); trainable.append(p)
    emb = model.base.get_input_embeddings().weight
    emb.requires_grad_(True)
    grad_mask = torch.zeros(emb.shape[0], 1, device=emb.device)
    if arm == "random":
        grad_mask[base_rows:] = 1.0            # delimiters + all cell rows
    else:
        grad_mask[base_rows:cell_first_id] = 1.0  # the 2 delimiter rows only
    emb.register_hook(lambda g: g * grad_mask)
    trainable.append(emb)
    return trainable, sum(p.numel() for p in trainable if p.requires_grad)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--arm", choices=["fingerprint", "shuffled", "random", "description"], default="fingerprint")
    ap.add_argument("--seed", type=int, default=80)
    ap.add_argument("--steps", type=int, default=8000)
    ap.add_argument("--bs", type=int, default=16)
    ap.add_argument("--lr", type=float, default=8e-4)
    ap.add_argument("--max-len", type=int, default=128)
    ap.add_argument("--unfreeze-top", type=int, default=16)
    ap.add_argument("--device", default=None)
    args = ap.parse_args()

    device = args.device or ("mps" if torch.backends.mps.is_available() else "cpu")
    rng = __import__("random").Random(args.seed)
    torch.manual_seed(args.seed)
    t0 = time.time()

    print(f"== building SmolLM2 model (arm {args.arm}, seed {args.seed}) on {device} ==", flush=True)
    model, tok, names, held, cell_first_id, base_rows = cn1_model_hf.build_hf(args.arm)
    model = model.to(device)
    trainable, n_tr = set_trainable(model, args.arm, base_rows, cell_first_id, args.unfreeze_top)
    print(f"  trainable params: {n_tr:,} (unfreeze_top={args.unfreeze_top}); base rows {base_rows}, cell_first_id {cell_first_id}")

    hf_map = json.loads((cn1_model_hf.HF_TOKEN_MAP).read_text())

    def encode_row(r):
        ids = [BOS] + tok.encode(r["text"], add_special_tokens=False)
        if len(ids) > args.max_len:
            return None
        cid = hf_map[f"<cell:{r['cell']}>"]
        if cid not in ids:
            return None
        return ids, ids.index(cid)

    rows = [json.loads(l) for l in TRAIN.read_text().splitlines() if l.strip()]
    data = [d for d in (encode_row(r) for r in rows) if d is not None]
    print(f"  train sequences: {len(data)} (from {len(rows)} rows)", flush=True)

    opt = torch.optim.Adam([p for p in trainable if p.requires_grad], lr=args.lr)
    sched = torch.optim.lr_scheduler.LambdaLR(opt, lambda s: max(0.0, 1.0 - s / max(1, args.steps)))

    def batches():
        order = list(range(len(data))); rng.shuffle(order)
        for i in range(0, len(data), args.bs):
            chunk = [data[j] for j in order[i:i + args.bs]]
            m = max(len(s) for s, _ in chunk)
            ids = torch.zeros(len(chunk), m, dtype=torch.long)
            amask = torch.zeros(len(chunk), m, dtype=torch.long)
            cpos = []
            for k, (s, cp) in enumerate(chunk):
                ids[k, :len(s)] = torch.tensor(s); amask[k, :len(s)] = 1; cpos.append(cp)
            yield ids.to(device), amask.to(device), torch.tensor(cpos, device=device)

    step, losses, accs = 0, [], []
    while step < args.steps:
        for ids, amask, cpos in batches():
            logits = model(ids, attention_mask=amask)
            b = torch.arange(ids.shape[0], device=device)
            pred = logits[b, cpos - 1]
            tgt = ids[b, cpos]
            loss = F.cross_entropy(pred, tgt)
            opt.zero_grad(); loss.backward(); opt.step(); sched.step()
            losses.append(float(loss)); accs.append(float((pred.argmax(-1) == tgt).float().mean()))
            step += 1
            if step % 25 == 0:
                print(f"  step {step:>4}/{args.steps}  loss {sum(losses[-25:])/25:.4f}  cell-acc {sum(accs[-25:])/25:.3f}  ({time.time()-t0:.0f}s)", flush=True)
            if step >= args.steps:
                break

    ckpt = HERE / f"cn1_ckpt_hf_{args.arm}_s{args.seed}.pt"
    state = {"arm": args.arm, "seed": args.seed, "embed": model.base.get_input_embeddings().weight.detach().cpu()}
    if args.arm in ("fingerprint", "shuffled", "description"):
        state["w_f"] = model.w_f.state_dict()
    for i, blk in enumerate(model.base.model.layers[-args.unfreeze_top:] if args.unfreeze_top else []):
        state[f"block_{i}"] = {k: v.detach().cpu() for k, v in blk.state_dict().items()}
    torch.save(state, ckpt)

    # eval: rank of the true cell among the 790 cell ids, per bucket
    cell_ids = sorted(v for k, v in hf_map.items() if k.startswith("<cell:"))
    cell_ids_t = torch.tensor(cell_ids, device=device)
    model.eval()
    eval_rows = [json.loads(l) for l in (HERE / "cn1_corpus_eval.jsonl").read_text().splitlines() if l.strip()]
    by_bucket = {}
    for r in eval_rows:
        by_bucket.setdefault((r["bucket_cell"], r["bucket_comp"]), []).append(r)

    @torch.no_grad()
    def bmetrics(items, cap=200):
        items = items[:cap]; top1 = top5 = 0; ranks = []
        for r in items:
            ids = torch.tensor([[BOS] + tok.encode(r["context"] + " <call>", add_special_tokens=False)], device=device)
            logits = model(ids)[0, -1]
            order = torch.argsort(logits[cell_ids_t], descending=True)
            pos = int((cell_ids_t[order] == hf_map[f"<cell:{r['cell']}>"]).nonzero().flatten()[0])
            ranks.append(pos); top1 += pos == 0; top5 += pos < 5
        ranks.sort()
        return {"top1": round(top1/len(items),4), "top5": round(top5/len(items),4),
                "median_rank": ranks[len(ranks)//2], "frac_top79": round(sum(x<79 for x in ranks)/len(items),3), "n": len(items)}

    print(f"\n== SmolLM2 eval by bucket (arm {args.arm}); chance median ~395/790 ==")
    results = {"base": "SmolLM2-135M", "arm": args.arm, "steps": args.steps, "unfreeze_top": args.unfreeze_top,
               "final_train_acc": round(sum(accs[-40:])/min(40,len(accs)),4), "buckets": {}}
    for bk in [("seen_cell","seen_comp"),("seen_cell","novel_comp"),("novel_cell","seen_comp"),("novel_cell","novel_comp")]:
        if bk not in by_bucket: continue
        m = bmetrics(by_bucket[bk]); results["buckets"]["|".join(bk)] = m
        tag = "  <- gate (ii)" if bk[0]=="novel_cell" else ""
        print(f"  {'|'.join(bk):<24} top1 {m['top1']:.3f}  top5 {m['top5']:.3f}  med.rank {m['median_rank']:>3}  top10% {m['frac_top79']:.3f}{tag}")
    out = HERE / f"cn1_train_result_hf_{args.arm}_s{args.seed}.json"
    out.write_text(json.dumps(results, indent=2))
    print(f"\nwrote {out.name}")


if __name__ == "__main__":
    main()
