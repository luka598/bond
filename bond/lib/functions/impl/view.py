import pathlib
from bond.lib.functions.interface import Function, FunctionType

LINES_COUNT = 1024

def _is_text_file(path: pathlib.Path) -> bool:
    try:
        with open(path, "rb") as f:
            chunk = f.read(1024)
            chunk.decode("utf-8")
        return True
    except UnicodeDecodeError:
        return False
    except Exception as e:
        return False


def _view_text(path: pathlib.Path, offset: int):
    lines = []
    with open(path, "r") as f:
        while offset:
            f.readline()
            offset -= 1

        lines_read = 0
        while ((line := f.readline()) and lines_read < LINES_COUNT):
            lines.append(f"{offset + lines_read}|{line}")
            lines_read += 1

    return {"success": True, "output": "\n".join(lines)}


def view(path: str, offset: int):
    p = pathlib.Path(path)
    if not p.exists():
        return {"success": False, "error": "Path not found"}
    if not p.is_file():
        return {"success": False, "error": "Path is not a file"}

    if _is_text_file(p):
        return _view_text(p, offset)
    else:
        return {"success": False, "error": "Reading binary files is not supported"}


class ViewFunction(Function):
    FUNCTION_t = FunctionType(
        "view",
        "Views the content of a file (text or binary). Line numbers start at 0.",
        [
            FunctionType.ParamLiteral("path", "string", "Path to the file."),
            FunctionType.ParamLiteral(
                "offset",
                "integer",
                "Starting offset (line number for text, byte for binary). If unsure set to 0.",
            ),
        ],
    )
    CALLABLE = view
