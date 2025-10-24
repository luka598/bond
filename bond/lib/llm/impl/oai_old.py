import typing as T
import json

from bond.config import Config
from bond.lib.llm.interface import (
    LLM,
    MSG_t,
    ROLE_t,
    TextMsg,
    FunctionResultMsg,
    FunctionType,
    FunctionCallMsg,
    ErorrMsg,
)

import requests


def translate_role(role: ROLE_t):
    if role == "system":
        return "system"
    elif role == "user":
        return "user"
    elif role == "llm":
        return "assistant"
    else:
        raise RuntimeError(f"Unknown role: {role}")


def convert_msg(msg: MSG_t):
    if isinstance(msg, TextMsg):
        return {
            "role": translate_role(msg.role),
            "content": [{"type": "text", "text": msg.data}],
        }
    elif isinstance(msg, FunctionCallMsg):
        return {
            "role": "assistant",
            "function_call": {"name": msg.name, "arguments": json.dumps(msg.params)},
        }
    elif isinstance(msg, FunctionResultMsg):
        return convert_msg(
            TextMsg(
                "system",
                "FUNCTION CALL RESULT (USER CANT SEE THIS): "
                + json.dumps({"name": msg.name, "content": msg.data}),
            )
        )
    else:
        print(f"Unknown message: {msg}")
        return {
            "role": translate_role("system"),
            "content": [{"type": "text", "text": ""}],
        }


def convert_function(f: FunctionType):
    return {
        "type": "function",
        "function": {
            "name": f.name,
            "description": f.description,
            "parameters": {
                "type": "object",
                "properties": {
                    n: {"type": t, "description": d} for (n, t, d) in f.params
                },
                "required": [n for (n, _, _) in f.params],
            },
        },
    }


class OAILLM(LLM):
    ENDPOINT = ""

    def __init__(self, config: Config) -> None:
        super().__init__(config)
        self.api_key = self.config["provider"]["api_key"]
        self.model_name = self.config["provider"]["model"]

        self.HEADERS = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json",
        }

    def send(
        self, messages: T.List[MSG_t], functions: T.List[FunctionType]
    ) -> T.List[MSG_t]:
        payload = {
            "model": self.model_name,
            # "reasoning_effort": "high",
            "messages": [convert_msg(m) for m in messages],
        }

        if functions:
            payload["tools"] = [convert_function(f) for f in functions]
            payload["tool_choice"] = "auto"

        resp = requests.post(self.ENDPOINT, headers=self.HEADERS, json=payload)
        if resp.status_code != 200:
            return [ErorrMsg(f"Response status code: {resp.status_code} != 200", resp.text)]
        j = resp.json()

        finish_reason = j["choices"][0]["finish_reason"]
        choice = j["choices"][0]["message"]
        if finish_reason == "tool_calls":
            fcall = choice.get("tool_calls")[0]["function"]
            return [FunctionCallMsg(fcall["name"], json.loads(fcall["arguments"]))]
        elif "content" not in choice:
            return []
        else:
            return [TextMsg("llm", choice["content"])]
