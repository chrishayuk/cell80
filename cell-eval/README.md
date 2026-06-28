# cell-eval — the cell80 agent eval harness

The headline question for cell80 isn't whether the VM works (it does). It's whether
**an agent reliably retrieves and runs the right `.cell` instead of writing code.** That
is the thesis. This package measures it.

It measures the whole arc — **find → run → compose** — as three numbers that fail for
different reasons:

| number | question | needs a model? |
|---|---|---|
| **retrieval precision** | given a query, is the right cell in the top-k? | no — deterministic |
| **adoption** | given a task, does an agent actually `search → inspect → run` a cell (and get it right) instead of doing the math itself? | yes — OpenAI-compatible endpoint |
| **composition** | given a task needing several cells, does it *wire them together* (via `cell_graph_run`) instead of doing the multi-step math itself? | yes — OpenAI-compatible endpoint |

Low adoption is usually weak **steering** (the system prompt), not bad retrieval. So the
harness **holds steering fixed** (one constant in `adoption.py`) and lets you vary the
library/model — so a one-line preamble fix isn't misdiagnosed as a week of index tuning,
and vice-versa.

Both evals drive the same `CellLibrary` the MCP server exposes, so `search`/`inspect`/`run`
go through the identical code path an agent gets over MCP — no separate mock surface.

## Install

```bash
# from the repo root, in a venv with cell80_py built (see repo README):
maturin develop -m cell80-py/Cargo.toml --release   # builds the engine
pip install -e cell80-mcp/                           # the CellLibrary + MCP tools
pip install -e 'cell-eval/[adoption]'                # this harness (drop [adoption] for retrieval-only)
```

## Retrieval eval (deterministic — run this anywhere)

```bash
cell-eval retrieval                 # human-readable
cell-eval retrieval --k 3 --json    # structured
cell-eval retrieval --fail-under 0.7   # exit 1 if P@1 below threshold (CI guard)
cell-eval retrieval --library path/to/other/cells   # eval a different library
```

Dataset: [`datasets/retrieval.jsonl`](datasets/retrieval.jsonl) — one
`{query, expected, category}` per line. Categories:

- **direct** — the query uses the library's own vocabulary
- **paraphrase** — a natural rewording that *avoids* the cell's tag words
- **adversarial** — deliberately tricky for token-overlap ranking

The paraphrase + adversarial rows are the point: *a `.cell` is only useful if findable when
the user doesn't speak the library's vocabulary.*

### Baseline (standard library, 98 cells, k=5)

```
OVERALL        P@1=0.69  hit@3=0.81  hit@5=0.85  MRR=0.75
  direct       P@1=0.92  hit@3=0.97  hit@5=0.98  MRR=0.94
  paraphrase   P@1=0.40  hit@3=0.58  hit@5=0.66  MRR=0.49
  adversarial  P@1=0.35  hit@3=0.62  hit@5=0.73  MRR=0.49
```

The story in one line: **token-overlap search is near-perfect on the library's own words and
a coin-flip under paraphrase** — and growing 8 → 98 cells made it *harder*, exactly as
intended (more confusable siblings = a more honest benchmark). Concrete misses today are
sibling collisions:

