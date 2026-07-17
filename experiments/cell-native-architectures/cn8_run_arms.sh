#!/bin/sh
# CN-8 training chain (prereg §3.4/§3.5): five arms, sequential on MPS.
cd "$(dirname "$0")" || exit 1
artifact_root="${CELL80_CELL_NATIVE_ARTIFACT_ROOT:-${CELL80_ARTIFACT_ROOT:-artifacts}}"
log_dir="$artifact_root/logs"
mkdir -p "$log_dir"

run() {
  tag="$1"; corpus="$2"; seed="$3"; tokens="$4"
  echo "=== launching $tag ($(date)) ==="
  python3 cn8_train.py --corpus "$corpus" --tag "$tag" --seed "$seed" --tokens "$tokens" \
    > "$log_dir/cn8_train_$tag.log" 2>&1 || echo "!!! FAILED: $tag"
  tail -2 "$log_dir/cn8_train_$tag.log"
}

run b_s80    cn8_corpus_b.jsonl    80 6000000
run b_s81    cn8_corpus_b.jsonl    81 6000000
run atok_s80 cn8_corpus_atok.jsonl 80 6000000
run atok_s81 cn8_corpus_atok.jsonl 81 6000000
run aex_s80  cn8_corpus_aex.jsonl  80 882500
echo "=== chain complete ($(date)) ==="
