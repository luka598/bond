import subprocess
import shlex

from bond.llm import FunctionType

TIMEOUT = 30


def bash(command: str) -> dict:
    try:
        args = shlex.split(command)

        env = None
        if args[0] == "git":
            env = {"GIT_TERMINAL_PROMPT": "0", "GIT_SSH_COMMAND": "ssh -oBatchMode=yes"}

        process = subprocess.run(
            args,
            shell=False,
            capture_output=True,
            text=True,
            timeout=TIMEOUT,
            env=env,
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


FUNCTION = (
    FunctionType(
        "bash",
        "runs a bash command",
        [FunctionType.Param("command", "string", "Command that will be run")],
    ),
    bash,
)
