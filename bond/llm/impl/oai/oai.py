import typing as T
import json

from bond.config import Config
from bond.llm.interface import (
    LLM,
    MSG_t,
    ROLE_t,
    TextMsg,
    FunctionResultMsg,
    FunctionType,
    FunctionCallMsg,
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
                "FUNCTION CALL RESULT: "
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
        "name": f.name,
        "description": f.description,
        "parameters": {
            "type": "object",
            "properties": {n: {"type": t, "description": d} for (n, t, d) in f.params},
            "required": [n for (n, _, _) in f.params],
        },
    }


class OAILLM(LLM):
    ENDPOINT = "https://api.openai.com/v1/chat/completions"

    def __init__(self, config: Config) -> None:
        super().__init__(config)
        self.api_key = self.config["api_key"]
        self.model = self.config["model"]

        self.HEADERS = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json",
        }

    def send(
        self, messages: T.List[MSG_t], functions: T.List[FunctionType]
    ) -> T.List[MSG_t]:
        payload = {"model": self.model, "messages": [convert_msg(m) for m in messages]}

        if functions:
            payload["functions"] = [convert_function(f) for f in functions]
            payload["function_call"] = "auto"

        resp = requests.post(self.ENDPOINT, headers=self.HEADERS, json=payload)
        if resp.status_code != 200:
            print(resp.text)
            return []
        j = resp.json()

        choice = j["choices"][0]["message"]
        fcall = choice.get("function_call")
        if fcall:
            return [FunctionCallMsg(fcall["name"], json.loads(fcall["arguments"]))]
        else:
            return [TextMsg("llm", choice["content"])]
