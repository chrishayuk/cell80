#!/bin/zsh
# CN-1 description baseline (mandatory, pre-registration amendment): the CoTools-style *language*
# address — cell row = W_d(sentence_encoder(descriptor)) — that the behaviour arm must beat. Runs
# on both bases at the fixed config, so fingerprint-vs-description is measured on v11 AND SmolLM2.
# The central question: does behaviour beat language as a tool address?
set -e
cd "$(dirname "$0")"
SEED=${1:-80}
python3 cn1_desc_features.py  # ensure the description embeddings are cached
echo "==================== v11 description arm ===================="
python3 cn1_train.py --arm description --seed $SEED --steps 8000 --bs 16 --unfreeze-top 16 --lr 8e-4
echo "==================== SmolLM2 description arm ===================="
python3 cn1_train_hf.py --arm description --seed $SEED --steps 8000 --bs 16 --unfreeze-top 16 --lr 8e-4
echo "==================== done ===================="
