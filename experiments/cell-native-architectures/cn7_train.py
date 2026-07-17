#!/usr/bin/env python3
"""CN-7.2 — the numeracy midtrain (prereg §4 CN-7.2, v0.2 §8.2: SP id space).

Midtrains the pretrained TinyModel v11 on the audited S1–S4 mix (cn7_corpus_train.jsonl,
7.1 gate OPEN required) with the per-token loss mask honoured — environment-injected
beyond-tier answers contribute ZERO gradient. Primary arm: full-model update with replay
(arithmetic circuits want FFN capacity); fallback arm: --attention-only (v10a-style, FFN
frozen) if P-c/P-d regress.

Vocab: resized 71261 -> 72053 (cn7_token_map.json). The ~62k never-trained SP rows stay as
they are — the corpus never targets them. P-e instrumentation: the last 500 S4 rows are held
back as a validation slice; pre-midtrain NLL on it is recorded in the checkpoint (the +5%
bound reads against that number).

Run: python3 cn7_train.py --tokens 15000000
"""
from __future__ import annotations

import argparse
import json
import random
import time
from pathlib import Path

import torch
import torch.nn.functional as F

import cn1_model
from cn1_model import resize_embedding
from cn7_corpus import CELL_FIRST_ID
from artifact_paths import checkpoint_output, dataset_input

HERE = Path(__file__).resolve().parent
PAD_ID = 0


