#!/usr/bin/env bash
# Fetch the ZEXDOC / ZEXALL Z80 instruction-exerciser ROMs into z80-tests/zex/ (git-ignored).
#
#   ./fetch.sh          # ZEXDOC (documented flags) — the usual acceptance run
#   ./fetch.sh --all    # ZEXDOC + ZEXALL (ZEXALL also checks the undocumented flags)
#
# Then run the (otherwise-ignored, slow) acceptance test — use --release:
#   cargo test -p z80-tests --release --lib zex -- --ignored --nocapture
#
# ZEXALL is billions of T-states; expect it to take a while even in release.
# Source: https://github.com/anotherlin/z80emu (testfiles).
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE="https://github.com/anotherlin/z80emu/raw/master/testfiles"

get() { curl -sfL "$BASE/$1" -o "$DIR/$1" && echo "  $1 ($(wc -c <"$DIR/$1" | tr -d ' ') bytes)"; }

get zexdoc.com
[ "${1:-}" = "--all" ] && get zexall.com
echo "Done. ROMs in $DIR"
