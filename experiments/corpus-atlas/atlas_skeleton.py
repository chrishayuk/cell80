#!/usr/bin/env python3
"""corpus-atlas skeleton index: normalized (frame-level) suffix array + the
surface<->skeleton alignment map (spec §3).

Run under the pinned venv: .venv-skeleton/bin/python atlas_skeleton.py …
(system spaCy is binary-broken against current numpy, and the system python
is owned by live training chains — do not touch it).

SKELETON-v1 (versioned choice, spec §3 — every wall claim states this
version):
  - spaCy(en_core_web_sm; tok2vec+tagger+attribute_ruler+ner) tokens.
  - any token containing a digit          -> D  (symbol 0)
  - PROPN or PERSON/GPE/LOC/ORG/FAC ents  -> N  (symbol 1), CONTIGUOUS
    runs collapsed to one N (a multi-word name is one referent)
  - everything else -> lowercased token text, word-level vocab id (>= 2)

SYMBOL SPACE — disclosed spec amendment: the skeleton index is built over
WORD-LEVEL vocabulary ids, not an SP re-encode of normalized text as the
draft first said. Reason: any textual D/N marker either collides with real
corpus text or encodes to unk through the pinned SP model (collapsing the
D/N distinction). Word-level ids are also the natural space for frames
("N gave N D apples" = 5 symbols) and for the catalogue harvest.

ALIGNMENT MAP — the part where silent bugs live (spec §3 gate): skeleton
position -> (chunk, piece_start, piece_end) into the ORIGINAL surface
stream. Char offsets come from reconstructing each chunk's text directly
from its pieces ('▁'->space, control ids zero-width, byte-fallback runs
decoded together as UTF-8 with the run's span shared) — never from
decode->re-encode, which is not identity mid-sequence. Receipts for
skeleton matches decode the ORIGINAL pieces, so a skeleton-familiar claim
always ships the real training text it matched. The map is gated on its
own straddle/verbatim-class smoke before any two-distance number is
believed.

"Seen" remains within-a-training-chunk (sentinels between chunks), same
as the surface index. Chunk boundaries cut stories mid-sentence; NER
quality at chunk edges inherits that — a within-chunk-semantics cost,
not a bug.

Build:  .venv-skeleton/bin/python atlas_skeleton.py build   (~20-40 min)
Smoke:  .venv-skeleton/bin/python atlas_skeleton.py smoke
Score:  .venv-skeleton/bin/python atlas_skeleton.py score --text "..."
"""

import argparse
import json
import sys
from array import array
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from atlas_surface import (CHUNK, PHASES, SENTINEL, V11_SP,  # noqa: E402
                            SurfaceIndex, build_suffix_array, sha256_file)

SKELETON_VERSION = "v1"
D_SYM, N_SYM = 0, 1
FIRST_WORD_SYM = 2
OOV = np.uint32(0xFFFFFFFF)  # probe-time only; never present in the stream
ENT_N = {"PERSON", "GPE", "LOC", "ORG", "FAC"}
NO_SPAN = 0xFFFF


def load_nlp():
    import spacy
    return spacy.load("en_core_web_sm",
                       exclude=["parser", "lemmatizer", "senter"])


def chunk_to_text_spans(sp, ids):
    """Reconstruct a chunk's text + per-piece char spans from ORIGINAL ids."""
    parts, spans, cur, i, n = [], [], 0, 0, len(ids)
    while i < n:
        pid = int(ids[i])
        if sp.IsControl(pid):
            spans.append((cur, cur))
            i += 1
            continue
        if sp.IsByte(pid):
            j, buf = i, bytearray()
            while j < n and sp.IsByte(int(ids[j])):
                buf.append(int(sp.IdToPiece(int(ids[j]))[1:-1], 16))
                j += 1
            s = buf.decode("utf-8", errors="replace")
            parts.append(s)
            for _ in range(i, j):  # run-level span, shared by the run
                spans.append((cur, cur + len(s)))
            cur += len(s)
            i = j
            continue
        piece = sp.IdToPiece(pid).replace("▁", " ")
        parts.append(piece)
        spans.append((cur, cur + len(piece)))
        cur += len(piece)
        i += 1
    return "".join(parts), spans


