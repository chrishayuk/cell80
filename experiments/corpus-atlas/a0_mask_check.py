#!/usr/bin/env python3
"""corpus-atlas gate A0, cross-check 2: replicate v11_train_mask.pt bit-for-bit.

The mask (8,599 true of 71,261) is NOT a trained-ids census — it is the
decode whitelist built by compile_capitals_v11.py::build_train_mask: every
id appearing in the FIRST 5,000 stories of the UNSHUFFLED TinyStories
train stream, plus SP specials, plus a hand list of capitals. Replicating
that construction today and matching the cached artifact bit-for-bit
verifies, against a training-era artifact, that (a) the dataset content
and streaming order and (b) the tokenizer id mapping are unchanged since
the v11 era — evidence the run1/run2 self-consistency audit cannot provide.

First run 2026-07-17: replicated sum 8599 == reference sum 8599,
bit-identical True.
"""

import sys
from pathlib import Path

import sentencepiece as spm
import torch

V11_DIR = Path.home() / "chris-source" / "chris-experiments" / "compilation" / "15_v11_model"
MUST_KEEP = "France Paris England London Italy Rome Germany Berlin Russia Moscow the is a an"


def main():
    sp = spm.SentencePieceProcessor()
    sp.load(str(V11_DIR / "v11_tokenizer" / "v11.model"))
    V = sp.get_piece_size()

    seen = torch.zeros(V, dtype=torch.bool)
    for sid in [sp.pad_id(), sp.unk_id(), sp.bos_id(), sp.eos_id()]:
        if sid >= 0:
            seen[sid] = True

    from datasets import load_dataset
    ds = load_dataset("roneneldan/TinyStories", split="train", streaming=True)
    for i, ex in enumerate(ds):
        if i >= 5000:
            break
        for tid in sp.encode(ex["text"]):
            if 0 <= tid < V:
                seen[tid] = True
    for tid in sp.encode(MUST_KEEP):
        if 0 <= tid < V:
            seen[tid] = True

    ref = torch.load(V11_DIR / "v11_train_mask.pt", weights_only=False).bool()
    ok = bool((seen == ref).all())
    print(f"replicated sum: {int(seen.sum())} | reference sum: {int(ref.sum())}")
    print(f"bit-identical: {ok}")
    if not ok:
        diff = (seen != ref).nonzero().flatten().tolist()
        print(f"differing ids ({len(diff)}): {diff[:40]}")
    sys.stdout.flush()
    import os
    os._exit(0 if ok else 1)  # datasets streaming threads block clean exit


if __name__ == "__main__":
    main()
