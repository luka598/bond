import typing as T
from bond.lib.llm.interface import FunctionType

class Function:
    FUNCTION_t: FunctionType
    CALLABLE: T.Callable

    @staticmethod
    def autogen(f: T.Callable):
        raise NotImplementedError() # TODO: Use inspect to get function name, docstring, inputs and outputs. 