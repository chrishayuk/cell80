#!/usr/bin/env python3
"""corpus-atlas surface index: suffix array over the materialized v11 stream.

Level-1 of the atlas (spec: experiments/corpus-atlas-DRAFT.md §2). Answers,
for any probe text: has this token sequence been seen in training, how many
times, and what is the longest match at every probe position — with
receipts (stream positions decoded back to the training text around them).

Design notes, load-bearing:

- Suffix ARRAY (numpy prefix-doubling), not a suffix automaton: a pure-
  Python automaton at 24M tokens is a memory blowup; the array builds in
  minutes and queries in O(L log N) per position.
- "Seen" means WITHIN A TRAINING CHUNK. The stream was consumed as
  independent 256-token rows — the model never attended across chunk
  boundaries — so a sentinel (>= 2^31, never a real id, never in a probe)
  is inserted between chunks and matches cannot straddle it. Text that
  flows across a boundary in the source story was still never *seen* as a
  sequence.
- Tokenizer identity is enforced structurally (spec §1): the index
  fingerprint stores the tokenizer sha256 and the query path refuses to
  score if the tokenizer file's current hash differs.

USAGE RULE: score probe surfaces from their natural sequence start (full
prompts — which DIV probes are anyway). SP decode->encode is not identity
for slices starting mid-sequence, so a text-entry profile beginning
mid-string undercounts match lengths (smoke1 failed on exactly this before
switching to raw ids). Verbatim-training-text checks must query by raw
token ids, never decode->re-encode.

Build:  python3 atlas_surface.py build            (~minutes, CPU only)
Query:  python3 atlas_surface.py query --text "Once upon a time"
Smoke:  python3 atlas_surface.py smoke
"""

import argparse
import hashlib
import json
import sys
from pathlib import Path

import numpy as np
from artifact_paths import index_input, index_output

HERE = Path(__file__).resolve().parent
V11_SP = (Path.home() / "chris-source" / "chris-experiments" / "compilation"
          / "15_v11_model" / "v11_tokenizer" / "v11.model")
CHUNK = 256
SENTINEL = np.uint32(0x80000000)  # > any SP id; probes can never contain it
PHASES = {1: "v11_stream_phase1_run1.u32", 3: "v11_stream_phase3_run1.u32"}


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def load_stream_with_sentinels(phase):
    ids = np.fromfile(index_input(PHASES[phase]), dtype=np.uint32)
    assert len(ids) % CHUNK == 0
    rows = ids.reshape(-1, CHUNK)
    with_sent = np.full((rows.shape[0], CHUNK + 1), SENTINEL, dtype=np.uint32)
    with_sent[:, :CHUNK] = rows
    return with_sent.ravel()


def build_suffix_array(text):
    """Prefix-doubling over uint32 symbols; returns int32 suffix array."""
    n = len(text)
    rank = text.astype(np.int64)
    sa = None
    k = 1
    tmp = np.empty(n, dtype=np.int64)
    while True:
        # sort by (rank[i], rank[i+k]) — out-of-range treated as -1
        second = np.full(n, -1, dtype=np.int64)
        second[: n - k] = rank[k:]
        sa = np.lexsort((second, rank))
        # re-rank
        tmp[sa[0]] = 0
        prev = sa[:-1]
        cur = sa[1:]
        newgroup = (rank[cur] != rank[prev]) | (second[cur] != second[prev])
        tmp[cur] = np.cumsum(newgroup)
        rank, tmp = tmp.copy(), rank
        if rank[sa[-1]] == n - 1:
            break
        k *= 2
    return sa.astype(np.int32)


