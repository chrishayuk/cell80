#!/usr/bin/env python3
"""CN-10 per-layer readout (draft prereg §3 route (b), §4.1 instruments) — feasibility smoke.

Extracts every TransformerBlock's residual output via forward hooks and projects each
through the model's own final RMSNorm + tied lm_head (logit lens). Per layer, at the
prompt-final position: KL(lens_l || final), top-1 agreement with the final layer, and
the rank of the reference next token. This run is GATE MACHINERY, not a measurement:
no probes are trained, no boundary criterion is applied, prompts are fresh literals
(the seed-90 eval sets stay untouched). The §4.1 boundary criterion stays TO-PIN and
nothing here is graded against it.

Smoke prompts are 10 fixed in-range additions in the CN-8 surface `{A} + {B} =`.
Tokenizer = cn8_corpus.SP_MODEL (the arms' own training encoder; identity asserted
against the v11 vocab). CPU by default — MPS belongs to the CN-8 training chain.

Run: python3 cn10_readout.py            (base v11)
     python3 cn10_readout.py --ckpt cn8_ckpt_b_s80.pt
"""
from __future__ import annotations

import argparse
import hashlib
import json
import time
from pathlib import Path

import torch
import torch.nn.functional as F

import cn1_model
from cn8_corpus import SP_MODEL
from artifact_paths import checkpoint_input

HERE = Path(__file__).resolve().parent
SP_VOCAB = 71261

SMOKE_PROBLEMS = [(7, 5), (23, 9), (58, 67), (104, 91), (386, 245),
                  (901, 99), (1234, 876), (4055, 3966), (7002, 2999), (8641, 1359)]


