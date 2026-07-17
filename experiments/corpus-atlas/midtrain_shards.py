#!/usr/bin/env python3
"""corpus-atlas wall-checker shards (spec §7): the midtrain corpora as
fingerprinted indexes, and the CN-8 band distances on the axis that
actually varies.

"Disjoint from all training" was quietly false for every post-midtrain
model while only the pretrain was indexed. This adds the midtrain
exposure as separately-fingerprinted shards: CN-7 (S1/S2/S3 template
corpus) and CN-8 (B / A-tok / A-ex). Matches must not cross training-row
boundaries (row sentinels, same reasoning as the pretrain's chunk
sentinels). Per the normalizer-authority ruling (equivalence audit),
template corpora are read at TWO levels: token-level surface (suffix
array over the corpus's own id streams, call tokens included) and
metrology-M1 skeleton (deterministic; the authority-holding normalizer
for template text) — skeleton-v1/spaCy is NOT used here.

Immediate deliverable: cn8_band_midtrain_distances.json — the frozen
B0/B1/B2 prompts scored against each arm's own training shard, committed
BEFORE any grading verdict exists. Pretrain-distance was flat across
bands (by design); this is the axis the P-m curve conditions on.

Also the FS-bank admission machinery: `check --text "…"` reports
per-shard surface and M1-skeleton matches; admission requires
skeleton-disjointness per the wall rule.

Build: python3 midtrain_shards.py build      (system python3; no spaCy)
Bands: python3 midtrain_shards.py bands
Check: python3 midtrain_shards.py check --text "..."
"""

import argparse
import hashlib
import json
import sys
from collections import Counter
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(Path.home() / "chris-source" / "v11-train-plan" / "div0"))
from atlas_surface import SENTINEL, V11_SP, build_suffix_array, sha256_file  # noqa: E402
from artifact_paths import index_input, index_output

CN_DIR = HERE.parent / "cell-native-architectures"


def cn_dataset(value):
    path = Path(value)
    artifact = CN_DIR / "artifacts" / "datasets" / path.name
    if artifact.exists():
        return artifact
    return path if path.exists() else CN_DIR / path.name


SHARDS = {
    "cn7_train": cn_dataset("cn7_corpus_train.jsonl"),
    "cn8_b": cn_dataset("cn8_corpus_b.jsonl"),
    "cn8_atok": cn_dataset("cn8_corpus_atok.jsonl"),
    "cn8_aex": cn_dataset("cn8_corpus_aex.jsonl"),
}


def load_shard_stream(path):
    """Rows' id sequences joined with sentinels; returns stream + row count."""
    ids, rows = [], 0
    for line in path.open():
        if not line.strip():
            continue
        r = json.loads(line)
        ids.extend(r["ids"])
        ids.append(int(SENTINEL))
        rows += 1
    return np.array(ids, dtype=np.uint32), rows


def cmd_build():
    meta = {"tokenizer_sha256": sha256_file(V11_SP), "shards": {}}
    for name, src in SHARDS.items():
        stream, rows = load_shard_stream(src)
        print(f"[build] {name}: {rows:,} rows, {len(stream):,} symbols")
        np.save(index_output(f"shard_{name}_stream.npy"), stream)
        sa = build_suffix_array(stream)
        np.save(index_output(f"shard_{name}_sa.npy"), sa)
        meta["shards"][name] = {
            "source": src.name, "source_sha256": sha256_file(src),
            "rows": rows, "symbols": len(stream),
            "sa_sha256": sha256_file(index_input(f"shard_{name}_sa.npy")),
        }
    with open(HERE / "midtrain_shards_meta.json", "w") as f:
        json.dump(meta, f, indent=2)
    print("[build] midtrain_shards_meta.json written")


