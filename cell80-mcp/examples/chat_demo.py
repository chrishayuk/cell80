#!/usr/bin/env python3
"""Ask a small local model a couple of questions over the cell80 MCP server.

Watch it search the cell library, run the cell it finds, and reuse a memoized
answer on a repeat call — no arithmetic happens in the model itself.

Setup:
    uv run cell80-mcp/examples/chat_demo.py   # resolves mcp + requests on its own
    ollama pull gemma4:e4b       # default model — or any other tool-calling small model
    ollama serve                 # (usually already running as a background service)

Usage:
    uv run cell80-mcp/examples/chat_demo.py --seed          # off-camera: warm the
                                                              #   2nd question's answer
                                                              #   into the server's cache
    uv run cell80-mcp/examples/chat_demo.py                  # on-camera: the two-question demo
    uv run cell80-mcp/examples/chat_demo.py "what is gcd(48, 18)?"   # or ask your own question
    CELL80_MCP_URL=http://127.0.0.1:8021/mcp uv run cell80-mcp/examples/chat_demo.py   # your own server
    CELL80_MODEL=granite4.1:3b uv run cell80-mcp/examples/chat_demo.py   # a different model

`--seed` and the recorded run must hit the SAME server process (the cache is
in-memory). The default URL above (the live fly.io deployment) works for
this now that it's pinned to a single machine — no local server needed. The
cache is shared across every caller though, not per-viewer: if you're not
the first to ask, `--seed` may find the fact already warm, and a fresh
`gcd(1071, 462)` may already read back `cached: true` for the same reason.
Point `CELL80_MCP_URL` at a local `cell80-mcp` instance instead if you'd
rather not depend on the network or want a guaranteed-cold cache.

This is plain MCP + Ollama's HTTP API — no mcp-cli, no chuk-llm, nothing
project-specific. Point any other MCP client at CELL80_MCP_URL instead and
you get the same tools.
"""

from __future__ import annotations

import asyncio
import json
import os
import sys

import requests
from mcp import ClientSession
from mcp.client.streamable_http import streamablehttp_client

CELL80_MCP_URL = os.environ.get("CELL80_MCP_URL", "https://cell80-mcp.fly.dev/mcp")
OLLAMA_URL = os.environ.get("OLLAMA_URL", "http://localhost:11434")
MODEL = os.environ.get("CELL80_MODEL", "gemma4:e4b")

SYSTEM_PROMPT = (
    "You have tool access to a library of deterministic compute cells "
    "(cell_search, cell_run, etc). For ANY arithmetic or math question, you "
    "MUST search for and run the appropriate cell tool rather than computing "
    "the answer yourself. Never do the arithmetic in your own head. cell_run's "
    "reply includes a `cached` field: true means this exact (cell, args) pair "
    "was already known and answered from memory with no new execution; false "
    "means it just ran fresh. Mention plainly which one happened."
)

QUESTIONS = [
    "do 1071 and 462 share a common factor bigger than 20?",
    "and 1071 with 231?",
]

# The second question's answer, seeded ahead of time so `--seed` doesn't have to
# guess a cell id/hash on its own — it discovers the real deployed artifact hash
# via a throwaway call, then imports this as a fact under that hash.
SEED_CELL_ID = "gcd"
SEED_ARGS = [1071, 231]
SEED_RESULT = {"r": [21, 0, 63], "cy": 1391, "tr": 5}


def mcp_tool_to_ollama(tool) -> dict:
    """MCP tool schema -> Ollama/OpenAI function-calling schema."""
    return {
        "type": "function",
        "function": {
            "name": tool.name,
            "description": tool.description or "",
            "parameters": tool.inputSchema,
        },
    }


async def ask(session: ClientSession, tools: list[dict], history: list[dict]) -> str:
    """Run one chat turn, executing any tool calls the model makes, until it answers."""
    while True:
        resp = requests.post(
            f"{OLLAMA_URL}/api/chat",
            json={
                "model": MODEL,
                "messages": history,
                "tools": tools,
                "stream": False,
                "options": {"temperature": 0},
            },
            timeout=300,
        ).json()
        if "error" in resp:
            raise RuntimeError(f"ollama error: {resp['error']}")
        message = resp["message"]
        history.append(message)

        tool_calls = message.get("tool_calls") or []
        if not tool_calls:
            return message.get("content", "")

        for call in tool_calls:
            name = call["function"]["name"]
            args = call["function"]["arguments"]
            if isinstance(args, str):
                args = json.loads(args)
            result = await session.call_tool(name, args)
            text = result.content[0].text if result.content else ""
            print(f"  [tool: {name}  args={args}  -> {text.strip()}]")
            history.append({"role": "tool", "name": name, "content": text})


async def seed(session: ClientSession) -> None:
    """Off-camera: warm the server's cache so question 2 replays from memory
    on-camera. A throwaway call (args that never appear in the recorded
    questions) loads the cell and reveals its real artifact hash without
    touching either recorded question's cache entry; that hash then seeds a
    fact for SEED_ARGS via cell_facts_import."""
    warm = await session.call_tool("cell_run", {"id": SEED_CELL_ID, "args": [2, 3]})
    print(f"  [seed: cell_run id={SEED_CELL_ID} args=[2, 3] (throwaway, learns the hash)]")
    export = await session.call_tool("cell_facts_export", {})
    facts_text = json.loads(export.content[0].text)["facts"]
    art_hash = None
    for line in facts_text.splitlines()[1:]:
        fact = json.loads(line)
        art_hash = fact["a"]
        break
    if art_hash is None:
        raise RuntimeError("seed: could not learn the artifact hash — nothing was exported")

    header = json.dumps({"facts": 1, "lib": "cell80", "producer": "seed", "created": 1, "count": 1})
    fact = json.dumps(
        {
            "a": art_hash,
            "e": "run",
            "args": SEED_ARGS,
            "r": SEED_RESULT["r"],
            "cy": SEED_RESULT["cy"],
            "tr": SEED_RESULT["tr"],
            "h": "ok",
        }
    )
    seed_text = header + "\n" + fact + "\n"
    rep = await session.call_tool("cell_facts_import", {"facts": seed_text, "verify_fraction": 1.0})
    report = json.loads(rep.content[0].text)
    print(f"  [seed: cell_facts_import -> {report}]")
    if report.get("accepted") != 1:
        raise RuntimeError(f"seed: fact was not accepted — {report}")
    print(f"  [seed: {SEED_CELL_ID}{tuple(SEED_ARGS)} now answers from memory on this server]")


async def main() -> None:
    do_seed = "--seed" in sys.argv
    args = [a for a in sys.argv[1:] if a != "--seed"]

    async with streamablehttp_client(CELL80_MCP_URL) as (read, write, _):
        async with ClientSession(read, write) as session:
            await session.initialize()

            if do_seed:
                await seed(session)
                return

            tools = [mcp_tool_to_ollama(t) for t in (await session.list_tools()).tools]
            history = [{"role": "system", "content": SYSTEM_PROMPT}]

            questions = [" ".join(args)] if args else QUESTIONS
            for question in questions:
                print(f"\n> {question}")
                history.append({"role": "user", "content": question})
                answer = await ask(session, tools, history)
                print(answer)


if __name__ == "__main__":
    asyncio.run(main())
