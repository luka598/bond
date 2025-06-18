import typing as T
import os

from bond.llm import FunctionType


def _edit_text(path: str, edit_text: str) -> str:
    try:
        with open(path, "r", encoding="utf-8") as f:
            lines = f.readlines()
    except UnicodeDecodeError:
        return f"Error: File '{path}' is not a valid UTF-8 text file."

    try:
        lines: T.Sequence[T.Optional[str]] = [line.rstrip("\n") for line in lines]

        for edit_cmd in edit_text.splitlines():
            parts = edit_cmd.split("|", 2)

            if len(parts) != 3:
                return f"Error: Invalid text edit line format: '{edit_cmd}'. Expected 'NNNN|O|content'."

            try:
                line_num = int(parts[0])
            except ValueError:
                return f"Error: Invalid line number in text edit: '{parts[0]}'."

            op = parts[1]
            if op not in ("U", "D", "I"):
                return f"Error: Invaid operation {parts[1]}. Expected U (update), D (delete) or I (insert)."

            content = parts[2]

            if line_num < len(lines):
                if op == "U":
                    lines[line_num] = content
                elif op == "D":
                    lines[line_num] = None

            elif line_num == len(lines):
                if op == "U":
                    lines.append(content)
                elif op == "D":
                    lines.append(None)

            else:
                for _ in range(len(lines), line_num):
                    lines.append("")

                if op == "U":
                    lines.append(content)
                elif op == "D":
                    lines.append(None)

        with open(path, "w", encoding="utf-8") as f:
            f.write("\n".join([line for line in lines if line is not None]))

        with open(path, "r", encoding="utf-8") as f:
            return "Success"

    except Exception as e:
        return f"Error editing text file: {e}"


def edit(path: str, edit_text: str, fmt: str) -> str:
    if not os.path.exists(path):
        return f"Error: File not found at {path}"
    if os.path.isdir(path):
        return f"Error: Path is a directory, not a file: {path}"

    if fmt == "text":
        result = _edit_text(path, edit_text)
    elif fmt == "binary":
        return "Binary editing is not supported"
    else:
        return "Error: Invalid 'format' specified. Must be 'text' or 'binary'."

    return result


FUNCTION = (
    FunctionType(
        "edit",
        "Edits the content of a file (text or binary) and modifies the actual file on disk. Line count starts at 0.",
        [
            FunctionType.Param("path", "string", "Path to the file to be edited."),
            FunctionType.Param(
                "edit_text",
                "string",
                """"String containing the edits.
                For 'text' format: each line 'NNNN|O|content'. N is line numeber. O is operation, it can either be update 'U' or 'D' delete.
                For 'binary' format: each line 'XXXXXXXX|YY YY...' (hex bytes, no ASCII part).""",
            ),
            FunctionType.Param(
                "fmt",
                "string",
                "Format of the file and edit_text: 'text' or 'binary'.",
            ),
        ],
    ),
    edit,
)
