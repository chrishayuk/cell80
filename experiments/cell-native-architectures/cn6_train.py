#!/usr/bin/env python3
"""CN-6 stage 2 training — fine-tune SmolLM2-135M to EMIT an I/O example spec (not a cell token).
Plain causal LM: no W_f, no cell tokens — the model just generates the spec text after `<call>`, and
the runtime router resolves it. SmolLM2 (code/math prior) is the base because generation requires
computing example outputs. Loss is on the SPEC tokens (everything from `<call>` onward), so the
gradient concentrates on producing the examples, not on the (frozen-relevant) descriptor prefix.

Run: python3 cn6_train.py --arm generation --steps 6000 [--smoke]
"""
from __future__ import annotations

import argparse
import json
import time
from pathlib import Path

import torch
import torch.nn.functional as F

HERE = Path(__file__).resolve().parent
BASE = "HuggingFaceTB/SmolLM2-135M"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--arm", choices=["generation", "extraction"], default="generation")
    ap.add_argument("--base", default=BASE, help="HF base model id (e.g. meta-llama/Llama-3.2-1B)")
    ap.add_argument("--input-max", type=int, default=1000, help="pick the corpus regenerated at this input range")
    ap.add_argument("--steps", type=int, default=6000)
    ap.add_argument("--bs", type=int, default=16)
    ap.add_argument("--lr", type=float, default=3e-4)
    ap.add_argument("--unfreeze-top", type=int, default=12)
    ap.add_argument("--max-len", type=int, default=96)
    ap.add_argument("--smoke", action="store_true")
    ap.add_argument("--device", default=None)
    args = ap.parse_args()
    TAG = "" if args.input_max == 1000 else f"_i{args.input_max}"
    device = args.device or ("mps" if torch.backends.mps.is_available() else "cpu")
    rng = __import__("random").Random(80)
    torch.manual_seed(80)
    t0 = time.time()

    from transformers import AutoModelForCausalLM, AutoTokenizer
    tok = AutoTokenizer.from_pretrained(args.base)
    tok.add_tokens(["<call>", "</call>"], special_tokens=True)
    call_id = tok.convert_tokens_to_ids("<call>")
    model = AutoModelForCausalLM.from_pretrained(args.base, dtype=torch.float32)
    model.resize_token_embeddings(len(tok))
    model.to(device)

    # freeze all but the top-N blocks + the new-token embedding rows + lm_head (tied)
    for p in model.parameters():
        p.requires_grad_(False)
    for blk in model.model.layers[-args.unfreeze_top:]:
        for p in blk.parameters():
            p.requires_grad_(True)
    for p in model.model.norm.parameters():
        p.requires_grad_(True)
    model.get_input_embeddings().weight.requires_grad_(True)  # incl. new <call>/</call> rows
    trainable = [p for p in model.parameters() if p.requires_grad]
    print(f"== CN-6 {args.arm} on {device} | trainable {sum(p.numel() for p in trainable):,} ==", flush=True)

    rows = [json.loads(l) for l in (HERE / f"cn6_corpus_train_{args.arm}{TAG}.jsonl").read_text().splitlines() if l.strip()]
    if args.smoke:
        rows = rows[:800]
    data = []
    for r in rows:
        ids = [tok.bos_token_id] + tok.encode(r["text"], add_special_tokens=False)
        if len(ids) > args.max_len or call_id not in ids:
            continue
        cpos = ids.index(call_id)
        data.append((ids, cpos))
    print(f"  train sequences: {len(data)}", flush=True)

    opt = torch.optim.AdamW(trainable, lr=args.lr)
    sched = torch.optim.lr_scheduler.LambdaLR(opt, lambda s: max(0.0, 1 - s / args.steps))
    PAD = tok.pad_token_id if tok.pad_token_id is not None else tok.eos_token_id

    def batches():
        order = list(range(len(data))); rng.shuffle(order)
        for i in range(0, len(data), args.bs):
            chunk = [data[j] for j in order[i:i + args.bs]]
            m = max(len(s) for s, _ in chunk)
            ids = torch.full((len(chunk), m), PAD, dtype=torch.long)
            amask = torch.zeros((len(chunk), m), dtype=torch.long)
            lmask = torch.zeros((len(chunk), m), dtype=torch.long)  # loss only on spec (>= cpos)
            for k, (s, cp) in enumerate(chunk):
                ids[k, :len(s)] = torch.tensor(s); amask[k, :len(s)] = 1; lmask[k, cp:len(s)] = 1
            yield ids.to(device), amask.to(device), lmask.to(device)

    step, losses = 0, []
    while step < args.steps:
        for ids, amask, lmask in batches():
            out = model(input_ids=ids, attention_mask=amask).logits  # (B,T,V)
            logits = out[:, :-1]; tgt = ids[:, 1:]; lm = lmask[:, 1:].float()
            ce = F.cross_entropy(logits.reshape(-1, logits.shape[-1]), tgt.reshape(-1), reduction="none")
            loss = (ce * lm.reshape(-1)).sum() / lm.sum().clamp(min=1)
            opt.zero_grad(); loss.backward(); opt.step(); sched.step()
            losses.append(float(loss)); step += 1
            if step % 50 == 0:
                print(f"  step {step:>4}/{args.steps}  spec-loss {sum(losses[-50:])/50:.4f}  ({time.time()-t0:.0f}s)", flush=True)
            if step >= args.steps:
                break

    tag = "" if args.base == "HuggingFaceTB/SmolLM2-135M" else "_" + args.base.split("/")[-1].replace(".", "").lower()
    ckpt = HERE / f"cn6_ckpt_{args.arm}{tag}.pt"
    torch.save({"arm": args.arm, "base_id": args.base, "input_max": args.input_max,
                "base": model.state_dict(), "call_id": call_id, "vocab": len(tok)}, ckpt)
    print(f"saved {ckpt.name} ({time.time()-t0:.0f}s)")


if __name__ == "__main__":
    main()
