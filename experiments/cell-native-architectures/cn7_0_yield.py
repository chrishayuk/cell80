#!/usr/bin/env python3
"""CN-7.0 — yield curve (pre-registration `experiments/cell-native-architectures-cn7-preregistration.md` §4).

Sample a checkpoint's emissions with the EXACT CN-6 stage-2 prompting (descriptor context +
` <call>`, spec grammar `a b = r ; ...`), cell-sign every parsed pair by executing the true cell
on the emitted inputs, and report signed-pair yield per cell and per frontier stratum
(within/beyond, from cn7_frontier_classification.json). No training; measurement only.

Substrates:
  --ckpt-hf PATH : a CN-6 HF checkpoint (SmolLM2/Llama) — the substrate B2=0.097 was measured on;
                   this is the bridge line that makes the 7.0 prediction gradeable as written.
  --v11          : the RAW pretrained TinyModel v11 (no emission finetune) — the midtrain target's
                   own pre-midtrain baseline. Expected near-zero in-format yield (v11 speaks the
                   cell-CALL grammar from pretrain, not the I/O-spec grammar); that floor is the
                   number 7.2's midtrain (species S3) must move.

Sampling protocol (fixed): per held-out cell, 1 greedy + --nsample sampled emissions at --temp,
48 generated tokens each (the CN-6 budget). Per-pair signed yield = signed / parsed, Wilson CIs.

Run: python3 cn7_0_yield.py --ckpt-hf cn6_ckpt_generation.pt
     python3 cn7_0_yield.py --v11
"""
from __future__ import annotations

import argparse
import json
import math
import re
from pathlib import Path

import torch

import cell80_py
from artifact_paths import checkpoint_input, dataset_input

HERE = Path(__file__).resolve().parent
CELLS_DIR = HERE.parent.parent / "cell80" / "cells"
GEN_BUDGET = 47  # tokens after the first, matching cn6_eval


def wilson(k, n):
    if n == 0:
        return (0.0, 0.0, 0.0)
    p = k / n
    z = 1.96
    d = 1 + z * z / n
    c = (p + z * z / (2 * n)) / d
    h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / d
    return p, max(0, c - h), min(1, c + h)


def parse_spec(text):
    out = []
    for chunk in text.split(";"):
        if "=" not in chunk:
            continue
        lhs, rhs = chunk.split("=", 1)
        nums = re.findall(r"-?\d+", lhs)
        rnum = re.findall(r"-?\d+", rhs)
        if nums and rnum:
            out.append(([int(x) & 0xFFFF for x in nums], int(rnum[0]) & 0xFFFF))
    return out


def hf_sampler(ckpt_path):
    from transformers import AutoModelForCausalLM, AutoTokenizer
    ck = torch.load(ckpt_path, map_location="cpu")
    base_id = ck.get("base_id", "HuggingFaceTB/SmolLM2-135M")
    tok = AutoTokenizer.from_pretrained(base_id)
    tok.add_tokens(["<call>", "</call>"], special_tokens=True)
    close_id = tok.convert_tokens_to_ids("</call>")
    model = AutoModelForCausalLM.from_pretrained(base_id, dtype=torch.float32)
    model.resize_token_embeddings(ck["vocab"])
    model.load_state_dict(ck["base"])
    model.to("cpu").eval()
    print(f"  substrate: {ckpt_path.name} (base {base_id}, input-max {ck.get('input_max', 1000)})", flush=True)

    @torch.no_grad()
    def gen(context, temp):
        prompt = [tok.bos_token_id] + tok.encode(context + " <call>", add_special_tokens=False)
        out = model(input_ids=torch.tensor([prompt]), use_cache=True)
        past = out.past_key_values

        def pick(lg):
            return int(torch.multinomial(torch.softmax(lg / temp, -1), 1)) if temp > 0 else int(lg.argmax())

        toks = [pick(out.logits[0, -1])]
        for _ in range(GEN_BUDGET):
            if toks[-1] == close_id:
                break
            out = model(input_ids=torch.tensor([[toks[-1]]]), past_key_values=past, use_cache=True)
            past = out.past_key_values
            toks.append(pick(out.logits[0, -1]))
        gen_txt = tok.decode(prompt + toks)
        return gen_txt.split("<call>", 1)[-1].split("</call>", 1)[0]

    return gen, f"hf:{ckpt_path.name}"


SP_MODEL = "/Users/christopherhay/chris-source/chris-experiments/compilation/15_v11_model/v11_tokenizer/v11.model"


