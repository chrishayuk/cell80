#!/usr/bin/env python3
"""Export a CN-7 checkpoint for `repl.py` / `load_from_artifacts` (plain, native-vocab).

`cn7_train.py` always resizes the tied embed/lm_head to the full ~790-cell vocab
(72,053 rows; see `cn1_model.resize_embedding`) regardless of which corpus species were
included, and saves a wrapper dict (`{"arm", "vocab", ..., "state": state_dict()}`), not
a plain `state_dict()`. Neither shape drops into `tiny_model_v11.loader.load_from_artifacts`
(used by `repl.py`/`cold_open.py`), which builds a model at the native SP vocab (71,261,
from `model/v11/config.json`) and strict-loads a plain state dict.

For a checkpoint whose corpus never referenced `<call>`/cell tokens at all (e.g. the
math-only S1+S4 control arm — verified by `cn7_audit.py`'s held-out/HELD-OUT check
finding zero such references), rows 71261+ of embed/lm_head are untrained xavier-init
noise the tokenizer never produces ids for. Truncating them off is behavior-preserving:
the model was equivalent to a native-vocab model all along, this just makes that
equivalence a matter of shape.

Run: python3 cn7_export_repl_ckpt.py --ckpt cn7_ckpt_midtrain_mathonly.pt \
    --out ~/chris-source/tiny-model/model/v11/artifacts/model_mathonly.pt
"""
from __future__ import annotations

import argparse
from pathlib import Path

import torch

from artifact_paths import checkpoint_input

NATIVE_VOCAB = 71261


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ckpt", required=True, help="CN-7 checkpoint name/path (wrapper dict format)")
    ap.add_argument("--out", required=True, help="output path for the plain state_dict "
                     "(e.g. .../tiny-model/model/v11/artifacts/model_mathonly.pt)")
    ap.add_argument("--native-vocab", type=int, default=NATIVE_VOCAB)
    args = ap.parse_args()

    ck = torch.load(checkpoint_input(args.ckpt), map_location="cpu")
    state = ck["state"] if "state" in ck else ck
    vocab = ck.get("vocab", state["embed.weight"].shape[0])
    if vocab < args.native_vocab:
        raise SystemExit(f"checkpoint vocab {vocab} is smaller than native vocab "
                          f"{args.native_vocab} — nothing to truncate, this isn't a resized "
                          f"CN-7 checkpoint")

    extra_rows = state["embed.weight"][args.native_vocab:]
    if extra_rows.shape[0] and float(extra_rows.abs().max()) == 0.0:
        raise SystemExit("extra embedding rows are all-zero, not xavier-init noise — "
                          "this checkpoint may not be what you think it is; aborting")

    new_state = dict(state)
    new_state["embed.weight"] = state["embed.weight"][:args.native_vocab].clone()
    new_state["lm_head.weight"] = state["lm_head.weight"][:args.native_vocab].clone()

    out_path = Path(args.out).expanduser()
    if out_path.exists():
        raise SystemExit(f"REFUSING to overwrite existing {out_path} — move it aside first")
    out_path.parent.mkdir(parents=True, exist_ok=True)
    torch.save(new_state, out_path)
    print(f"wrote {out_path} (embed/lm_head truncated {vocab} -> {args.native_vocab} rows, "
          f"{len(new_state)} keys)")


if __name__ == "__main__":
    main()
