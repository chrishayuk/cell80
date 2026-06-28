"""A minimal stand-in for an OpenAI-compatible client — replays a scripted list of assistant
turns so the agent loop (adoption / composition) can be tested offline. Not a test module."""

import json


class _Fn:
    def __init__(self, name, args):
        self.name = name
        self.arguments = json.dumps(args)


class _ToolCall:
    _n = 0

    def __init__(self, name, args):
        _ToolCall._n += 1
        self.id = f"call_{_ToolCall._n}"
        self.function = _Fn(name, args)

    def model_dump(self):
        return {
            "id": self.id,
            "type": "function",
            "function": {"name": self.function.name, "arguments": self.function.arguments},
        }


class _Msg:
    def __init__(self, content=None, tool_calls=None):
        self.content = content
        self.tool_calls = tool_calls


class _Resp:
    def __init__(self, message):
        self.choices = [type("C", (), {"message": message})()]


class _Completions:
    def __init__(self, script):
        self._script = list(script)
        self._i = 0

    def create(self, **_):
        item = self._script[self._i]
        self._i += 1
        if isinstance(item, Exception):
            raise item
        return _Resp(item)


class FakeClient:
    """Replays a scripted list of assistant turns (each an _Msg, or an Exception to raise)."""

    def __init__(self, script):
        self.chat = type("Chat", (), {"completions": _Completions(script)})()