class SurfaceIndex:
    def __init__(self):
        meta_path = HERE / "surface_index_meta.json"
        if not meta_path.exists():
            sys.exit("surface index not built — run: atlas_surface.py build")
        self.meta = json.load(open(meta_path))
        cur = sha256_file(V11_SP)
        if cur != self.meta["tokenizer_sha256"]:
            sys.exit(f"REFUSING to score: tokenizer hash {cur[:12]} != "
                     f"index fingerprint {self.meta['tokenizer_sha256'][:12]}")
        import sentencepiece as spm
        self.sp = spm.SentencePieceProcessor()
        self.sp.load(str(V11_SP))
        self.text = {p: load_stream_with_sentinels(p) for p in PHASES}
        self.sa = {p: np.load(index_input(f"surface_sa_phase{p}.npy")) for p in PHASES}

    def _narrow(self, phase, lo, hi, offset, token):
        """Narrow SA range [lo,hi) to suffixes whose `offset`-th symbol == token."""
        text, sa, n = self.text[phase], self.sa[phase], len(self.text[phase])

        def sym(m):
            p = sa[m] + offset
            return text[p] if p < n else SENTINEL  # past-end never matches

        a, b = lo, hi
        while a < b:  # first index with sym >= token
            m = (a + b) // 2
            if sym(m) < token:
                a = m + 1
            else:
                b = m
        first = a
        a, b = first, hi
        while a < b:  # first index with sym > token
            m = (a + b) // 2
            if sym(m) <= token:
                a = m + 1
            else:
                b = m
        return first, a

    def longest_match_at(self, phase, tokens, i):
        """Longest match for tokens[i:] and its occurrence range."""
        lo, hi = 0, len(self.sa[phase])
        length = 0
        best = (0, 0, 0)
        while i + length < len(tokens):
            lo, hi = self._narrow(phase, lo, hi, length, tokens[i + length])
            if lo == hi:
                break
            length += 1
            best = (length, lo, hi)
        return best

    def receipts(self, phase, lo, hi, length, k=3, ctx=12):
        out = []
        for m in range(lo, min(lo + k, hi)):
            pos = int(self.sa[phase][m])
            chunk_idx, in_chunk = divmod(pos, CHUNK + 1)
            row = self.text[phase][chunk_idx * (CHUNK + 1):
                                    (chunk_idx + 1) * (CHUNK + 1)][:CHUNK]
            a = max(0, in_chunk - ctx)
            b = min(CHUNK, in_chunk + length + ctx)
            out.append({
                "phase": phase, "chunk": chunk_idx, "offset": in_chunk,
                "context": self.sp.decode([int(t) for t in row[a:b]]),
            })
        return out

    def profile(self, text_str, receipts_for_max=True):
        tokens = self.sp.encode(text_str)
        per_pos = []
        for i in range(len(tokens)):
            entry = {"pos": i, "piece": self.sp.IdToPiece(tokens[i])}
            for phase in PHASES:
                length, lo, hi = self.longest_match_at(phase, tokens, i)
                entry[f"p{phase}_len"] = length
                entry[f"p{phase}_count"] = int(hi - lo)
            per_pos.append(entry)
        result = {"text": text_str, "n_tokens": len(tokens),
                  "per_position": per_pos,
                  "max_match": {p: max((e[f"p{p}_len"] for e in per_pos),
                                        default=0) for p in PHASES}}
        if receipts_for_max and per_pos:
            p = max(PHASES, key=lambda p: result["max_match"][p])
            i = max(range(len(per_pos)), key=lambda i: per_pos[i][f"p{p}_len"])
            length, lo, hi = self.longest_match_at(p, tokens, i)
            if length:
                result["max_match_receipts"] = {
                    "phase": p, "probe_pos": i, "length": length,
                    "count": int(hi - lo),
                    "matched_text": self.sp.decode(tokens[i:i + length]),
                    "rows": self.receipts(p, lo, hi, length),
                }
        return result


def cmd_build():
    meta = {"tokenizer_sha256": sha256_file(V11_SP), "chunk": CHUNK,
            "sentinel": int(SENTINEL), "phases": {}}
    for phase, fname in PHASES.items():
        print(f"[build] phase {phase}: loading {fname}")
        text = load_stream_with_sentinels(phase)
        print(f"[build] phase {phase}: suffix array over {len(text):,} symbols")
        sa = build_suffix_array(text)
        out = index_output(f"surface_sa_phase{phase}.npy")
        np.save(out, sa)
        meta["phases"][str(phase)] = {
            "stream_file": fname, "stream_sha256": sha256_file(index_input(fname)),
            "symbols": len(text), "sa_file": out.name,
            "sa_sha256": sha256_file(out),
        }
        print(f"[build] phase {phase}: saved {out.name}")
    with open(HERE / "surface_index_meta.json", "w") as f:
        json.dump(meta, f, indent=2)
    print("[build] surface_index_meta.json written")


def cmd_query(text):
    idx = SurfaceIndex()
    print(json.dumps(idx.profile(text), indent=1))


def cmd_smoke():
    idx = SurfaceIndex()
    import sentencepiece  # noqa: F401  (already loaded via idx)

    # 1. verbatim training tokens must match full-length, in-chunk.
    # NB: queried by RAW IDS — SP decode->encode is not identity for a slice
    # starting mid-sequence, so text-round-tripping is the wrong test (and a
    # real caveat for text-entry probes: mid-string starts undercount).
    tokens = [int(t) for t in idx.text[1][40:72]]
    length, lo, hi = idx.longest_match_at(1, tokens, 0)
    print(f"smoke1 verbatim in-chunk: match {length}/{len(tokens)} "
          f"count {hi - lo}  {'PASS' if length == len(tokens) else 'FAIL'}")

    # 2. straddling probe must be capped at the boundary
    strad = np.concatenate([idx.text[1][CHUNK - 16:CHUNK],
                             idx.text[1][CHUNK + 1:CHUNK + 17]])
    assert SENTINEL not in strad
    sl, *_ = idx.longest_match_at(1, [int(t) for t in strad], 0)
    print(f"smoke2 boundary straddle: match {sl}/32 "
          f"{'PASS' if sl < 32 else 'FAIL (crossed a chunk boundary!)'}")

    # 3. novel text gets a short-match profile with receipts
    r = idx.profile("What is 25 multiplied by 32?")
    mm = r["max_match"]
    rec = r.get("max_match_receipts", {})
    print(f"smoke3 novel probe: max match p1={mm[1]} p3={mm[3]}; "
          f"longest seen: '{rec.get('matched_text', '')}' "
          f"(count {rec.get('count')})")
    for row in rec.get("rows", [])[:2]:
        print(f"        receipt: …{row['context']}…")


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("build")
    q = sub.add_parser("query")
    q.add_argument("--text", required=True)
    sub.add_parser("smoke")
    args = ap.parse_args()
    if args.cmd == "build":
        cmd_build()
    elif args.cmd == "query":
        cmd_query(args.text)
    else:
        cmd_smoke()