def v11_sampler():
    """Raw pretrained TinyModel v11 under its ORIGINAL SP tokenizer (the training id space).

    Discovered 2026-07-15 while wiring this script: the tiny-model repo's committed tokenizer
    artifacts (v11.vocab.bin / tokenizer.json) are a DIFFERENT piece->id mapping than the one
    v11 was trained with; only artifacts/train_mask.pt + the SP model recovered from
    chris-experiments/compilation/15_v11_model make the checkpoint decodable (NLL 0.66 on
    TinyStories vs 18.0 = worse-than-uniform through the committed mapping). The CN-1 lane
    encoded through the committed mapping: only ~30%% of its context tokens hit trained rows.
    Decode here is restricted to trained ids (train_mask) — the other ~62k rows are init noise.
    '<call>'/'</call>' do not exist in the SP vocab (encode to unk): raw v11 has never seen the
    emission grammar, so this arm measures the true pre-midtrain in-format floor.
    """
    import sentencepiece as spm
    import cn1_model  # sets up the tiny-model sys.path
    from tiny_model_v11.loader import load_from_artifacts
    sp = spm.SentencePieceProcessor(model_file=SP_MODEL)
    base, cfg = load_from_artifacts(str(cn1_model.TINY_MODEL / "model" / "v11"), device="cpu")
    base.eval()
    mask = torch.load(str(cn1_model.TINY_MODEL / "model" / "v11" / "artifacts" / "train_mask.pt"),
                      map_location="cpu")
    neg = torch.full((cn1_model.BASE_VOCAB_ROWS,), float("-inf"))
    neg[mask] = 0.0
    print(f"  substrate: raw TinyModel v11, original SP mapping ({int(mask.sum())} trained ids)", flush=True)

    @torch.no_grad()
    def gen(context, temp):
        ids = [sp.bos_id()] + sp.encode(context + " <call>")
        toks = []
        for _ in range(GEN_BUDGET + 1):
            logits = base(torch.tensor([ids + toks]))[0, -1][:cn1_model.BASE_VOCAB_ROWS] + neg
            if temp > 0:
                nxt = int(torch.multinomial(torch.softmax(logits / temp, -1), 1))
            else:
                nxt = int(logits.argmax())
            if nxt == sp.eos_id():
                break
            toks.append(nxt)
        txt = sp.decode(toks)
        return txt.split("</call>", 1)[0]

    return gen, "v11:raw-pretrain-sp"


