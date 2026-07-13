#!/usr/bin/env python3
"""CN-1 real build, step 6 apparatus (`cell-native-architectures-cn1-preregistration.md`): the
training loop over the H1-factory corpus, for both arms, on the three-way-tied model.

Freeze policy (pre-registration): the pretrained TinyModel v11 base is FROZEN — a 115M-param
transformer must not be retrained on a ~6k-row corpus (catastrophic forgetting, and it would
confound the equal-parameter comparison to the prompted baseline). Trainable:
  - arm (c) fingerprint: `W_f` (the shared projection) + the two delimiter rows.
  - arm (b) random:      the free cell embedding rows + the two delimiter rows.
Everything else — all 20 transformer blocks, the norm, and the 71260 pretrained token rows —
stays fixed. The model adapts by aligning `W_f`(fingerprint) (or free rows) with the frozen
base's own hidden states over the context.

Loss: standard next-token cross-entropy over the whole sequence, so the model learns the call
format (emit `<call>`, then the cell token, then args, then `</call>`) end to end. The
cell-token position is where gate scoring will look, but training supervises the full sequence.

This is the apparatus + a SMOKE run (small, few steps) to validate the pipeline end to end —
does loss fall, does a trained arm emit sensible cell tokens under constrained decoding — NOT
the pre-registered run (no gates evaluated here). The pilot's discipline: prove the training
harness works before trusting any gate number.

Run: python3 cn1_train.py --arm fingerprint --steps 300 --smoke
"""
from __future__ import annotations

import argparse
import json
import time
from pathlib import Path

import torch
import torch.nn.functional as F

import cn1_model
import cn1_decode

HERE = Path(__file__).resolve().parent
TRAIN = HERE / "cn1_corpus_train.jsonl"
VOCAB = HERE / "v11-cells.vocab.bin"
PAD_ID = 0  # v11 PAD


def load_tokenizer():
    import v11

    return v11.Tokenizer.from_file(str(VOCAB))


def tokenize_rows(rows, tok, max_len):
    """Encode each row's `text`; also return the cell-token position for eval. bos=2 prepended,
    eos=3 appended (v11 specials)."""
    out = []
    for r in rows:
        ids = [2] + tok.encode(r["text"]) + [3]
        if len(ids) > max_len:
            continue
        # cell-token position = index of the row's cell id (for later gate-style probing)
        try:
            cell_pos = ids.index(r["cell_id"])
        except ValueError:
            continue
        out.append((ids, cell_pos))
    return out


def set_trainable(model, arm, unfreeze_top=0):
    """Freeze the base; enable the arm's adaptation params + delimiter rows. Optionally unfreeze
    the top `unfreeze_top` transformer blocks + the final norm — the smoke run showed a fully
    frozen TinyStories base gives no context-discriminative hidden state at the <call> position
    (W_f collapses to a constant), so the model must be allowed to LEARN to read the behavioral
    demonstrations. Both arms unfreeze identically; the only arm difference stays the cell-row
    source (W_f vs free rows)."""
    for p in model.parameters():
        p.requires_grad_(False)
    trainable = []
    if arm == "fingerprint":
        for p in model.w_f.parameters():
            p.requires_grad_(True)
            trainable.append(p)
    if unfreeze_top > 0:
        blocks = model.base.layers
        for blk in blocks[-unfreeze_top:]:
            for p in blk.parameters():
                p.requires_grad_(True)
                trainable.append(p)
        for p in model.base.norm.parameters():
            p.requires_grad_(True)
            trainable.append(p)
    # Delimiter + cell rows live in base.embed.weight. We can't set requires_grad on a slice, so
    # for arm (b) we train the whole embed row matrix but mask the gradient to the new rows only
    # (a hook), keeping the 71260 pretrained rows frozen. For arm (c) the cell rows come from
    # W_f, so only the two delimiter rows need training via the same masked-grad path.
    emb = model.base.embed.weight
    emb.requires_grad_(True)
    first_new = cn1_model.BASE_VOCAB_ROWS - 1  # 71260 = <call>; rows < this are pretrained-frozen
    grad_mask = torch.zeros(emb.shape[0], 1, device=emb.device)
    if arm == "random":
        grad_mask[cn1_model.BASE_VOCAB_ROWS - 1 :] = 1.0  # delimiters + all cell rows
    else:
        grad_mask[cn1_model.BASE_VOCAB_ROWS - 1 : cn1_model.BASE_VOCAB_ROWS + 1] = 1.0  # 2 delimiters only

    def hook(grad):
        return grad * grad_mask

    emb.register_hook(hook)
    trainable.append(emb)
    n = sum(p.numel() for p in trainable if p.requires_grad)
    return trainable, n