@torch.no_grad()
def layer_lens(model, ids):
    """ids: (1, S). Returns list over layers of lens logits at the final position, plus final logits."""
    resids = []
    hooks = [layer.register_forward_hook(lambda _m, _i, out, acc=resids: acc.append(out[:, -1].detach()))
             for layer in model.layers]
    try:
        final_logits = model(ids)[0, -1]
    finally:
        for h in hooks:
            h.remove()
    assert len(resids) == len(model.layers), "hook count mismatch"
    lens = [model.lm_head(model.norm(r))[0] for r in resids]
    return lens, final_logits


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ckpt", default=None, help="cn8 checkpoint; default raw v11")
    ap.add_argument("--device", default="cpu", help="cpu default: MPS belongs to the cn8 chain")
    args = ap.parse_args()
    t0 = time.time()

    import sentencepiece as spm
    sp = spm.SentencePieceProcessor(model_file=SP_MODEL)
    assert sp.GetPieceSize() == SP_VOCAB, \
        f"tokenizer identity FAILED: {SP_MODEL} has {sp.GetPieceSize()} pieces, model vocab {SP_VOCAB}"

    from tiny_model_v11.loader import load_from_artifacts
    base, cfg = load_from_artifacts(str(cn1_model.TINY_MODEL / "model" / "v11"), device="cpu")
    tag = "raw_v11"
    if args.ckpt:
        ck = torch.load(checkpoint_input(args.ckpt), map_location="cpu")
        base.load_state_dict(ck["state"])
        tag = Path(args.ckpt).stem.replace("cn8_ckpt_", "")
    base = base.to(args.device).eval()
    n_layers = len(base.layers)
    print(f"== CN-10 readout smoke [{tag}] on {args.device} | {n_layers} layers | "
          f"tokenizer {Path(SP_MODEL).name} ({sp.GetPieceSize()} pieces, identity OK) ==", flush=True)

    def settle_depth(per_layer):
        return next((i for i in range(n_layers)
                     if all(pl["top1_agrees_final"] for pl in per_layer[i:])), n_layers)

    digit_ids = {}
    for d in "0123456789":
        pid = sp.PieceToId(d)
        assert pid != sp.unk_id(), f"digit piece '{d}' not in vocab"
        digit_ids[d] = pid

    rows = []
    for a, b in SMOKE_PROBLEMS:
        prompt = f"{a} + {b} ="
        penc = sp.encode(prompt)
        cont = sp.encode(f"{prompt} {a + b} .")[len(penc):]
        # first digit-bearing answer token — the bare space piece is trivially rank 1 and carries no signal
        ref_off, ref_tok = next((k, t) for k, t in enumerate(cont)
                                if any(c.isdigit() for c in sp.IdToPiece(t)))
        # condition on the gold tokens preceding it so every lens reads the same prediction point
        ids = torch.tensor([penc + cont[:ref_off]], device=args.device)
        lens, final = layer_lens(base, ids)
        logp_final = F.log_softmax(final, -1)
        final_top1 = int(final.argmax())
        per_layer = []
        for lg in lens:
            logp = F.log_softmax(lg, -1)
            kl = float(F.kl_div(logp_final, logp, reduction="sum", log_target=True))
            rank = int((lg > lg[ref_tok]).sum()) + 1
            per_layer.append({"kl_to_final": round(kl, 4),
                              "top1_agrees_final": bool(int(lg.argmax()) == final_top1),
                              "ref_tok_rank": rank})
        # digit-prior control: rank of EVERY digit piece at this position. The operand-shuffle
        # row falls out of this table — the correct digit's rank under a permuted prompt is
        # just its rank in that prompt's own distribution.
        digit_ranks = {d: int((final > final[pid]).sum()) + 1 for d, pid in digit_ids.items()}
        rows.append({"prompt": prompt, "ref_next_token": sp.IdToPiece(ref_tok),
                     "final_top1": sp.IdToPiece(final_top1), "layers": per_layer,
                     "digit_ranks_final": digit_ranks, "settle_layer": settle_depth(per_layer)})
        print(f"  {prompt:<14} ref '{rows[-1]['ref_next_token']}' rank@final "
              f"{per_layer[-1]['ref_tok_rank']:>6} | top1==final from layer "
              f"{rows[-1]['settle_layer']:>2} | KL L0 {per_layer[0]['kl_to_final']:.2f} -> "
              f"L{n_layers - 1} {per_layer[-1]['kl_to_final']:.2f}", flush=True)

    # --- operand-shuffle statistic (digit-prior vs operand-dependence) --------------------
    print("  -- digit-prior control (matched rank vs same digit's rank in the other prompts) --",
          flush=True)
    shuffle_rows = []
    for i, r in enumerate(rows):
        d = r["ref_next_token"]
        matched = r["digit_ranks_final"][d]
        others = sorted(rows[j]["digit_ranks_final"][d] for j in range(len(rows)) if j != i)
        med = others[len(others) // 2]
        shuffle_rows.append({"prompt": r["prompt"], "digit": d, "matched_rank": matched,
                             "shuffled_ranks": others, "shuffled_median": med})
        print(f"  {r['prompt']:<14} '{d}': matched {matched:>4} | shuffled median {med:>4} "
              f"(range {others[0]}-{others[-1]})", flush=True)

    # --- narrative settling baseline (is late settling the lens, or the task?) ------------
    import cn8_train
    narr = cn8_train.tinystories_val()[:10]
    narr_rows = []
    for r in narr:
        ctx = r["ids"][:24]
        if len(r["ids"]) < 26:
            continue
        lens, final = layer_lens(base, torch.tensor([ctx], device=args.device))
        logp_final = F.log_softmax(final, -1)
        final_top1 = int(final.argmax())
        per_layer = []
        for lg in lens:
            logp = F.log_softmax(lg, -1)
            per_layer.append({"kl_to_final": round(float(F.kl_div(
                logp_final, logp, reduction="sum", log_target=True)), 4),
                "top1_agrees_final": bool(int(lg.argmax()) == final_top1)})
        narr_rows.append({"ctx_tail": sp.decode(ctx[-8:]), "final_top1": sp.IdToPiece(final_top1),
                          "layers": per_layer, "settle_layer": settle_depth(per_layer)})
    arith_settles = sorted(r["settle_layer"] for r in rows)
    narr_settles = sorted(r["settle_layer"] for r in narr_rows)
    print(f"  -- settle-depth baseline: arithmetic {arith_settles} | "
          f"narrative(TinyStories s4) {narr_settles} --", flush=True)

    manifest = {"tag": tag, "n_layers": n_layers, "tokenizer": SP_MODEL,
                "tokenizer_pieces": sp.GetPieceSize(),
                "tokenizer_sha256": hashlib.sha256(Path(SP_MODEL).read_bytes()).hexdigest(),
                "smoke_problems": SMOKE_PROBLEMS, "rows": rows,
                "digit_prior_control": shuffle_rows,
                "narrative_baseline": narr_rows,
                "note": "feasibility-gate machinery run; no probes trained, no boundary criterion applied"}
    out = HERE / f"cn10_readout_smoke_{tag}.json"
    out.write_text(json.dumps(manifest, indent=1))
    print(f"wrote {out.name} ({time.time()-t0:.0f}s)")


if __name__ == "__main__":
    main()