def skeletonize(doc, vocab, grow):
    """spaCy doc -> (symbols, char spans). vocab grows only at build time."""
    syms, spans = [], []
    prev_n = False
    for tok in doc:
        if tok.is_space:
            prev_n = False
            continue
        if any(c.isdigit() for c in tok.text):
            syms.append(D_SYM)
            spans.append((tok.idx, tok.idx + len(tok.text)))
            prev_n = False
        elif tok.pos_ == "PROPN" or tok.ent_type_ in ENT_N:
            if prev_n:  # extend the previous N over a contiguous name
                spans[-1] = (spans[-1][0], tok.idx + len(tok.text))
            else:
                syms.append(N_SYM)
                spans.append((tok.idx, tok.idx + len(tok.text)))
            prev_n = True
        else:
            w = tok.text.lower()
            sid = vocab.get(w)
            if sid is None:
                if grow:
                    sid = FIRST_WORD_SYM + len(vocab)
                    vocab[w] = sid
                else:
                    sid = int(OOV)
            syms.append(sid)
            spans.append((tok.idx, tok.idx + len(tok.text)))
            prev_n = False
    return syms, spans


def word_span_to_pieces(piece_spans, a, b):
    """Pieces overlapping char range [a,b) -> (p0, p1) piece indices."""
    starts = [s for s, _ in piece_spans]
    ends = [e for _, e in piece_spans]
    import bisect
    p0 = bisect.bisect_right(ends, a)
    p1 = bisect.bisect_left(starts, b)
    if p1 <= p0:
        p1 = p0 + 1
    return p0, min(p1, len(piece_spans))


def cmd_build(n_process):
    import sentencepiece as spm
    import spacy
    sp = spm.SentencePieceProcessor()
    sp.load(str(V11_SP))
    nlp = load_nlp()

    vocab = {}
    for phase, fname in PHASES.items():
        ids = np.fromfile(HERE / fname, dtype=np.uint32).reshape(-1, CHUNK)
        print(f"[build] phase {phase}: {len(ids):,} chunks -> text")
        recon = [chunk_to_text_spans(sp, row) for row in ids]
        texts = [t for t, _ in recon]

        skel = array("I")
        chunk_of = array("I")
        pspan = array("H")  # interleaved (p0, p1); NO_SPAN for sentinels
        print(f"[build] phase {phase}: spaCy over {sum(map(len, texts)) / 1e6:.0f}M chars "
              f"(n_process={n_process})")
        for ci, doc in enumerate(nlp.pipe(texts, batch_size=64,
                                            n_process=n_process)):
            syms, spans = skeletonize(doc, vocab, grow=True)
            piece_spans = recon[ci][1]
            for s, (a, b) in zip(syms, spans):
                p0, p1 = word_span_to_pieces(piece_spans, a, b)
                skel.append(s)
                chunk_of.append(ci)
                pspan.extend((p0, p1))
            skel.append(int(SENTINEL))
            chunk_of.append(ci)
            pspan.extend((NO_SPAN, NO_SPAN))
            if (ci + 1) % 10000 == 0:
                print(f"    {ci + 1:,} chunks, {len(skel):,} symbols, "
                      f"vocab {len(vocab):,}", flush=True)

        stream = np.frombuffer(skel, dtype=np.uint32)
        np.save(HERE / f"skeleton_stream_phase{phase}.npy", stream)
        np.save(HERE / f"skeleton_chunk_phase{phase}.npy",
                np.frombuffer(chunk_of, dtype=np.uint32))
        np.save(HERE / f"skeleton_pspan_phase{phase}.npy",
                np.frombuffer(pspan, dtype=np.uint16).reshape(-1, 2))
        print(f"[build] phase {phase}: suffix array over {len(stream):,} symbols")
        sa = build_suffix_array(stream)
        np.save(HERE / f"skeleton_sa_phase{phase}.npy", sa)

    with open(HERE / "skeleton_vocab.json", "w") as f:
        json.dump(vocab, f)
    meta = {"skeleton_version": SKELETON_VERSION,
            "tokenizer_sha256": sha256_file(V11_SP),
            "spacy": spacy.__version__,
            "spacy_model": nlp.meta["name"] + "-" + nlp.meta["version"],
            "vocab_size": len(vocab),
            "symbols": {"D": D_SYM, "N": N_SYM, "first_word": FIRST_WORD_SYM},
            "ent_n": sorted(ENT_N),
            "phases": {str(p): {
                "stream_sha256": sha256_file(HERE / f"skeleton_stream_phase{p}.npy"),
                "sa_sha256": sha256_file(HERE / f"skeleton_sa_phase{p}.npy"),
            } for p in PHASES}}
    with open(HERE / "skeleton_index_meta.json", "w") as f:
        json.dump(meta, f, indent=2)
    print(f"[build] done: vocab {len(vocab):,}; skeleton_index_meta.json written")


