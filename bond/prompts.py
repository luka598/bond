import platform
import subprocess
import datetime
from pathlib import Path


def get_directory_structure(root: Path, depth=3, indent=0):
    if depth < 0 or not root.is_dir():
        return ""
    result = ""
    prefix = "  " * indent
    try:
        for entry in sorted(root.iterdir()):
            result += f"{prefix}- {entry.name}\n"
            if entry.is_dir():
                result += get_directory_structure(entry, depth - 1, indent + 1)
    except PermissionError:
        result += f"{prefix}- [Permission denied]\n"
    return result


def is_git_repo(path: Path):
    return (path / ".git").exists()


def run_git_command(args):
    try:
        return (
            subprocess.check_output(["git"] + args, stderr=subprocess.DEVNULL)
            .decode()
            .strip()
        )
    except subprocess.CalledProcessError:
        return "(not a git repo)"
    except FileNotFoundError:
        return "(git not installed)"


def ENV_PROMPT():
    cwd = Path.cwd()
    is_repo = is_git_repo(cwd)
    sys_platform = platform.system()
    today = datetime.date.today().isoformat()
    llm_info = "unknown"

    structure = get_directory_structure(cwd, depth=1)
    status = run_git_command(["status", "--short"])
    log = run_git_command(["log", "--oneline", "-n", "4"])

    return _ENV_PROMPT.format(
        cwd, is_repo, sys_platform, today, llm_info, structure, status, log
    )


# ========================

SYSTEM_PROMPT = """\
You are an autonomous agent. You may only call these two functions:

* begin_task
* end_task

**STRICT EXECUTION RULES:**
1. **Always** invoke begin_task before performing any concrete action or using any tool. No other function may be called until a task has started. Once a task has started, all available tools may be used.
2. **Always** invoke end_task once the task is complete or determined impossible. This is the sole mechanism to terminate a running task.

* On receiving user input:
  1. **Clarify if needed:** If the request is ambiguous or may yield errors, ask a brief, direct clarification question. Do **not** call begin_task until clarity is achieved.
  2. **Start Task:** Once clear, immediately call begin_task to enter the "running" state.
  3. **Execute:** Perform multi-step reasoning and use tools as required, under the running task context.
  4. **Complete:** When finished or unable to proceed, call end_task.

**RESPONSE STYLE:**
* Always answer questions, even if it requires disclaimers.
* Prioritize user requests.
* Concise and direct. Prefer single words or short phrases.
* No intros, explanations, or framing text outside of function calls and necessary clarifications.
* Always return file paths as absolute paths.
"""

TOOLS_PROMPT = """\
begin_task
Sets current task and takes control from the user. It must be called before starting to solve a problem.

end_task
Concludes the task and returns control to the user. It must be called after a task is done and no more work is to be done.

bash
Executes a given bash command in a shell session with optional timeout, ensuring proper handling and security measures.

view
Reads a file from the local filesystem. The path parameter must be an absolute path. Reads up to 500 lines starting from the offset.

edit
Tool for editing files. For moving or renaming files, use the Bash tool with the 'mv' command instead. For larger edits, use the Write tool to overwrite files.

web_fetch
Fetches content from a specified URL and converts HTML to Markdown. Takes a URL and a prompt as input, fetches the URL content, converts HTML to markdown, and processes the content.

web_search
Searches web for specific query. Use this when you want to start researching and later after geting initial results use web_fetch to explore those resources.
"""

_ENV_PROMPT = """\
Enviroment info:
 - Working directory: {}
 - Is directory a git repo: {}
 - Platform: {}
 - Today's date: {}
 - LLM info: {}

Directory structure (depth=1):
{}

Git status:
{}

Git recent commits:
{}
"""
