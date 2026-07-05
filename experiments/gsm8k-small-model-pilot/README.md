# GSM8K small-model extraction pilot — scripts

Findings and full write-up: `../gsm8k-small-model-pilot-findings.md`. This directory holds
the actual scripts that produced them, plus the saved transcripts (`results/`), so the runs
are reproducible rather than only reported.

Needs: Ollama running locally with the target model pulled, and the real `cell80` release
binary built (`cargo build --release -p cell80 --bin cell80` from the repo root).

- `gsm8k_small_model_pilot.py` — Part 1: single-shot extraction, no tools. The model writes
  a plan-IR JSON blob in plain chat; this script parses it and runs it through the real
  `cell80 solve` binary. `PILOT_MODEL`/`PILOT_RESULTS_FILE` env vars select the model/output.
- `gsm8k_repair_round.py` — Part 1's one-shot repair round (reads a prior run's
  `pilot_results.json`, feeds each failure's own output + diagnostic back for one retry,
  mirroring `cell-eval/src/cell_eval/repair.py`'s philosophy).
- `gsm8k_tool_calling_pilot.py` — Part 2a: `cell_solve` as a real OpenAI-format tool, driven
  over Ollama's OpenAI-compatible endpoint via a small inline multi-turn tool-calling loop
  (mirrors `cell-eval/src/cell_eval/agent.py`'s `run_episode` shape; kept self-contained
  since the `cell_solve` tool schema was a pilot-only addition, not shipped in `cell-eval`).
- `gsm8k_native_ollama_pilot.py` — Part 2b/2c: the same tool-calling test over Ollama's
  *native* `/api/chat` endpoint instead (matching `cell80-mcp/examples/chat_demo.py`'s
  proven-working pattern). `PILOT_MODEL` selects the model (tested: `granite4.1:3b`,
  `qwen2.5:3b`, `gemma4:e4b`, `qwen3.5:4b`); the schema-strictness variants described in the
  findings doc were tested by editing `SOLVE_TOOL`'s `parameters` in place, not via a flag —
  the three variants aren't preserved as separate files, since the point was to find the
  breaking points, not to keep every intermediate revision.

`results/` holds the saved transcripts for the runs that completed in full (all granite4.1:3b,
Part 1 and 2a). The Part 2b/2c cross-model and schema-variant checks were deliberately small
(N=1-2 diagnostic samples, not full 20-problem batches) and run interactively rather than
via a saved-results harness — see the findings doc for those transcripts inline.
