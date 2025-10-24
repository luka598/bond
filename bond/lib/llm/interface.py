import typing as T
from collections import namedtuple

from bond.config import Config

ROLE_t = T.Literal["system", "user", "llm"]


class TextMsg:
    def __init__(self, role: ROLE_t, data: str) -> None:
        self.role = role
        self.data = data


class ImageMsg:
    def __init__(self, role: ROLE_t, data: str) -> None:
        self.role = role
        self.data = data


class FunctionCallMsg:
    def __init__(
        self,
        name: str,
        params: T.Dict[str, T.Any],
    ) -> None:
        self.name = name
        self.params = params


class FunctionResultMsg:
    def __init__(self, name: str, data: str) -> None:
        self.name = name
        self.data = data


class ErorrMsg:
    def __init__(self, data: str, ext: T.Any = None) -> None:
        self.data = data
        self.ext = ext


MSG_t = T.Union[TextMsg, ImageMsg, FunctionCallMsg, FunctionResultMsg, ErorrMsg]


class FunctionType:
    Param = namedtuple("function_param", ["name", "type", "description"])

    def __init__(
        self,
        name: str,
        description: str,
        params: T.List["FunctionType.Param"],
    ) -> None:
        self.name = name
        self.description = description
        self.params = params


class LLM:
    def __init__(self, config: Config) -> None:
        self.config = config

    def info(self) -> str:
        return ""

    def send(
        self, messages: T.List[MSG_t], functions: T.List[FunctionType]
    ) -> T.Sequence[MSG_t]:
        raise NotImplementedError()