def v11_ckpt_sampler(ckpt_name):
    """Midtrained v11 (SP space + cn7 vocab), legality-masked decode — the 7.6 gate input."""
    import cn1_model
    from cn1_model import resize_embedding
    from cn7_corpus import Enc as CN7Enc
    from cn7_deck import decode_mask
    from tiny_model_v11.loader import load_from_artifacts
    import json as _json
    tokmap = _json.load(open(HERE / "cn7_token_map.json"))
    enc = CN7Enc(tokmap["cells"])
    base, cfg = load_from_artifacts(str(cn1_model.TINY_MODEL / "model" / "v11"), device="cpu")
    ck = torch.load(checkpoint_input(ckpt_name), map_location="cpu")
    resize_embedding(base, ck["vocab"])
    base.load_state_dict(ck["state"])
    base.eval()
    neg = torch.full((ck["vocab"],), float("-inf"))
    neg[decode_mask(ck["vocab"])] = 0.0
    CALL, CLOSE = 71261, 71262
    print(f"  substrate: {ckpt_name} (midtrained v11, SP space)", flush=True)

    @torch.no_grad()
    def gen(context, temp):
        ids = enc.seg_ids(context) + [CALL]
        toks = []
        for _ in range(GEN_BUDGET + 1):
            logits = base(torch.tensor([ids + toks]))[0, -1] + neg
            if temp > 0:
                nxt = int(torch.multinomial(torch.softmax(logits / temp, -1), 1))
            else:
                nxt = int(logits.argmax())
            if nxt in (CLOSE, 3):
                break
            toks.append(nxt)
        return enc.sp.decode([t for t in toks if t < 71261])

    return gen, f"v11:{Path(ckpt_name).stem}"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ckpt-hf", default=None)
    ap.add_argument("--ckpt-v11", default=None, help="midtrained v11 checkpoint (7.6 gate input)")
    ap.add_argument("--v11", action="store_true")
    ap.add_argument("--nsample", type=int, default=8)
    ap.add_argument("--temp", type=float, default=0.7)
    ap.add_argument("--seed", type=int, default=80)
    ap.add_argument("--out", default=None)
    args = ap.parse_args()
    torch.manual_seed(args.seed)

    lib = {json.loads(l)["name"]: json.loads(l) for l in (HERE / "cn1_library.jsonl").read_text().splitlines() if l.strip()}
    strata = json.load(open(HERE / "cn7_frontier_classification.json"))["cells"]
    ev = [json.loads(l) for l in dataset_input("cn6_corpus_eval_generation.jsonl").read_text().splitlines() if l.strip()]
    contexts = {}
    for r in ev:
        contexts.setdefault(r["cell"], r["context"])
    assert set(contexts) == set(strata), "held-out set and classification disagree"

    host = cell80_py.CellHost()
    handles = {}
    for n in contexts:
        host.add_source(n, next(CELLS_DIR.rglob(f"{n}.rs")).read_text())
        handles[n] = host.load(n)

    if args.ckpt_hf:
        gen, tag = hf_sampler(checkpoint_input(args.ckpt_hf))
    elif args.ckpt_v11:
        gen, tag = v11_ckpt_sampler(args.ckpt_v11)
    elif args.v11:
        gen, tag = v11_sampler()
    else:
        raise SystemExit("pass --ckpt-hf PATH, --ckpt-v11 PATH, or --v11")

    def sign(name, pairs):
        ok = []
        for a, o in pairs:
            try:
                r = host.run(handles[name], list(a))
                ok.append(bool(r.get("halt") == "returned" and r["result"] == o))
            except Exception:
                ok.append(False)
        return ok

    per_cell = {}
    for name in sorted(contexts):
        parsed, signed, emissions, all_pairs = 0, 0, [], []
        for i in range(1 + args.nsample):
            temp = 0.0 if i == 0 else args.temp
            seg = gen(contexts[name], temp)
            pairs = parse_spec(seg)
            oks = sign(name, pairs)
            parsed += len(pairs)
            signed += sum(oks)
            all_pairs.extend(pairs)
            emissions.append({"temp": temp, "raw": seg.strip()[:120], "parsed": len(pairs), "signed": sum(oks)})
        y = signed / parsed if parsed else 0.0
        # permutation null: signing rate when THIS cell's emitted answers are shuffled against
        # its emitted inputs — the chance-of-signing baseline a raw yield must beat
        # ([[frequency-needs-drift-baseline]]: never read a frequency without its null)
        null = 0.0
        if all_pairs:
            truths = []
            for a, _ in all_pairs:
                try:
                    r = host.run(handles[name], list(a))
                    truths.append(r["result"] if r.get("halt") == "returned" else None)
                except Exception:
                    truths.append(None)
            answers = [o for _, o in all_pairs]
            hits = sum(1 for t in truths for o in answers if t is not None and t == o)
            null = hits / (len(truths) * len(answers))
        per_cell[name] = {"stratum": strata[name]["stratum"], "parsed_pairs": parsed,
                          "signed_pairs": signed, "yield": y, "null": round(null, 4),
                          "excess": round(y - null, 4),
                          "pairs": [[a, o] for a, o in all_pairs], "emissions": emissions}
        print(f"  {name:<32} [{strata[name]['stratum']:<6}] parsed {parsed:>3}  signed {signed:>3}  "
              f"yield {y:.3f}  null {null:.3f}  excess {y-null:+.3f}", flush=True)

    print(f"\nCN-7.0 yield — substrate {tag} | greedy + {args.nsample} samples @ T={args.temp} per cell")
    summary = {}
    for st in ("within", "beyond"):
        cells = [c for c in per_cell.values() if c["stratum"] == st]
        k = sum(c["signed_pairs"] for c in cells)
        n = sum(c["parsed_pairs"] for c in cells)
        p, lo, hi = wilson(k, n)
        e_null = sum(c["null"] * c["parsed_pairs"] for c in cells) / max(1, n)
        summary[st] = {"cells": len(cells), "signed": k, "parsed": n, "yield": p, "ci95": [lo, hi],
                       "null": round(e_null, 4), "excess": round(p - e_null, 4)}
        print(f"  {st:<7} ({len(cells)} cells): signed {k}/{n} parsed  ->  per-pair yield {p:.3f} [{lo:.3f},{hi:.3f}]"
              f"  null {e_null:.3f}  EXCESS {p - e_null:+.3f}")

    out = HERE / (args.out or f"cn7_0_yield_{tag.replace(':', '_').replace('.', '')}.json")
    out.write_text(json.dumps({"substrate": tag, "protocol": {"nsample": args.nsample, "temp": args.temp,
                               "seed": args.seed, "budget": GEN_BUDGET + 1, "prompting": "cn6-stage2-generation"},
                               "summary": summary, "per_cell": per_cell}, indent=1))
    print(f"saved {out.name}")


if __name__ == "__main__":
    main()
