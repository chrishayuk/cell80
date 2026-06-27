"""cell80 agent eval harness.

The headline question for cell80 is not whether the VM works — it does — but whether
*an agent reliably retrieves and runs the right cell instead of writing code*. This
package measures that, and it measures **two numbers, not one** (per the roadmap):

* **retrieval precision** — given a query, is the right cell in the top-k? Deterministic,
  no LLM, runs anywhere. This reads the *index* quality directly. See `retrieval`.
* **adoption** — given a task, does an LLM agent actually `search → inspect → run` a cell
  (and get the right answer) instead of doing the arithmetic itself? Needs a live model
  over an OpenAI-compatible endpoint (Ollama by default). See `adoption`.

They fail for different reasons: low adoption is usually weak *steering* (the system
prompt), not bad retrieval. So the harness holds steering fixed and lets you vary the
library, so a one-line preamble fix is not misdiagnosed as a week of index tuning.
"""

from .library import open_library, seed_library_dir

__all__ = ["open_library", "seed_library_dir"]
