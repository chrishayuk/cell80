#!/usr/bin/env python3
"""Ask a small local model a couple of questions over the cell80 MCP server.

Watch it search the cell library, run the cell it finds, and reuse a memoized
answer on a repeat call — no arithmetic happens in the model itself.

Setup:
    pip install mcp requests
    ollama pull granite4.1:3b   # or any other tool-calling-capable small model
    ollama serve                # (usually already running as a background service)

Usage:
    python chat_demo.py                                    # runs the two-question demo
    python chat_demo.py "what is gcd(48, 18)?"              # or ask your own question
    CELL80_MCP_URL=http://127.0.0.1:8021/mcp python chat_demo.py   # against your own server

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
MODEL = os.environ.get("CELL80_MODEL", "granite4.1:3b")

SYSTEM_PROMPT = (
    "You have tool access to a library of deterministic compute cells "
    "(cell_search, cell_run, etc). For ANY arithmetic or math question, you "
    "MUST search for and run the appropriate cell tool rather than computing "
    "the answer yourself. Never do the arithmetic in your own head."
)

QUESTIONS = [
    "do 1071 and 462 share a common factor bigger than 20?",
    "and 1071 with 231?",
]


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


async def main() -> None:
    async with streamablehttp_client(CELL80_MCP_URL) as (read, write, _):
        async with ClientSession(read, write) as session:
            await session.initialize()
            tools = [mcp_tool_to_ollama(t) for t in (await session.list_tools()).tools]
            history = [{"role": "system", "content": SYSTEM_PROMPT}]

            questions = [" ".join(sys.argv[1:])] if len(sys.argv) > 1 else QUESTIONS
            for question in questions:
                print(f"\n> {question}")
                history.append({"role": "user", "content": question})
                answer = await ask(session, tools, history)
                print(answer)


if __name__ == "__main__":
    asyncio.run(main())
