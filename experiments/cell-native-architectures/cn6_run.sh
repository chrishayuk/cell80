#!/bin/zsh
# CN-6 stage 2 — train both arms (resilient: a failure logs and the queue continues).
cd "$(dirname "$0")"
for arm in generation extraction; do
  echo "==================== CN-6 $arm ===================="
  if python3 cn6_train.py --arm $arm --steps 6000 --bs 16 --unfreeze-top 12 --lr 3e-4; then
    echo "OK: $arm"; else echo "FAILED: $arm (continuing)"; fi
done
echo "done"
