import os
from bond.llm import FunctionType


def _is_text_file(path: str) -> bool:
    try:
        with open(path, "rb") as f:
            chunk = f.read(1024)
            chunk.decode("utf-8")
        return True
    except UnicodeDecodeError:
        return False
    except Exception as e:
        return False


def _view_text(path: str, offset: int = 0) -> str:
    output = []
    try:
        with open(path, "r", encoding="utf-8", errors="ignore") as f:
            for _ in range(offset):
                next(f, None)

            for i, line in enumerate(f):
                if i >= 500:
                    break
                output.append(f"{offset + i:4d}|{line.rstrip()}")
    except FileNotFoundError:
        return f"Error: File not found at {path}"
    except Exception as e:
        return f"Error reading text file: {e}"
    return "\n".join(output)


def view(path: str, offset: int = 0) -> str:
    if not os.path.exists(path):
        return f"Error: Path not found: {path}"
    if os.path.isdir(path):
        return f"Error: Path is a directory, not a file: {path}"

    if _is_text_file(path):
        return _view_text(path, offset)
    else:
        return "Binary viewing is not supported"


FUNCTION = (
    FunctionType(
        "view",
        "Views the content of a file (text or binary). Line count starts at 0.",
        [
            FunctionType.Param("path", "string", "Path to the file."),
            FunctionType.Param(
                "offset",
                "integer",
                "Starting offset (line number for text, byte for binary). If unsure set to 0.",
            ),
        ],
    ),
    view,
)