- *"is this number within the allowed limits"* → `is_le` / `bit_is_set` win — `range_check`
  not in top-k (a boolean three-bound query, but the words don't overlap).
- *"the largest integer that divides both numbers evenly"* → `gcd3` / `lcm` outrank `gcd`.
- *"grid distance between two points"* → `chebyshev` / `euclid_sq` tie and beat `manhattan`.

This is the measured case for **roadmap item 3 (type-led index)**: rank on the typed
signature first (`range_check : (x,lo,hi)->bool` *is* a boolean-output, three-bound query),
embeddings as the tiebreaker. Re-run this eval to know if a change actually helped — the
direct row is the regression guard, the paraphrase row is the score to move.

## Adoption eval (LLM agent — OpenAI-compatible, Ollama by default)

```bash
# point at any model you've pulled in Ollama:
cell-eval adoption --model qwen2.5
cell-eval adoption --model llama3.1 --json

# or any OpenAI-compatible endpoint:
CELL_EVAL_BASE_URL=http://host:11434/v1 CELL_EVAL_MODEL=qwen2.5 cell-eval adoption
```

Config (env or flags):

| env | default | meaning |
|---|---|---|
| `CELL_EVAL_BASE_URL` | `http://localhost:11434/v1` | OpenAI-compatible endpoint (Ollama) |
| `CELL_EVAL_API_KEY`  | `ollama` | ignored by Ollama; the SDK needs *something* |
| `CELL_EVAL_MODEL`    | — (required) | model name, e.g. `qwen2.5`, `llama3.1` |
| `CELL_EVAL_MAX_TURNS`| `8` | tool-call rounds per task |

> Use a tool-calling-capable model — the agent loop needs OpenAI-style function calling.

Dataset: [`datasets/tasks.jsonl`](datasets/tasks.jsonl) — `{prompt, expected, cell}` per
line; prompts are phrased as a user would ask, so this measures retrieval **and** adoption
together. Three signals per task:

- **adoption** — did it call `cell_run` at all (vs. answering from its head)?
- **correct** — is the final `ANSWER: <n>` right?
- **correct_via_cell** — correct *and* it ran a cell (the outcome we want).

### Baseline (gemma-4-26B-A4B via Ollama, 8 tasks)

```
adoption=0.75   correct=1.00   correct_via_cell=0.75
```

Whenever the model reached for a cell it found and ran the right one (correct=1.00). The two
non-adoptions — `max` of 17/42, and *"is 25 within 1–10"* — were answered directly in one
turn: the model shortcuts the cell when the arithmetic is trivial. That's an **adoption**
gap (steering / task difficulty), not a retrieval gap — which is the whole reason the two
numbers are tracked apart. Use a tool-calling model: `gemma3` does **not** support tools in
Ollama; `gemma-4-26B-A4B` does.

## Composition eval (the capstone)

Adoption asks "did it run the *right cell*"; composition asks the harder question — given a
task that needs **several** cells, does the agent **wire them together** instead of doing the
multi-step arithmetic itself? Same fixed-steering discipline; the agent also gets the
`cell_graph_run` tool (compose cells into a host-routed, type-checked graph).

```bash
cell-eval composition --model qwen2.5        # Ollama by default
cell-eval composition --model llama3.1 --json
```

Dataset: [`datasets/composition_tasks.jsonl`](datasets/composition_tasks.jsonl) — each task
needs ≥2 cells (one's output feeds the next), e.g. *"manhattan distance from (3,4) to (10,8),
score it `dist + 2·risk + 3·cost`, then clamp to 0–10."* Three signals per task:

- **composed** — did it wire cells (a `cell_graph_run` with ≥2 nodes, or ≥2 distinct cells run)?
- **correct** — is the final `ANSWER: <n>` right?
- **correct_via_composition** — correct *and* composed (the outcome we want).

This is the proof the graph matters: the consumer doesn't just *find* a tool, it *builds* one
from several.

### Baseline (granite4.1:3b via Ollama, 6 tasks)

```
composed=0.50   used_graph=0.00   correct=0.83   correct_via_composition=0.50
```

The finding that only the eval surfaces: granite **composes by chaining `cell_run` calls**
(chain:2, chain:3) and **never authors a graph** (`used_graph=0.00`) — the JSON graph
manifest is too much for a 3B to construct from scratch. So composition happens, but not via
`cell_graph_run`; closing that gap is a steering / tool-ergonomics problem, not a VM one.
(For contrast, the *same* model scores **adoption 1.00 / correct 1.00** on the single-cell
tasks — it's an eager, reliable tool-caller; the hard part is graph *authoring*, not tool use.)

## Layout

```
src/cell_eval/
  library.py     locate the seed lib + open the real CellLibrary
  retrieval.py   deterministic retrieval eval + report
  metrics.py     precision@1, hit@k, MRR
  tools.py       cell tools as OpenAI function schemas + dispatcher (mirrors MCP)
  agent.py       the shared OpenAI-compatible agent loop (adoption + composition)
  adoption.py    fixed steering + single-cell scoring
  composition.py fixed steering + the graph tool + composition scoring
  report.py      human-readable rendering
  __main__.py    `cell-eval retrieval | adoption | composition`
datasets/        retrieval.jsonl, tasks.jsonl, composition_tasks.jsonl
tests/           deterministic; no network (the LLM network path is run by you)
```

## Typed-state cells

State cells (e.g. `manhattan`, with named fields `x1,y1,x2,y2,dist`) are driven **by name**:
`cell_run` takes a `fields` object `{name: int}` and returns the full post-run `state`. The
harness wires this through automatically (`tools.dispatch` routes `fields` →
`CellLibrary.run_state`), and `manhattan` is in the adoption tasks. This is roadmap item 2
(done) — and the wiring substrate for the networked CellGraph.
