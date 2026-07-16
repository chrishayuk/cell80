#!/usr/bin/env python3
"""CN-8 arm trainer (prereg §3.5) — cn7_train.py's recipe with the species machinery removed.

Raw TinyModel v11 in the original SP id space (vocab 71261, NO resize, no cell tokens),
full-model update, full loss on every token (no mask — no tier boundary in CN-8), no replay.
TinyStories NLL is recorded pre/post on the same 500-row s4 val slice cn7 used
(recorded-not-gated, §3.5).

Run: python3 cn8_train.py --corpus cn8_corpus_b.jsonl --tag b_s80 --seed 80 --tokens 6000000
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

HERE = Path(__file__).resolve().parent
PAD_ID = 0
SP_VOCAB = 71261


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


def tinystories_val():
    rows = [json.loads(l) for l in (HERE / "cn7_corpus_train.jsonl").read_text().splitlines() if l.strip()]
    s4_idx = [i for i, r in enumerate(rows) if r["species"] == "s4"]
    val = [rows[i] for i in s4_idx[-500:]]
    assert all(i < SP_VOCAB for r in val for i in r["ids"]), "s4 val slice must be pure SP text"
    return val


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--corpus", required=True)
    ap.add_argument("--tag", required=True)
    ap.add_argument("--tokens", type=int, default=6_000_000)
    ap.add_argument("--bs", type=int, default=16)
    ap.add_argument("--lr", type=float, default=1e-4)
    ap.add_argument("--warmup", type=int, default=200)
    ap.add_argument("--seed", type=int, default=80)
    ap.add_argument("--val-every", type=int, default=2000)
    ap.add_argument("--smoke", action="store_true")
    ap.add_argument("--device", default=None)
    args = ap.parse_args()
    device = args.device or ("mps" if torch.backends.mps.is_available() else "cpu")
    rng = random.Random(args.seed)
    torch.manual_seed(args.seed)
    t0 = time.time()

    ckpt_path = HERE / (f"cn8_ckpt_{args.tag}" + ("_smoke" if args.smoke else "") + ".pt")
    if ckpt_path.exists() and not args.smoke:
        raise SystemExit(f"REFUSING to run: {ckpt_path.name} already exists — a result-bearing "
                         f"checkpoint is never overwritten. Rename or move it first.")

    from tiny_model_v11.loader import load_from_artifacts
    base, cfg = load_from_artifacts(str(cn1_model.TINY_MODEL / "model" / "v11"), device="cpu")
    base = base.to(device)
    trainable = [p for p in base.parameters() if p.requires_grad]
    print(f"== CN-8 train [{args.tag}] on {device} | corpus {args.corpus} | "
          f"trainable {sum(p.numel() for p in trainable):,} ==", flush=True)

    train = [json.loads(l) for l in (HERE / args.corpus).read_text().splitlines() if l.strip()]
    if args.smoke:
        train = train[:400]
    val = tinystories_val()
    print(f"  rows: train {len(train)} | TinyStories val slice {len(val)}", flush=True)

    nll0 = val_nll(base, val, device)
    print(f"  pre-train TinyStories val NLL: {nll0:.4f} (recorded-not-gated)", flush=True)

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

    avg_len = sum(len(r["ids"]) for r in train) / len(train)
    total_steps_est = max(1, int(args.tokens / (args.bs * avg_len)))
    opt = torch.optim.AdamW(trainable, lr=args.lr, weight_decay=0.01)
    sched = torch.optim.lr_scheduler.LambdaLR(
        opt, lambda s: min(1.0, (s + 1) / args.warmup) * max(0.05, 1.0 - s / total_steps_est))

    seen_tokens, step, losses, log = 0, 0, [], []
    base.train()
    done = False
    while not done:
        for chunk in epoch_batches():
            m = max(len(r["ids"]) for r in chunk)
            ids = torch.full((len(chunk), m), PAD_ID, dtype=torch.long)
            lm = torch.zeros((len(chunk), m))
            for k, r in enumerate(chunk):
                ids[k, :len(r["ids"])] = torch.tensor(r["ids"])
                lm[k, :len(r["ids"])] = 1.0
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
            seen_tokens += int(ids.numel())
            step += 1
            if step % 100 == 0:
                print(f"  step {step:>6} ({seen_tokens/1e6:.2f}M tok)  "
                      f"loss {sum(losses[-100:])/100:.4f}  ({time.time()-t0:.0f}s)", flush=True)
            if step % args.val_every == 0:
                nv = val_nll(base, val, device)
                log.append({"step": step, "tokens": seen_tokens, "val_nll": nv})
                print(f"  [ts-val] step {step}: TinyStories NLL {nv:.4f}", flush=True)
            if seen_tokens >= args.tokens or (args.smoke and step >= 30):
                done = True
                break

    nll1 = val_nll(base, val, device)
    print(f"== done: {step} steps, {seen_tokens/1e6:.2f}M tokens | "
          f"TinyStories NLL {nll0:.4f} -> {nll1:.4f} ({time.time()-t0:.0f}s) ==", flush=True)

    ckpt = ckpt_path
    while ckpt.exists():
        ckpt = ckpt.with_name(ckpt.stem + "+.pt")
    torch.save({"tag": args.tag, "corpus": args.corpus, "seed": args.seed, "tokens": seen_tokens,
                "pre_ts_nll": nll0, "post_ts_nll": nll1, "val_log": log,
                "state": base.state_dict()}, ckpt)
    print(f"saved {ckpt.name}")


if __name__ == "__main__":
    main()
