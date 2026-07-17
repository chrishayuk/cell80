#!/bin/sh
# CN-8/CN-8b master: everything still owed, sequential, single MPS user. Run detached
# (new session via double-fork) because harness-tracked background tasks keep getting
# reaped mid-run. Band lines append to the SAME logs the session monitors tail.
cd "$(dirname "$0")" || exit 1

{
  python3 cn8_eval.py --raw --format answer
  python3 cn8_eval.py --ckpt cn8_ckpt_b_s81.pt --format trace
  python3 cn8_eval.py --raw --format trace
  echo "ALL EVALS DONE"
} >> cn8_eval_run.log 2>&1

{
  for spec in "bp_s80 cn8b_corpus_b.jsonl 80 4981759" \
              "bp_s81 cn8b_corpus_b.jsonl 81 4981759" \
              "aexp_s80 cn8b_corpus_aex.jsonl 80 6000000"; do
    tag=$(echo "$spec" | cut -d' ' -f1)
    corpus=$(echo "$spec" | cut -d' ' -f2)
    seed=$(echo "$spec" | cut -d' ' -f3)
    tokens=$(echo "$spec" | cut -d' ' -f4)
    if [ -f "cn8_ckpt_$tag.pt" ]; then
      echo "=== $tag already trained, skipping ($(date)) ==="
    else
      echo "=== launching $tag ($(date)) ==="
      python3 cn8_train.py --corpus "$corpus" --tag "$tag" --seed "$seed" --tokens "$tokens" \
        > "cn8b_train_$tag.log" 2>&1 || echo "!!! FAILED: $tag"
      tail -2 "cn8b_train_$tag.log"
    fi
  done
  echo "=== training done, evals ($(date)) ==="
  python3 cn8b_eval.py --ckpt cn8_ckpt_bp_s80.pt --format trace
  python3 cn8b_eval.py --ckpt cn8_ckpt_bp_s81.pt --format trace
  python3 cn8b_eval.py --ckpt cn8_ckpt_aexp_s80.pt --format answer
  echo "ALL CN8B DONE ($(date))"
} >> cn8b_chain.log 2>&1
