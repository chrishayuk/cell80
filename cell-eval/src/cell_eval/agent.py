"""The shared LLM agent loop for the cell evals (adoption, composition).

One tool-using episode over an **OpenAI-compatible** endpoint (Ollama by default): given a
fixed steering prompt, a user task, and a set of cell tools, run the model turn-by-turn —
dispatching its tool calls against the warm `CellLibrary` — until it answers `ANSWER: <int>`.
Gated: nothing imports `openai` until `make_client` is called.
"""

from __future__ import annotations

import json
import os
import re
from dataclasses import dataclass

from .tools import ToolTrace, dispatch

DEFAULT_BASE_URL = "http://localhost:11434/v1"  # Ollama's OpenAI-compatible endpoint
DEFAULT_API_KEY = "ollama"  # Ollama ignores it, but the SDK requires a non-empty key

_ANSWER_RE = re.compile(r"ANSWER:\s*(-?\d+)", re.IGNORECASE)


@dataclass
class AgentConfig:
    model: str
    base_url: str = DEFAULT_BASE_URL
    api_key: str = DEFAULT_API_KEY
    max_turns: int = 8
    temperature: float = 0.0

    @classmethod
    def from_env(cls, model: str | None = None) -> "AgentConfig":
        m = model or os.environ.get("CELL_EVAL_MODEL")
        if not m:
            raise ValueError(
                "no model set — pass --model or set CELL_EVAL_MODEL "
                "(e.g. a model you've pulled in Ollama like 'qwen2.5' or 'llama3.1')"
            )
        return cls(
            model=m,
            base_url=os.environ.get("CELL_EVAL_BASE_URL", DEFAULT_BASE_URL),
            api_key=os.environ.get("CELL_EVAL_API_KEY", DEFAULT_API_KEY),
            max_turns=int(os.environ.get("CELL_EVAL_MAX_TURNS", "8")),
        )


def parse_answer(text: str) -> int | None:
    matches = _ANSWER_RE.findall(text or "")
    return int(matches[-1]) if matches else None


def make_client(cfg: AgentConfig):
    """Construct the OpenAI-compatible client, importing `openai` lazily (so the retrieval
    eval and offline tests stay dependency-free)."""
    try:
        from openai import OpenAI
    except ImportError as e:
        raise RuntimeError(
            "the LLM evals need the OpenAI client — install with: "
            "pip install 'cell-eval[adoption]'"
        ) from e
    return OpenAI(base_url=cfg.base_url, api_key=cfg.api_key)


@dataclass
class Episode:
    """The outcome of one agent episode."""

    answer: int | None
    trace: ToolTrace
    turns: int
    error: str | None = None


def run_episode(client, cfg: AgentConfig, lib, user_prompt: str, system_prompt: str, tools: list) -> Episode:
    """Drive one tool-using episode: loop create→dispatch until the model answers or runs out
    of turns. `tools` is the OpenAI tool schema list the model may call; all calls are routed
    through `dispatch` (which records into a `ToolTrace`). Endpoint errors are captured, not
    raised."""
    trace = ToolTrace()
    messages = [
        {"role": "system", "content": system_prompt},
        {"role": "user", "content": user_prompt},
    ]
    answer, turns, error = None, 0, None
    try:
        for turns in range(1, cfg.max_turns + 1):
            resp = client.chat.completions.create(
                model=cfg.model,
                messages=messages,
                tools=tools,
                temperature=cfg.temperature,
            )
            msg = resp.choices[0].message
            tool_calls = msg.tool_calls or []
            messages.append(
                {
                    "role": "assistant",
                    "content": msg.content or "",
                    "tool_calls": [tc.model_dump() for tc in tool_calls] or None,
                }
            )
            if not tool_calls:
                answer = parse_answer(msg.content or "")
                break
            for tc in tool_calls:
                try:
                    args = json.loads(tc.function.arguments or "{}")
                except json.JSONDecodeError:
                    args = {}
                result = dispatch(lib, tc.function.name, args, trace)
                messages.append(
                    {"role": "tool", "tool_call_id": tc.id, "content": json.dumps(result)}
                )
    except Exception as e:  # network / endpoint / SDK error — record, don't crash the run
        error = f"{type(e).__name__}: {e}"
    return Episode(answer=answer, trace=trace, turns=turns, error=error)