class SkeletonIndex:
    def __init__(self, with_nlp=True):
        meta_path = HERE / "skeleton_index_meta.json"
        if not meta_path.exists():
            sys.exit("skeleton index not built — run: atlas_skeleton.py build")
        self.meta = json.load(open(meta_path))
        cur = sha256_file(V11_SP)
        if cur != self.meta["tokenizer_sha256"]:
            sys.exit("REFUSING to score: tokenizer hash mismatch with index")
        self.vocab = json.load(open(HERE / "skeleton_vocab.json"))
        self.inv = {v: k for k, v in self.vocab.items()}
        self.inv[D_SYM], self.inv[N_SYM] = "D", "N"
        self.stream, self.sa, self.chunk_of, self.pspan = {}, {}, {}, {}
        for p in PHASES:
            self.stream[p] = np.load(HERE / f"skeleton_stream_phase{p}.npy")
            self.sa[p] = np.load(HERE / f"skeleton_sa_phase{p}.npy")
            self.chunk_of[p] = np.load(HERE / f"skeleton_chunk_phase{p}.npy")
            self.pspan[p] = np.load(HERE / f"skeleton_pspan_phase{p}.npy")
        import sentencepiece as spm
        self.sp = spm.SentencePieceProcessor()
        self.sp.load(str(V11_SP))
        self.nlp = load_nlp() if with_nlp else None

    def encode_probe(self, text):
        syms, spans = skeletonize(self.nlp(text), self.vocab, grow=False)
        return syms, spans

    def _narrow(self, phase, lo, hi, offset, sym):
        stream, sa, n = self.stream[phase], self.sa[phase], len(self.stream[phase])
        if sym == int(OOV):
            return 0, 0

        def at(m):
            p = sa[m] + offset
            return stream[p] if p < n else SENTINEL

        a, b = lo, hi
        while a < b:
            m = (a + b) // 2
            if at(m) < sym:
                a = m + 1
            else:
                b = m
        first = a
        a, b = first, hi
        while a < b:
            m = (a + b) // 2
            if at(m) <= sym:
                a = m + 1
            else:
                b = m
        return first, a

    def longest_match_at(self, phase, syms, i):
        lo, hi = 0, len(self.sa[phase])
        length, best = 0, (0, 0, 0)
        while i + length < len(syms):
            lo, hi = self._narrow(phase, lo, hi, length, syms[i + length])
            if lo == hi:
                break
            length += 1
            best = (length, lo, hi)
        return best

    def receipt(self, phase, sa_idx, length, ctx_pieces=10):
        """Map a skeleton match back to ORIGINAL surface text."""
        pos = int(self.sa[phase][sa_idx])
        ci = int(self.chunk_of[phase][pos])
        p0 = int(self.pspan[phase][pos][0])
        p1 = int(self.pspan[phase][pos + length - 1][1])
        ids = np.fromfile(HERE / PHASES[phase], dtype=np.uint32
                           ).reshape(-1, CHUNK)[ci]
        a, b = max(0, p0 - ctx_pieces), min(CHUNK, p1 + ctx_pieces)
        return {"phase": phase, "chunk": ci, "pieces": [p0, p1],
                "matched_original": self.sp.decode(
                    [int(t) for t in ids[p0:p1]]),
                "context": self.sp.decode([int(t) for t in ids[a:b]])}

    def profile(self, text):
        syms, spans = self.encode_probe(text)
        per_pos = []
        for i, s in enumerate(syms):
            entry = {"pos": i, "symbol": self.inv.get(s, "<OOV>"),
                      "surface_text": text[spans[i][0]:spans[i][1]]}
            for p in PHASES:
                length, lo, hi = self.longest_match_at(p, syms, i)
                entry[f"p{p}_len"] = length
                entry[f"p{p}_count"] = int(hi - lo)
            per_pos.append(entry)
        out = {"text": text, "skeleton_version": SKELETON_VERSION,
                "skeleton": " ".join(self.inv.get(s, "<OOV>") for s in syms),
                "per_position": per_pos,
                "max_match": {p: max((e[f"p{p}_len"] for e in per_pos),
                                      default=0) for p in PHASES}}
        if per_pos:
            p = max(PHASES, key=lambda p: out["max_match"][p])
            i = max(range(len(per_pos)), key=lambda i: per_pos[i][f"p{p}_len"])
            length, lo, hi = self.longest_match_at(p, syms, i)
            if length:
                out["max_match_receipt"] = dict(
                    self.receipt(p, lo, length), count=int(hi - lo),
                    matched_skeleton=" ".join(
                        self.inv.get(s, "<OOV>") for s in syms[i:i + length]))
        return out


