#!/bin/zsh
# CN-1 next phase (pre-registration amendments 2026-07-13): the base-swap top-1 discriminator
# and the 3-seed inversion replication. Ordered by value: SmolLM2 swap first (does a code/math
# prior convert rank->top-1?), then v11 seeds 81/82 (does the pre-registered seen-cell inversion
# replicate?). All at the fixed config the seed-80 v11 run used (top-16, lr 8e-4 + decay, 8000).
set -e
cd "$(dirname "$0")"

echo "############ SmolLM2-135M base swap (top-1 discriminator) ############"
for arm in fingerprint shuffled random; do
  echo "==================== SWAP ARM $arm ===================="
  python3 cn1_train_hf.py --arm $arm --seed 80 --steps 8000 --bs 16 --unfreeze-top 16 --lr 8e-4
done

echo "############ v11 inversion replication (seeds 81, 82) ############"
for seed in 81 82; do
  for arm in fingerprint shuffled random; do
    echo "==================== v11 SEED $seed ARM $arm ===================="
    python3 cn1_train.py --arm $arm --seed $seed --steps 8000 --bs 16 --unfreeze-top 16 --lr 8e-4
  done
done

echo "############ done ############"
