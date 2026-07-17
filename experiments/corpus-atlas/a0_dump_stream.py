#!/usr/bin/env python3
"""corpus-atlas gate A0: materialize the v11 pretrain token stream.

Replicates the data path of chris-experiments/compilation/15_v11_model/
train_v11_tinystories.py exactly, data-side only (no model, no torch):
streaming HF roneneldan/TinyStories train split, .shuffle(seed,
buffer_size=10000), BOS-prefixed SentencePiece encode truncated at
max_seq*2 = 512 ids, concatenated into 256-token chunks, stop when
tokens_seen would exceed the phase budget (the overshooting chunk and the
leftover buffer are discarded, exactly as in training). Phase 1 = 16M
tokens @ seed 42, phase 3 = 8M tokens @ seed 43.

Emits one uint32 array per phase plus a manifest with sha256 hashes,
tokenizer hash, HF dataset revision, and library versions. The SP id space
is 71,261 pieces (so u16 would overflow); the training-exercised subset is
8,599 ids (v11_train_mask.pt) — the manifest records each phase's distinct-id
census for the cross-check.

Determinism audit: run twice (--tag run1 / --tag run2); per-phase sha256
must match across runs. Usage:

  python3 a0_dump_stream.py --tag run1
"""

import argparse
import hashlib
import json
import platform
import sys
from array import array
from pathlib import Path

import sentencepiece as spm

HERE = Path(__file__).resolve().parent
V11_DIR = Path.home() / "chris-source" / "chris-experiments" / "compilation" / "15_v11_model"
V11_MODEL_PATH = V11_DIR / "v11_tokenizer" / "v11.model"

MAX_SEQ = 256
SEED = 42
PHASES = [
    {"phase": 1, "seed": SEED, "max_tokens": 16_000_000},
    {"phase": 3, "seed": SEED + 1, "max_tokens": 8_000_000},
]
EXPECTED_VOCAB = 71261  # full SP piece space; 8,599 of these are train-exercised


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def encode(sp, bos_id, text, max_length):
    # V11Tokenizer.encode(add_special_tokens=True, truncation=True)
    ids = sp.encode(text)
    if bos_id >= 0:
        ids = [bos_id] + ids
    if len(ids) > max_length:
        ids = ids[:max_length]
    return ids


def dump_phase(sp, bos_id, phase, seed, max_tokens, out_path):
    from datasets import load_dataset
    ds = load_dataset("roneneldan/TinyStories", split="train", streaming=True)
    ds = ds.shuffle(seed=seed, buffer_size=10000)

    out = array("I")
    seen_ids = bytearray(EXPECTED_VOCAB)
    tokens_seen = 0
    samples_consumed = 0
    buffer = []
    done = False
    for sample in ds:
        samples_consumed += 1
        ids = encode(sp, bos_id, sample["text"], MAX_SEQ * 2)
        buffer.extend(ids)
        while len(buffer) >= MAX_SEQ:
            chunk = buffer[:MAX_SEQ]
            buffer = buffer[MAX_SEQ:]
            tokens_seen += len(chunk)
            if tokens_seen > max_tokens:
                done = True
                break
            out.extend(chunk)
            for i in chunk:
                seen_ids[i] = 1
        if done:
            break
    assert sys.byteorder == "little", "u32 dump is defined little-endian"
    assert array("I").itemsize == 4
    with open(out_path, "wb") as f:
        out.tofile(f)
    n = len(out)
    print(f"  phase {phase}: {n:,} tokens ({n // MAX_SEQ:,} chunks), "
          f"{samples_consumed:,} stories consumed -> {out_path.name}")
    assert n == min(max_tokens, (max_tokens // MAX_SEQ) * MAX_SEQ)
    return {
        "phase": phase,
        "seed": seed,
        "max_tokens": max_tokens,
        "tokens_dumped": n,
        "chunks": n // MAX_SEQ,
        "stories_consumed": samples_consumed,
        "distinct_ids": sum(seen_ids),
        "dtype": "uint32-le",
        "file": out_path.name,
        "sha256": sha256_file(out_path),
        "first_16_ids": list(out[:16]),
        "last_16_ids": list(out[-16:]),
    }


def hub_revision():
    try:
        from huggingface_hub import HfApi
        info = HfApi().dataset_info("roneneldan/TinyStories")
        return {"sha": info.sha, "last_modified": str(info.last_modified)}
    except Exception as e:  # network-optional; recorded as unavailable
        return {"error": repr(e)}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--tag", required=True, help="run1 / run2 (determinism audit)")
    args = ap.parse_args()

    import datasets
    sp = spm.SentencePieceProcessor()
    sp.load(str(V11_MODEL_PATH))
    vocab = sp.get_piece_size()
    assert vocab == EXPECTED_VOCAB, f"tokenizer identity: {vocab} != {EXPECTED_VOCAB}"
    bos_id = sp.bos_id()

    print(f"[a0 {args.tag}] vocab={vocab} bos_id={bos_id} "
          f"datasets={datasets.__version__} sentencepiece={spm.__version__}")

    manifest = {
        "gate": "A0",
        "tag": args.tag,
        "tokenizer": {
            "path": str(V11_MODEL_PATH),
            "sha256": sha256_file(V11_MODEL_PATH),
            "vocab_size": vocab,
            "bos_id": bos_id,
        },
        "dataset": {"name": "roneneldan/TinyStories", "split": "train",
                     "streaming": True, "shuffle_buffer": 10000,
                     "hub": hub_revision()},
        "env": {
            "python": platform.python_version(),
            "datasets": datasets.__version__,
            "sentencepiece": spm.__version__,
            "byteorder": sys.byteorder,
        },
        "max_seq": MAX_SEQ,
        "phases": [],
    }
    for spec in PHASES:
        out_path = HERE / f"v11_stream_phase{spec['phase']}_{args.tag}.u32"
        manifest["phases"].append(
            dump_phase(sp, bos_id, spec["phase"], spec["seed"],
                        spec["max_tokens"], out_path))

    man_path = HERE / f"a0_manifest_{args.tag}.json"
    with open(man_path, "w") as f:
        json.dump(manifest, f, indent=2)
    print(f"[a0 {args.tag}] manifest -> {man_path.name}")
    for p in manifest["phases"]:
        print(f"  phase {p['phase']} sha256 {p['sha256']}")
    sys.stdout.flush()
    # datasets' streaming leaves non-daemon threads that block interpreter
    # shutdown; all artifacts are flushed above, so exit hard.
    import os
    os._exit(0)


if __name__ == "__main__":
    main()
