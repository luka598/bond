import pathlib
from bond.lib.functions.interface import Function, FunctionType


def _edit_text(path: pathlib.Path, begin_line: int, end_line: int, text: str):
    with open(path, "r") as f:
        lines = f.readlines()

    lines = [x.strip() for x in lines]
    num_lines = len(lines)

    if not (0 <= begin_line <= num_lines):
        return {
            "success": False,
            "error": f"Invalid begin_line: {begin_line}. Must be between 0 and {num_lines} (inclusive).",
        }
    if not (0 <= end_line <= num_lines):
        return {
            "success": False,
            "error": f"Invalid end_line: {end_line}. Must be between 0 and {num_lines} (inclusive).",
        }
    if begin_line > end_line:
        return {
            "success": False,
            "error": f"begin_line ({begin_line}) cannot be greater than end_line ({end_line})",
        }

    new_text_lines = text.splitlines()

    new_lines_list = lines[:begin_line]
    new_lines_list.extend(new_text_lines)
    new_lines_list.extend(lines[end_line:])

    new_content = "\n".join(new_lines_list)

    with open(path, "w") as f:
        f.write(new_content)

    return {"success": True}


def edit(path: str, begin_line: int, end_line: int, text: str):
    p = pathlib.Path(path)
    if not p.exists():
        open(p, "w").close()
        # return {"success": False, "error": "Path not found"}
    if not p.is_file():
        return {"success": False, "error": "Path is not a file"}

    return _edit_text(p, begin_line, end_line, text)


DOC = """\
!!!UNSAFE!!!

 Purpose
 - Edit the content of a file by replacing lines in the half-open range [begin_line, end_line) (0-based).

 Semantics and constraints
 - Range semantics:
   - end_line is exclusive; 0 <= begin_line <= end_line <= num_lines
   - If begin_line == end_line, the edit is an insertion at that position
 - Path handling:
   - If path does not exist, the implementation may create an empty file before editing
   - If path exists but is not a regular file, return error: "Path is not a file"
   - If parent directories do not exist or permissions prevent file creation, write will fail

 Behaviors
 - begin_line / end_line bounds:
   - begin_line < 0 or end_line < 0 → error "Invalid begin_line"/"Invalid end_line"
   - begin_line > end_line → error "begin_line cannot be greater than end_line"
   - end_line > num_lines → error "Invalid end_line" (must be <= num_lines)
 - Text handling:
   - text = "" yields no insertion (no-op)
   - If original file used CRLF, the result will use LF only (newline normalization)
 - Encoding and I/O:
   - Reads use the platform default text encoding; non-UTF-8 bytes may raise UnicodeDecodeError
   - Writing uses default encoding; encoding mismatches can occur for non-ASCII content
 - File content integrity:
   - The operation is not atomic; interruptions during write may leave the file partially updated
   - If end_line equals num_lines, replacement occurs at end of file (append-like)
 - Edge behaviors:
   - Insertion near the start or end of a large file is handled the same as any other range
   - Non-text files (where reading as text would fail) will raise an error when attempting to read/write
 - Return values:
   - On success: {"success": True, "output": "", "error": ""} (output may be empty by design)
   - On error: {"success": False, "output": "", "error": "<descriptive message>"}

 Examples
 - Insert at position 2 in a 10-line file:
   edit("/path/to/file.txt", 2, 2, "new content")
 - Replace lines 2 through 5 (4 lines) with new content:
   edit("/path/to/file.txt", 2, 6, "new content")
 - Replace entire file:
   edit("/path/to/file.txt", 0, num_lines, "fresh start")
"""


class EditFunction(Function):
    FUNCTION_t = FunctionType(
        "edit",
        DOC,
        [
            FunctionType.ParamLiteral(
                "path", "string", "Path to the file to be edited."
            ),
            FunctionType.ParamLiteral(
                "begin_line", "integer", "Starting line number (inclusive)."
            ),
            FunctionType.ParamLiteral(
                "end_line", "integer", "Ending line number (exclusive)."
            ),
            FunctionType.ParamLiteral(
                "text", "string", "New text to insert between begin_line and end_line."
            ),
        ],
    )
    CALLABLE = edit
