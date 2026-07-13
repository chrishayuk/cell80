#!/bin/zsh
# CN-1 3-arm run at one fixed config (pre-registration amendment 2026-07-13): fingerprint (c),
# shuffled (s, the behaviour-vs-projection control), random (b). Stronger than the confirmed
# top-12 run: top-16 capacity + LR linear decay (fixes the late-run top-1 regression). Then the
# full rank distribution across all three arms.
set -e
cd "$(dirname "$0")"
SEED=${1:-80}
STEPS=${2:-8000}
UNFREEZE=${3:-16}
LR=${4:-8e-4}
for arm in fingerprint shuffled random; do
  echo "==================== ARM $arm (seed $SEED) ===================="
  python3 cn1_train.py --arm $arm --seed $SEED --steps $STEPS --bs 16 --unfreeze-top $UNFREEZE --lr $LR
done
echo "==================== RANK DISTRIBUTION (all arms) ===================="
python3 cn1_eval_ckpt.py --seed $SEED --arms fingerprint shuffled random