class Shards:
    def __init__(self):
        meta_path = HERE / "midtrain_shards_meta.json"
        if not meta_path.exists():
            sys.exit("shards not built — run: midtrain_shards.py build")
        self.meta = json.load(open(meta_path))
        cur = sha256_file(V11_SP)
        if cur != self.meta["tokenizer_sha256"]:
            sys.exit("REFUSING: tokenizer hash mismatch with shard fingerprint")
        for name, info in self.meta["shards"].items():
            if sha256_file(cn_dataset(info["source"])) != info["source_sha256"]:
                sys.exit(f"REFUSING: shard source drifted since build: {name}")
        self.stream = {n: np.load(index_input(f"shard_{n}_stream.npy"))
                        for n in SHARDS}
        self.sa = {n: np.load(index_input(f"shard_{n}_sa.npy")) for n in SHARDS}
        import sentencepiece as spm
        self.sp = spm.SentencePieceProcessor()
        self.sp.load(str(V11_SP))
        # M1 machinery: lexicon per shard from its own texts (deterministic)
        from metrology import build_name_lexicon, norm_m1  # noqa: F401
        self._norm_m1 = norm_m1
        self._lex = {}

    def lexicon(self, name):
        if name not in self._lex:
            from metrology import build_name_lexicon
            texts = (json.loads(l)["text"] for l in
                      cn_dataset(self.meta["shards"][name]["source"]).open()
                      if l.strip())
            self._lex[name] = build_name_lexicon(texts)
        return self._lex[name]

    def m1_prefix_counts(self, name, probe_text):
        """How many shard rows' M1 skeletons start with the probe's M1
        skeleton — the skeleton-familiarity count under the
        authority-holding normalizer."""
        lex = self.lexicon(name)
        probe = self._norm_m1(probe_text, lex)
        n = 0
        for l in cn_dataset(self.meta["shards"][name]["source"]).open():
            if not l.strip():
                continue
            row = self._norm_m1(json.loads(l)["text"], lex)
            if row[:len(probe)] == probe:
                n += 1
        return n

    def _narrow(self, name, lo, hi, offset, token):
        stream, sa, n = self.stream[name], self.sa[name], len(self.stream[name])

        def sym(m):
            p = sa[m] + offset
            return stream[p] if p < n else SENTINEL

        a, b = lo, hi
        while a < b:
            m = (a + b) // 2
            if sym(m) < token:
                a = m + 1
            else:
                b = m
        first = a
        a, b = first, hi
        while a < b:
            m = (a + b) // 2
            if sym(m) <= token:
                a = m + 1
            else:
                b = m
        return first, a

    def longest_match_at(self, name, tokens, i):
        lo, hi = 0, len(self.sa[name])
        length, best = 0, (0, 0, 0)
        while i + length < len(tokens):
            lo, hi = self._narrow(name, lo, hi, length, tokens[i + length])
            if lo == hi:
                break
            length += 1
            best = (length, lo, hi)
        return best

    def surface_profile(self, name, text):
        toks = self.sp.encode(text)
        best_len = best_count = 0
        for i in range(len(toks)):
            l, lo, hi = self.longest_match_at(name, toks, i)
            if l > best_len:
                best_len, best_count = l, hi - lo
        return {"n_tokens": len(toks), "max_match": best_len,
                "max_match_count": int(best_count)}


def cmd_bands():
    sh = Shards()
    bands = json.loads((CN_DIR / "cn8_eval_problems.json").read_text())
    out = {}
    for band, probs in bands.items():
        per_shard = {}
        for name in SHARDS:
            profs = [sh.surface_profile(name, f"{a} + {b} =")
                     for a, b in probs]
            per_shard[name] = {
                "surface_max_mean": round(float(np.mean(
                    [p["max_match"] for p in profs])), 2),
                "surface_max_hist": dict(Counter(
                    p["max_match"] for p in profs)),
            }
        # M1 skeleton: every band prompt normalizes identically ("N + N =")
        m1 = {name: sh.m1_prefix_counts(name, f"{probs[0][0]} + {probs[0][1]} =")
              for name in SHARDS}
        out[band] = {"n": len(probs), "surface": per_shard,
                      "m1_skeleton_prefix_rows": m1}
        print(band, json.dumps(out[band]["surface"], indent=1)[:400])
        print("  m1 prefix rows:", m1)
    with open(HERE / "cn8_band_midtrain_distances.json", "w") as f:
        json.dump({"note": ("frozen CN-8 band prompts scored against the "
                              "midtrain shards, committed before any grading "
                              "verdict; the axis the P-m curve conditions on"),
                    "bands": out}, f, indent=1)
    print("-> cn8_band_midtrain_distances.json")


def cmd_check(text):
    sh = Shards()
    rep = {"text": text}
    for name in SHARDS:
        rep[name] = sh.surface_profile(name, text)
        rep[name]["m1_skeleton_prefix_rows"] = sh.m1_prefix_counts(name, text)
    print(json.dumps(rep, indent=1, ensure_ascii=False))


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("build")
    sub.add_parser("bands")
    c = sub.add_parser("check")
    c.add_argument("--text", required=True)
    args = ap.parse_args()
    if args.cmd == "build":
        cmd_build()
    elif args.cmd == "bands":
        cmd_bands()
    else:
        cmd_check(args.text)