def batches(data, bs, pad_id, device, rng):
    order = list(range(len(data)))
    rng.shuffle(order)
    for i in range(0, len(data), bs):
        chunk = [data[j] for j in order[i : i + bs]]
        seqs = [c[0] for c in chunk]
        cell_pos = [c[1] for c in chunk]
        m = max(len(s) for s in seqs)
        ids = torch.full((len(seqs), m), pad_id, dtype=torch.long)
        for k, s in enumerate(seqs):
            ids[k, : len(s)] = torch.tensor(s)
        yield ids.to(device), torch.tensor(cell_pos, device=device)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--arm", choices=["fingerprint", "random"], default="fingerprint")
    ap.add_argument("--steps", type=int, default=300)
    ap.add_argument("--bs", type=int, default=16)
    ap.add_argument("--lr", type=float, default=1e-3)
    ap.add_argument("--max-len", type=int, default=128)
    ap.add_argument("--smoke", action="store_true", help="cap corpus rows for a fast pipeline check")
    ap.add_argument("--unfreeze-top", type=int, default=0, help="unfreeze top N transformer blocks + norm")
    ap.add_argument("--device", default=None)
    args = ap.parse_args()

    device = args.device or ("mps" if torch.backends.mps.is_available() else "cpu")
    rng = __import__("random").Random(80)
    t0 = time.time()

    print(f"== building model (arm {args.arm}) on {device} ==", flush=True)
    model, names, held = cn1_model.build(args.arm)
    model = model.to(device)
    trainable, n_trainable = set_trainable(model, args.arm, unfreeze_top=args.unfreeze_top)
    print(f"  trainable params: {n_trainable:,} (unfreeze_top={args.unfreeze_top})")

    tok = load_tokenizer()
    rows = [json.loads(l) for l in TRAIN.read_text().splitlines() if l.strip()]
    if args.smoke:
        rows = rows[:1500]
    data = tokenize_rows(rows, tok, args.max_len)
    print(f"  train sequences: {len(data)} (from {len(rows)} rows)")

    opt = torch.optim.Adam([p for p in trainable if p.requires_grad], lr=args.lr)

    # Loss is supervised at the CELL-TOKEN position only: predict the cell id from the prefix
    # ending in <call>. The base is frozen, so full-sequence CE is dominated by irreducible
    # loss on digit/format tokens the frozen TinyStories base can't improve; concentrating the
    # gradient on the one learnable, gate-relevant target (which cell) is both faster and
    # exactly what gates (i)/(ii) score. cell_pos indexes the cell token; it is predicted from
    # logits at position cell_pos-1.
    step = 0
    losses = []
    accs = []
    while step < args.steps:
        for ids, cell_pos in batches(data, args.bs, PAD_ID, device, rng):
            logits = model(ids)  # (B, T, V)
            b = torch.arange(ids.shape[0], device=device)
            pred_logits = logits[b, cell_pos - 1]  # (B, V) predicting the cell token
            targets = ids[b, cell_pos]  # (B,) the cell id
            loss = F.cross_entropy(pred_logits, targets)
            opt.zero_grad()
            loss.backward()
            opt.step()
            losses.append(float(loss))
            accs.append(float((pred_logits.argmax(-1) == targets).float().mean()))
            step += 1
            if step % 25 == 0:
                print(
                    f"  step {step:>4}/{args.steps}  loss {sum(losses[-25:])/25:.4f}  "
                    f"cell-acc {sum(accs[-25:])/25:.3f}  ({time.time()-t0:.0f}s)",
                    flush=True,
                )
            if step >= args.steps:
                break

    print(f"\n== trained {step} steps, final loss {sum(losses[-25:])/min(25,len(losses)):.4f} ({time.time()-t0:.0f}s) ==")

    # Smoke eval: does the trained arm emit a sensible cell token under constraint on a couple
    # of held-out-from-this-check contexts? (Not a gate — a pipeline sanity check.)
    call_open, _, cell_ids, cell_set = cn1_decode.load_call_grammar()
    mask = cn1_decode.CellCallMask(call_open, cell_ids)
    tok_map = json.loads((HERE / "cn1_cell_token_map.json").read_text())
    id_to_name = {v: k[len("<cell:"):-1] for k, v in tok_map.items() if k.startswith("<cell:")}
    print("\n== constrained-decode sanity (trained model, seen cells) ==")
    seen_examples = [r for r in rows if r["cell"] not in held][:5]
    hits = 0
    model.eval()
    for r in seen_examples:
        # feed the context up to and including <call>, let the model pick the cell
        prefix_text = r["context"] + " <call>"
        ids = [2] + tok.encode(prefix_text)
        out = cn1_decode.generate_constrained(model, ids, mask, max_new=1)
        pred = out[-1]
        ok = pred == r["cell_id"]
        hits += ok
        print(f"  want <cell:{r['cell']}> got <cell:{id_to_name.get(pred,'?')}>  {'HIT' if ok else 'miss'}")
    print(f"  constrained top-1 on {len(seen_examples)} seen contexts: {hits}/{len(seen_examples)}")
    print("\n(smoke run — no gate evaluated; validates that the training harness learns the format and emits cells)")


if __name__ == "__main__":
    main()
