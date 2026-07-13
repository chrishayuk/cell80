#!/usr/bin/env python3
"""CN-1 usable-level target, derived from execution economics BEFORE the faithful arms land (so the
mechanism is judged against a standard specified from the architecture, not one reverse-engineered
from the result).

The two-tier design: the model narrows to top-K, the runtime EXECUTES those K to resolve. Execution
is nearly free (µs/cell), so the requirement is NOT top-1 or an arbitrary per-cell top-50 recall —
it is: **is the true cell within the number of candidates you can execute per token, K_exec?**
K_exec is set by hardware and token rate, NOT by library size. So the deciding question is whether
the true cell's ABSOLUTE rank stays <= K_exec as the library grows (scale-invariant -> usable) or
grows to a fixed fraction (fractional -> unusable at scale).

Run: python3 cn1_usable_target.py
"""
TOK_RATE = 117.0          # tok/s (LARQL Gemma 3 4B reference)
BUDGET_S = 1.0 / TOK_RATE  # ~8.5 ms/token
P = 32                    # probes/candidate to separate cells at ~0.245 agreement (0.245**16 ~1e-9)
GPU_EVAL_S = 1 / 3.7e8    # GPU interpreter 3.7e8 evals/s (memory: gpu-cells-e1)
CPU_EVAL_S = 1e-6         # ~1 µs/eval (order-of-magnitude CPU)


def main():
    print(f"token budget: {BUDGET_S*1e3:.1f} ms  (at {TOK_RATE:.0f} tok/s) | P={P} probes/candidate\n")
    for label, t in [("GPU 3.7e8/s", GPU_EVAL_S), ("CPU ~1us/eval", CPU_EVAL_S)]:
        K = BUDGET_S / (P * t)
        print(f"  {label:14} K_exec = {K:,.0f} executable candidates / token")
    print("\nUSABLE TARGET (execution-derived): true cell's ABSOLUTE rank <= K_exec")
    print("  ~260 (CPU) .. ~100,000 (GPU), FIXED by hardware/token-rate, independent of library size.")
    print("\nCurrent held-out median ABSOLUTE rank = 114 (random-sampled, 790-cell library):")
    for N in [790, 1_000_000]:
        print(f"  library {N:>9,}: rank-114 absolute -> {'USABLE (114 << K_exec)' if 114 < 260 else 'check'};"
              f"  rank-14.4% -> {0.144*N:,.0f} candidates -> {'usable' if 0.144*N < 260 else 'UNUSABLE on CPU'}")
    print("\n=> The deciding metric is top-K_exec recall + ABSOLUTE-rank-vs-library-size (scale-invariance),")
    print("   NOT per-cell top-50 recall. rank-114 already clears K_exec at 790; scale-invariance decides at 1e6.")


if __name__ == "__main__":
    main()
