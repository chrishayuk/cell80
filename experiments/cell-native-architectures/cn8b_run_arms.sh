#!/bin/sh
# CN-8b chain: wait for the CN-8 eval chain to release MPS, then train B' s80/s81 + A-ex' s80
# (cn8_train.py reused verbatim per prereg §4), then run the CN-8b band evals.
cd "$(dirname "$0")" || exit 1

until grep -q "ALL EVALS DONE" cn8_eval_run.log 2>/dev/null; do sleep 60; done

run() {
  tag="$1"; corpus="$2"; seed="$3"; tokens="$4"
  echo "=== launching $tag ($(date)) ==="
  python3 cn8_train.py --corpus "$corpus" --tag "$tag" --seed "$seed" --tokens "$tokens" \
    > "cn8b_train_$tag.log" 2>&1 || echo "!!! FAILED: $tag"
  tail -2 "cn8b_train_$tag.log"
}

run bp_s80   cn8b_corpus_b.jsonl   80 4981759
run bp_s81   cn8b_corpus_b.jsonl   81 4981759
run aexp_s80 cn8b_corpus_aex.jsonl 80 6000000

echo "=== training done, evals ($(date)) ==="
python3 cn8b_eval.py --ckpt cn8_ckpt_bp_s80.pt --format trace
python3 cn8b_eval.py --ckpt cn8_ckpt_bp_s81.pt --format trace
python3 cn8b_eval.py --ckpt cn8_ckpt_aexp_s80.pt --format answer
echo "ALL CN8B DONE ($(date))"