def cmd_score(text):
    """The two-distance scorer (spec §5): surface + skeleton, receipts on both."""
    surf = SurfaceIndex().profile(text)
    skel = SkeletonIndex().profile(text)
    print(json.dumps({
        "text": text,
        "surface": {"max_match": surf["max_match"],
                     "receipt": surf.get("max_match_receipts")},
        "skeleton": {"version": SKELETON_VERSION,
                      "rendering": skel["skeleton"],
                      "max_match": skel["max_match"],
                      "receipt": skel.get("max_match_receipt")},
    }, indent=1, ensure_ascii=False))


def cmd_smoke():
    idx = SkeletonIndex()
    p = 1
    stream, chunk_of = idx.stream[p], idx.chunk_of[p]

    # A. verbatim: a training chunk's own skeleton slice must self-match and
    # its receipt must decode to that chunk's original text.
    pos = np.where((chunk_of == 1000) & (stream != SENTINEL))[0]
    probe = [int(s) for s in stream[pos[10:30]]]
    length, lo, hi = idx.longest_match_at(p, probe, 0)
    rec = idx.receipt(p, lo, length) if length else {}
    ids = np.fromfile(HERE / PHASES[p], dtype=np.uint32).reshape(-1, CHUNK)[1000]
    own = idx.sp.decode([int(t) for t in ids])
    ok_a = length == len(probe) and hi - lo >= 1 and (
        rec.get("chunk") == 1000 or rec.get("matched_original", "#") in own)
    print(f"smokeA verbatim+receipt: match {length}/{len(probe)} count {hi - lo} "
          f"chunk {rec.get('chunk')}  {'PASS' if ok_a else 'FAIL'}")

    # B. straddle: skeleton matches must not cross the chunk sentinel.
    sent = np.where(stream == SENTINEL)[0][500]
    strad = [int(s) for s in np.concatenate([stream[sent - 8:sent],
                                               stream[sent + 1:sent + 9]])]
    sl, *_ = idx.longest_match_at(p, strad, 0)
    print(f"smokeB straddle: match {sl}/16 "
          f"{'PASS' if sl < 16 else 'FAIL (crossed sentinel!)'}")

    # C. alignment round-trip on random positions: the aligned piece span's
    # decoded text must contain the skeleton symbol's surface form.
    rng = np.random.default_rng(90)
    cand = np.where((stream >= FIRST_WORD_SYM) & (stream != SENTINEL))[0]
    sample = rng.choice(cand, size=500, replace=False)
    all_ids = np.fromfile(HERE / PHASES[p], dtype=np.uint32).reshape(-1, CHUNK)
    good = 0
    for s in sample:
        w = idx.inv[int(stream[s])]
        ci = int(chunk_of[s])
        p0, p1 = (int(x) for x in idx.pspan[p][s])
        dec = idx.sp.decode([int(t) for t in all_ids[ci][p0:p1]]).lower()
        good += w in dec or w in idx.sp.decode(
            [int(t) for t in all_ids[ci][max(0, p0 - 1):p1 + 1]]).lower()
    frac = good / len(sample)
    print(f"smokeC alignment round-trip: {frac:.1%} of 500 "
          f"{'PASS' if frac >= 0.99 else 'FAIL'}")

    # D. the two-distance demo: novel surface, familiar frame.
    demo = "One day, a little girl named Zorblax found 7 shiny pebbles."
    surf = SurfaceIndex().profile(demo)
    skel = idx.profile(demo)
    r = skel.get("max_match_receipt", {})
    print(f"smokeD two-distance: surface max {surf['max_match']} vs "
          f"skeleton max {skel['max_match']}")
    print(f"        skeleton: {skel['skeleton']}")
    print(f"        longest frame: '{r.get('matched_skeleton')}' "
          f"(count {r.get('count')})")
    print(f"        receipt: …{r.get('context')}…")


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    b = sub.add_parser("build")
    b.add_argument("--n-process", type=int, default=4)
    q = sub.add_parser("score")
    q.add_argument("--text", required=True)
    sub.add_parser("smoke")
    args = ap.parse_args()
    if args.cmd == "build":
        cmd_build(args.n_process)
    elif args.cmd == "score":
        cmd_score(args.text)
    else:
        cmd_smoke()
