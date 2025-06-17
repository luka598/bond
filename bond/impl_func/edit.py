import os
from bond.llm import FunctionType


def _edit_text(path: str, edit_text: str) -> str:
    try:
        with open(path, "r", encoding="utf-8") as f:
            current_lines = f.readlines()
        current_lines = [line.rstrip("\n") for line in current_lines]

        modified_lines = list(current_lines)

        for line_edit_entry in edit_text.splitlines():
            parts = line_edit_entry.split("|", 1)
            if len(parts) != 2:
                return f"Error: Invalid text edit line format: '{line_edit_entry}'. Expected 'NNN|content'."
            try:
                line_num = int(parts[0].strip())
            except ValueError:
                return f"Error: Invalid line number in text edit: '{parts[0].strip()}'."

            new_content = parts[1]
            line_idx = line_num - 1

            if line_idx < len(modified_lines):
                modified_lines[line_idx] = new_content
            elif line_idx == len(modified_lines):
                modified_lines.append(new_content)
            else:
                for _ in range(len(modified_lines), line_idx):
                    modified_lines.append("")
                modified_lines.append(new_content)

        with open(path, "w", encoding="utf-8") as f:
            f.write("\n".join(modified_lines))

        with open(path, "r", encoding="utf-8") as f:
            return "Success"

    except UnicodeDecodeError:
        return f"Error: File '{path}' is not a valid UTF-8 text file."
    except Exception as e:
        return f"Error editing text file: {e}"


def _edit_binary(path: str, edit_text: str) -> str:
    try:
        with open(path, "rb") as f:
            current_bytes = bytearray(f.read())

        modified_bytes = bytearray(current_bytes)

        for byte_edit_entry in edit_text.splitlines():
            parts = byte_edit_entry.split("|", 1)
            if len(parts) != 2:
                return f"Error: Invalid binary edit line format: '{byte_edit_entry}'. Expected 'XXXXXXXX|YY YY...'.'"
            try:
                offset = int(parts[0].strip(), 16)
                hex_data_str = parts[1].strip().replace(" ", "")
                bytes_to_write = bytes.fromhex(hex_data_str)
            except ValueError:
                return f"Error: Invalid offset or hex data in binary edit: '{byte_edit_entry}'."

            required_size = offset + len(bytes_to_write)
            if required_size > len(modified_bytes):
                modified_bytes.extend(b"\x00" * (required_size - len(modified_bytes)))

            modified_bytes[offset : offset + len(bytes_to_write)] = bytes_to_write

        with open(path, "wb") as f:
            f.write(modified_bytes)

        return "Success"

    except Exception as e:
        return f"Error editing binary file: {e}"


def edit(path: str, edit_text: str, format: str) -> str:
    if not os.path.exists(path):
        return f"Error: File not found at {path}"
    if os.path.isdir(path):
        return f"Error: Path is a directory, not a file: {path}"

    if format == "text":
        result = _edit_text(path, edit_text)
    elif format == "binary":
        result = _edit_binary(path, edit_text)
    else:
        return "Error: Invalid 'format' specified. Must be 'text' or 'binary'."

    return result


FUNCTION = (
    FunctionType(
        "edit",
        "Edits the content of a file (text or binary) and modifies the actual file on disk. Prints the modified content.",
        [
            FunctionType.Param("path", "string", "Path to the file to be edited."),
            FunctionType.Param(
                "edit_text",
                "string",
                """"String containing the edits.
                For 'text' format: each line 'NNN|content'.
                For 'binary' format: each line 'XXXXXXXX|YY YY...' (hex bytes, no ASCII part).""",
            ),
            FunctionType.Param(
                "format",
                "string",
                "Format of the file and edit_text: 'text' or 'binary'.",
            ),
        ],
    ),
    edit,
)
