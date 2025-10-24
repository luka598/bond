import typing as T
import subprocess

from bond.lib.functions.interface import FunctionType, Function
from bond.config import GLOBAL_CONFIG

TIMEOUT = int(GLOBAL_CONFIG.get("proc_timeout", 30))


def proc(args: T.List[str]) -> dict:
    try:
        process = subprocess.run(
            args,
            capture_output=True,
            text=True,
            timeout=TIMEOUT,
            env={},
            check=False,
        )

        return {
            "success": process.returncode == 0,
            "output": process.stdout.strip() if process.returncode == 0 else "",
            "error": process.stderr.strip() if process.returncode != 0 else "",
            "code": process.returncode,
        }

    except subprocess.TimeoutExpired:
        return {
            "success": False,
            "output": "",
            "error": f"Command timed out after {TIMEOUT} seconds",
            "code": -1,
        }
    except Exception as e:
        return {"success": False, "output": "", "error": str(e), "code": -1}


class ProcFunction(Function):
    FUNCTION_t = FunctionType(
        "proc",
        "Starts a process and returns its output. Make sure you split the args properly.",
        [FunctionType.ParamArray("args", "string", "args that will be used to run a program where args[0] is the program")],
    )
    CALLABLE = proc

