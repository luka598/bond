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
    line_count = 0
    try:
        with open(path, "r", encoding="utf-8", errors="ignore") as f:
            # Skip lines until the offset
            for _ in range(offset):
                next(f, None)

            # Read up to 500 lines
            for i, line in enumerate(f):
                if line_count >= 500:
                    break
                output.append(f"{offset + i + 1:4d}|{line.rstrip()}")
                line_count += 1
    except FileNotFoundError:
        return f"Error: File not found at {path}"
    except Exception as e:
        return f"Error reading text file: {e}"
    return "\n".join(output)


def _view_binary(path: str, offset: int = 0) -> str:
    output = []
    bytes_to_read = 500
    bytes_read_current_block = 0
    try:
        with open(path, "rb") as f:
            f.seek(offset)
            while bytes_read_current_block < bytes_to_read:
                chunk_size = min(16, bytes_to_read - bytes_read_current_block)
                chunk = f.read(chunk_size)
                if not chunk:
                    break  # EOF

                hex_part = " ".join([f"{byte:02x}" for byte in chunk])
                padded_hex_part = f"{hex_part:<{16 * 3 - 1}}"

                ascii_part = "".join(
                    [chr(byte) if 32 <= byte <= 126 else "." for byte in chunk]
                )

                output.append(
                    f"{offset + bytes_read_current_block:08x}|{padded_hex_part} |{ascii_part}|"
                )
                bytes_read_current_block += len(chunk)

    except FileNotFoundError:
        return f"Error: File not found at {path}"
    except Exception as e:
        return f"Error reading binary file: {e}"
    return "\n".join(output)


def view(path: str, offset: int = 0) -> str:
    if not os.path.exists(path):
        return f"Error: Path not found: {path}"
    if os.path.isdir(path):
        return f"Error: Path is a directory, not a file: {path}"

    if _is_text_file(path):
        return _view_text(path, offset)
    else:
        return _view_binary(path, offset)


FUNCTION = (
    FunctionType(
        "view",
        "Views the content of a file (text or binary).",
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