def val_nll(model, val, device, bs=16):
    model.eval()
    tot, n = 0.0, 0
    with torch.no_grad():
        for i in range(0, len(val), bs):
            chunk = val[i:i + bs]
            m = max(len(r["ids"]) for r in chunk)
            ids = torch.full((len(chunk), m), PAD_ID, dtype=torch.long)
            am = torch.zeros((len(chunk), m))
            for k, r in enumerate(chunk):
                ids[k, :len(r["ids"])] = torch.tensor(r["ids"])
                am[k, :len(r["ids"])] = 1
            ids, am = ids.to(device), am.to(device)
            lg = model(ids)[:, :-1]
            tgt = ids[:, 1:]
            w = am[:, 1:]
            ce = F.cross_entropy(lg.reshape(-1, lg.shape[-1]), tgt.reshape(-1), reduction="none")
            tot += float((ce * w.reshape(-1)).sum())
            n += int(w.sum())
    model.train()
    return tot / max(1, n)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--tokens", type=int, default=15_000_000)
    ap.add_argument("--bs", type=int, default=16)
    ap.add_argument("--lr", type=float, default=1e-4)
    ap.add_argument("--warmup", type=int, default=200)
    ap.add_argument("--attention-only", action="store_true", help="fallback arm: freeze FFN")
    ap.add_argument("--no-mask", action="store_true", help="CN-7.5 control: injected/beyond-tier answers carry loss")
    ap.add_argument("--seed", type=int, default=80)
    ap.add_argument("--val-every", type=int, default=2000)
    ap.add_argument("--smoke", action="store_true")
    ap.add_argument("--device", default=None)
    args = ap.parse_args()
    device = args.device or ("mps" if torch.backends.mps.is_available() else "cpu")
    rng = random.Random(args.seed)
    torch.manual_seed(args.seed)
    t0 = time.time()

    stem = "cn7_ckpt_midtrain_attn" if args.attention_only else "cn7_ckpt_midtrain"
    if args.no_mask:
        stem += "_nomask"
    ckpt_path = checkpoint_output(stem + ("_smoke" if args.smoke else "") + ".pt")
    if ckpt_path.exists() and not args.smoke:
        raise SystemExit(f"REFUSING to run: {ckpt_path.name} already exists — a result-bearing "
                         f"checkpoint is never overwritten (CN-6 §8.3 lesson). Rename or move it first.")

    tokmap = json.load(open(HERE / "cn7_token_map.json"))
    vocab = tokmap["vocab"]
    from tiny_model_v11.loader import load_from_artifacts
    base, cfg = load_from_artifacts(str(cn1_model.TINY_MODEL / "model" / "v11"), device="cpu")
    resize_embedding(base, vocab)
    base = base.to(device)

    arm = "attention-only" if args.attention_only else "full"
    if args.attention_only:
        for blk in base.layers:
            for name, p in blk.named_parameters():
                if "ffn" in name or "mlp" in name or "feed" in name:
                    p.requires_grad_(False)
    trainable = [p for p in base.parameters() if p.requires_grad]
    print(f"== CN-7.2 midtrain ({arm}) on {device} | vocab {vocab} | "
          f"trainable {sum(p.numel() for p in trainable):,} ==", flush=True)

    rows = [json.loads(l) for l in dataset_input("cn7_corpus_train.jsonl").read_text().splitlines() if l.strip()]
    s4_idx = [i for i, r in enumerate(rows) if r["species"] == "s4"]
    val_set = set(s4_idx[-500:])
    val = [rows[i] for i in sorted(val_set)]
    train = [r for i, r in enumerate(rows) if i not in val_set]
    if args.smoke:
        train = train[:400]
    print(f"  rows: train {len(train)} | P-e val slice {len(val)}", flush=True)

    nll0 = val_nll(base, val, device)
    print(f"  pre-midtrain TinyStories val NLL: {nll0:.4f}  (P-e bound: {nll0 * 1.05:.4f})", flush=True)

    # length-bucketed batching: sort a shuffled window by length, batch, shuffle batches
    def epoch_batches():
        order = list(range(len(train)))
        rng.shuffle(order)
        W = 4096
        batches = []
        for w in range(0, len(order), W):
            win = sorted(order[w:w + W], key=lambda i: len(train[i]["ids"]))
            for i in range(0, len(win), args.bs):
                batches.append([train[j] for j in win[i:i + args.bs]])
        rng.shuffle(batches)
        return batches

    total_steps_est = max(1, args.tokens // (args.bs * 54))
    opt = torch.optim.AdamW(trainable, lr=args.lr, weight_decay=0.01)
    sched = torch.optim.lr_scheduler.LambdaLR(
        opt, lambda s: min(1.0, (s + 1) / args.warmup) * max(0.05, 1.0 - s / total_steps_est))

    seen_tokens, step, losses = 0, 0, []
    log = []
    # per-species loss decomposition (accumulated as tensors; synced only at log time) — the
    # blend alone can't distinguish "drill saturated, rest is maintenance" from "still absorbing",
    # and the saturation point is the empirical anchor for later runs' token budgets.
    spec_acc = {}
    base.train()
    done = False
    while not done:
        for chunk in epoch_batches():
            m = max(len(r["ids"]) for r in chunk)
            ids = torch.full((len(chunk), m), PAD_ID, dtype=torch.long)
            lm = torch.zeros((len(chunk), m))
            for k, r in enumerate(chunk):
                ids[k, :len(r["ids"])] = torch.tensor(r["ids"])
                if args.no_mask:  # CN-7.5: every in-row token carries loss, incl. injected answers
                    lm[k, :len(r["ids"])] = 1.0
                else:
                    lm[k, :len(r["ids"])] = torch.tensor(r["loss"], dtype=torch.float32)
            ids, lm = ids.to(device), lm.to(device)
            lg = base(ids)[:, :-1]
            tgt = ids[:, 1:]
            w = lm[:, 1:]
            ce = F.cross_entropy(lg.reshape(-1, lg.shape[-1]), tgt.reshape(-1), reduction="none")
            loss = (ce * w.reshape(-1)).sum() / w.sum().clamp(min=1)
            opt.zero_grad(); loss.backward()
            torch.nn.utils.clip_grad_norm_(trainable, 1.0)
            opt.step(); sched.step()
            losses.append(float(loss))
            ce_rows = (ce.reshape(w.shape) * w).sum(1).detach()
            w_rows = w.sum(1).detach()
            groups = {}
            for k, r in enumerate(chunk):
                groups.setdefault(r["species"], []).append(k)
            for sp_name, idx in groups.items():
                acc = spec_acc.setdefault(sp_name, [torch.zeros((), device=device),
                                                    torch.zeros((), device=device)])
                acc[0] += ce_rows[idx].sum()
                acc[1] += w_rows[idx].sum()
            seen_tokens += int(ids.numel())
            step += 1
            if step % 100 == 0:
                per = "  ".join(f"{sp_}:{float(a[0])/max(1.0,float(a[1])):.3f}"
                                for sp_, a in sorted(spec_acc.items()))
                spec_acc = {}
                print(f"  step {step:>6} ({seen_tokens/1e6:.2f}M tok)  loss {sum(losses[-100:])/100:.4f}"
                      f"  [{per}]  ({time.time()-t0:.0f}s)", flush=True)
            if step % args.val_every == 0:
                nv = val_nll(base, val, device)
                log.append({"step": step, "tokens": seen_tokens, "val_nll": nv})
                print(f"  [P-e] step {step}: TinyStories val NLL {nv:.4f} (bound {nll0*1.05:.4f})", flush=True)
            if seen_tokens >= args.tokens or (args.smoke and step >= 30):
                done = True
                break

    nll1 = val_nll(base, val, device)
    print(f"== done: {step} steps, {seen_tokens/1e6:.2f}M tokens | "
          f"val NLL {nll0:.4f} -> {nll1:.4f} ({(nll1/nll0-1)*100:+.1f}%) ({time.time()-t0:.0f}s) ==", flush=True)

    ckpt = ckpt_path
    while ckpt.exists():  # never overwrite, even in a race or smoke rerun
        ckpt = ckpt.with_name(ckpt.stem + "+.pt")
    torch.save({"arm": arm, "vocab": vocab, "seed": args.seed, "tokens": seen_tokens,
                "pre_val_nll": nll0, "post_val_nll": nll1, "val_log": log,
                "state": base.state_dict()}, ckpt)
    print(f"saved {ckpt.name}")


if __name__ == "__main__":
    main()
