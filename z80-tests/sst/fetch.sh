#!/usr/bin/env bash
# Fetch SingleStepTests z80 opcode vectors into z80-tests/sst/v1/ (git-ignored).
#
#   ./fetch.sh          # a representative subset covering every instruction class (~70 files)
#   ./fetch.sh --all    # the ENTIRE instruction set (~1.8k files, ~1.5 GB) — slow
#
# Then run the correctness harness:
#   cargo test -p z80-tests --release single_step -- --nocapture
#
# Source: https://github.com/SingleStepTests/z80 (v1). Files are named by opcode bytes,
# e.g. "00.json", "cb 40.json", "ed 44.json", "dd cb __ 06.json" — spaces are URL-encoded.
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/v1"
BASE="https://raw.githubusercontent.com/SingleStepTests/z80/main/v1"
API="https://api.github.com/repos/SingleStepTests/z80/contents/v1?ref=main"
mkdir -p "$DIR"

urlenc() { printf '%s' "$1" | sed 's/ /%20/g'; }

fetch_one() {
  local name="$1" out
  out="$DIR/$name"
  [ -f "$out" ] && return 0
  if curl -sfL "$BASE/$(urlenc "$name")" -o "$out"; then
    echo "  $name"
  else
    echo "  FAILED $name" >&2
    rm -f "$out"
  fi
}

if [ "${1:-}" = "--all" ]; then
  echo "Fetching the full SingleStepTests set into $DIR (large)…"
  page=1
  while :; do
    names=$(curl -sfL "$API&per_page=100&page=$page" | grep '"name"' | sed -E 's/.*"name": *"([^"]+)".*/\1/')
    [ -z "$names" ] && break
    while IFS= read -r n; do fetch_one "$n"; done <<< "$names"
    page=$((page + 1))
  done
else
  echo "Fetching a representative subset into $DIR …"
  # One or more per instruction class: arithmetic/logic, loads, control flow, DAA & the
  # SCF/CCF X/Y quirk, block ops, rotates/bit (CB), IX/IY (DD/FD), DDCB/FDCB, IN/OUT,
  # and the undocumented NEG/RRD/RLD/LD A,I.
  SUBSET=(
    "00.json" "01.json" "04.json" "07.json" "09.json" "0a.json" "18.json" "20.json"
    "27.json" "2f.json" "37.json" "3f.json" "40.json" "46.json" "70.json" "76.json"
    "80.json" "88.json" "90.json" "98.json" "a0.json" "a8.json" "b0.json" "b8.json"
    "c1.json" "c3.json" "c5.json" "c9.json" "cd.json" "d3.json" "db.json" "e3.json"
    "e9.json" "f3.json" "fb.json" "fe.json"
    "cb 00.json" "cb 06.json" "cb 16.json" "cb 3e.json" "cb 40.json" "cb 46.json"
    "cb 80.json" "cb c0.json"
    "ed 42.json" "ed 44.json" "ed 4d.json" "ed 57.json" "ed 5f.json" "ed 67.json"
    "ed 6f.json" "ed 78.json" "ed 79.json"
    # All block ops — these exposed the EI/IFF, LDIR/CPIR and INIR/OTIR flag bugs.
    "ed a0.json" "ed b0.json" "ed a8.json" "ed b8.json"
    "ed a1.json" "ed b1.json" "ed a9.json" "ed b9.json"
    "ed a2.json" "ed b2.json" "ed aa.json" "ed ba.json"
    "ed a3.json" "ed b3.json" "ed ab.json" "ed bb.json"
    "dd 09.json" "dd 21.json" "dd 23.json" "dd 34.json" "dd 46.json" "dd 70.json"
    "dd 7e.json" "dd e1.json" "dd e5.json" "dd e9.json"
    "fd 21.json" "fd 34.json" "fd 7e.json"
    "dd cb __ 06.json" "dd cb __ 46.json" "dd cb __ c6.json" "fd cb __ 06.json"
  )
  for n in "${SUBSET[@]}"; do fetch_one "$n"; done
fi

echo "Done. $(find "$DIR" -name '*.json' | wc -l | tr -d ' ') opcode file(s) in $DIR"
