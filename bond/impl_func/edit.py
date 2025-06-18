import typing as T
import os

from bond.llm import FunctionType


def _edit_text(path: str, begin_line: int, end_line: int, text: str) -> str:
    try:
        with open(path, "r", encoding="utf-8") as f:
            lines = f.readlines()
    except UnicodeDecodeError:
        return f"Error: File '{path}' is not a valid UTF-8 text file."

    try:
        lines = [line.rstrip("\n") for line in lines]
        new_lines = text.splitlines()

        if begin_line < 0 or begin_line > len(lines):
            return f"Error: Invalid begin_line {begin_line}. Must be between 0 and {len(lines)}"
        if end_line < begin_line:
            return f"Error: end_line ({end_line}) must be greater than begin_line ({begin_line})"

        lines[begin_line:end_line] = new_lines

        with open(path, "w", encoding="utf-8") as f:
            f.write("\n".join(lines))

        return "Success"

    except Exception as e:
        return f"Error editing text file: {e}"


def edit(path: str, begin_line: int, end_line: int, text: str, fmt: str) -> str:
    if not os.path.exists(path):
        return f"Error: File not found at {path}"
    if os.path.isdir(path):
        return f"Error: Path is a directory, not a file: {path}"

    if fmt == "text":
        result = _edit_text(path, begin_line, end_line, text)
    elif fmt == "binary":
        return "Binary editing is not supported"
    else:
        return "Error: Invalid 'format' specified. Must be 'text' or 'binary'."

    return result


FUNCTION = (
    FunctionType(
        "edit",
        "Edits the content of a file by replacing text between specified line numbers. Line count starts at 0.",
        [
            FunctionType.Param("path", "string", "Path to the file to be edited."),
            FunctionType.Param(
                "begin_line", "integer", "Starting line number (inclusive)."
            ),
            FunctionType.Param(
                "end_line", "integer", "Ending line number (exclusive)."
            ),
            FunctionType.Param(
                "text", "string", "New text to insert between begin_line and end_line."
            ),
            FunctionType.Param(
                "fmt",
                "string",
                "Format of the file: 'text' or 'binary'.",
            ),
        ],
    ),
    edit,
)
