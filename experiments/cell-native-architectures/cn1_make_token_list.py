#!/usr/bin/env python3
"""CN-1 real build, step 2a (`cell-native-architectures-cn1-preregistration.md`): emit the
natural surface forms of the cell-identity tokens + call-grammar delimiters, one per line, for
`append_user_tokens` (v11-core) to append to `v11.vocab.bin`.

Design (recorded, so the corpus and constrained decoder agree):
  - one atomic token per library cell, surface `<cell:NAME>` — angle-bracket + colon-namespaced
    so it is not word-like and cannot be produced by ordinary text; NAME is the library name,
    verified unique across all 790 cells (see cn1_library.jsonl). Being a single token is what
    makes constrained decoding a one-step mask over a fixed id set (no per-character op-name FSM
    is needed, unlike LARQL's multi-subword names) and what W_f places directly.
  - two delimiter tokens, `<call>` and `</call>`, wrapping the cell token in the corpus:
    `... <call> <cell:NAME> arg ... </call> ...`, space-delimited so each is its own ▁-prefixed
    chunk and encodes to exactly one id.

Emits ALL 790 cells (axis-A held-out cells included — held-out means never *called* in
training, not absent from the vocabulary; constrained decoding must be able to emit them, which
is the whole point of gate (ii)).

Run: python3 cn1_make_token_list.py
"""
from __future__ import annotations

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
LIBRARY = HERE / "cn1_library.jsonl"
OUT = HERE / "cn1_cell_tokens.txt"

DELIMITERS = ["<call>", "</call>"]


def main() -> None:
    rows = [json.loads(line) for line in LIBRARY.read_text().splitlines() if line.strip()]
    names = [r["name"] for r in rows]
    assert len(names) == len(set(names)), "cell names must be unique to be atomic tokens"

    tokens = list(DELIMITERS) + [f"<cell:{n}>" for n in names]
    OUT.write_text("\n".join(tokens) + "\n")
    print(f"wrote {len(tokens)} tokens ({len(DELIMITERS)} delimiters + {len(names)} cells) -> {OUT}")


if __name__ == "__main__":
    main()
