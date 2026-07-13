#!/bin/zsh
# CN-1 next phase — RESILIENT runner (no set -e): a crash in one run logs and the queue
# continues, so a transient MPS failure can't destroy the whole batch (the earlier lesson).
# SmolLM2 swap (top-1 discriminator) then v11 seeds 81/82 (inversion replication). HF eval now
# runs on CPU (dodges the MPSGraph LM-head matmul crash); checkpoints saved before eval either way.
cd "$(dirname "$0")"
run () {  # run <label> <cmd...>
  echo "==================== $1 ===================="
  if "${@:2}"; then echo "OK: $1"; else echo "FAILED: $1 (continuing)"; fi
}

echo "############ SmolLM2-135M base swap ############"
for arm in fingerprint shuffled random; do
  run "SWAP $arm" python3 cn1_train_hf.py --arm $arm --seed 80 --steps 8000 --bs 16 --unfreeze-top 16 --lr 8e-4
done

echo "############ v11 inversion replication (seeds 81, 82) ############"
for seed in 81 82; do
  for arm in fingerprint shuffled random; do
    run "v11 s$seed $arm" python3 cn1_train.py --arm $arm --seed $seed --steps 8000 --bs 16 --unfreeze-top 16 --lr 8e-4
  done
done
echo "############ done ############"
