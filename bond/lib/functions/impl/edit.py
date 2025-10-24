import pathlib
from bond.lib.functions.interface import Function, FunctionType

def _edit_text(path: pathlib.Path, begin_line: int, end_line: int, text: str):
    with open(path, "r") as f:
        lines = f.readlines()

    num_lines = len(lines)

    if not (0 <= begin_line <= num_lines):
        return {"success": False, "error": f"Invalid begin_line: {begin_line}. Must be between 0 and {num_lines} (inclusive)."}
    if not (0 <= end_line <= num_lines):
        return {"success": False, "error": f"Invalid end_line: {end_line}. Must be between 0 and {num_lines} (inclusive)."}
    if begin_line > end_line:
        return {"success": False, "error": f"begin_line ({begin_line}) cannot be greater than end_line ({end_line})"}

    new_text_lines = text.splitlines()

    new_lines_list = lines[:begin_line]
    new_lines_list.extend(new_text_lines)
    new_lines_list.extend(lines[end_line:])

    new_content = "\n".join(new_lines_list)

    with open(path, "w") as f:
        f.write(new_content)

    return {"success": True}


def edit(path: str, begin_line: int, end_line: int, text: str, fmt: str):
    p = pathlib.Path(path)
    if not p.exists():
        open(p, "w").close()
        # return {"success": False, "error": "Path not found"}
    if not p.is_file():
        return {"success": False, "error": "Path is not a file"}

    if fmt == "text":
        return _edit_text(p, begin_line, end_line, text)
    elif fmt == "binary":
        return {"success": False, "error": "Writing binary files is not supported"}

class EditFunction(Function):
    FUNCTION_t = FunctionType(
        "edit",
        "Edits the content of a file by replacing text between specified line numbers. Line numbers start at 0.",
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
    )
    CALLABLE=edit