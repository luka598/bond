import typing as T
import threading
import os


from prompt_toolkit import Application
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.layout import Layout
from prompt_toolkit.layout.containers import Window
from prompt_toolkit.layout.controls import FormattedTextControl

from bond.config import Config
from bond.llm.interface import (
    LLM,
    MSG_t,
    TextMsg,
    FunctionType,
    FunctionCallMsg,
    FunctionResultMsg,
)
from bond.agent.prompts import SYSTEM_PROMPT, TOOLS_PROMPT, ENV_PROMPT
from bond.functions.impl.bash import FUNCTION as F_bash
from bond.functions.impl.view import FUNCTION as F_view
from bond.functions.impl.edit import FUNCTION as F_edit
from bond.functions.impl.web_fetch import FUNCTION as F_web_fetch
from bond.functions.impl.web_search import FUNCTION as F_web_search


class NicePrint:
    UL = "╔"
    UR = "╗"
    LL = "╚"
    LR = "╝"
    LINE_H = "═"
    LINE_V = "║"

    @staticmethod
    def upper_b():
        w, h = os.get_terminal_size()

        s = ""
        s += NicePrint.UL
        s += NicePrint.LINE_H * (w - 2)
        s += NicePrint.UR

        print(s)

    @staticmethod
    def lower_b():
        w, h = os.get_terminal_size()

        s = ""
        s += NicePrint.LL
        s += NicePrint.LINE_H * (w - 2)
        s += NicePrint.LR

        print(s)

    @staticmethod
    def middle(text: str, border: bool = False) -> None:
        w, h = os.get_terminal_size()
        max_width = w - (2 if border else 0)
        padding = (max_width - len(text)) // 2

        if border:
            result = f"{NicePrint.LINE_V}{' ' * padding}{text}{' ' * (max_width - padding - len(text))}{NicePrint.LINE_V}"
        else:
            result = " " * padding + text

        print(result)

    @staticmethod
    def left(text: str, border: bool = False) -> None:
        w, h = os.get_terminal_size()
        max_w = w - (2 if border else 1)

        result = []
        lines = text.split("\n")

        for line in lines:
            while line:
                if border:
                    result.append(
                        NicePrint.LINE_V
                        + line[:max_w]
                        + " " * (max_w - len(line[:max_w]))
                        + NicePrint.LINE_V
                    )
                else:
                    result.append(line[:max_w])

                line = line[max_w:]

        print("\n".join(result))


def get_user_perm():
    NicePrint.middle("Allow this function to run? (y/n)", True)
    result = {}

    kb = KeyBindings()

    @kb.add("<any>")
    def _(event):
        k = event.key_sequence[0].key.lower()
        if k in ("y", "n"):
            result["key"] = k
            event.app.exit()

    dummy_layout = Layout(Window(FormattedTextControl(""), height=1))

    app = Application(
        key_bindings=kb, layout=dummy_layout, full_screen=False, mouse_support=False
    )
    app.run()
    return result["key"]


class Agent:
    def __init__(self, config: Config, llm: LLM) -> None:
        self.config = config
        self.llm = llm

        self.current_thread: T.Optional[str] = None
        self.threads: T.Dict[str, T.List[MSG_t]] = {}

        # self.running_mutex = threading.Lock()
        # self.running: bool = False

        self.task_functions = {
            F_bash[0].name: F_bash[1],
            F_view[0].name: F_view[1],
            F_edit[0].name: F_edit[1],
            F_web_search[0].name: F_web_search[1],
            F_web_fetch[0].name: F_web_fetch[1],
        }

        self.task_name = ""
        self.task_description = ""
        self.task_running = False

    @property
    def thread(self):
        if self.current_thread is None:
            self.new_thread()

        return self.threads[self.current_thread]  # type: ignore

    @property
    def functions(self):
        functions = []

        if self.task_running:
            functions.append(F_bash[0])
            functions.append(F_view[0])
            functions.append(F_edit[0])
            functions.append(F_web_search[0])
            functions.append(F_web_fetch[0])
            functions.append(
                FunctionType(
                    "end_task",
                    "Returns control to the user",
                    [FunctionType.Param("success", "boolean", "status of the task")],
                )
            )
        else:
            functions.append(
                FunctionType(
                    "begin_task",
                    "Sets task and takes control from the user",
                    [
                        FunctionType.Param("name", "string", "name of the task"),
                        FunctionType.Param(
                            "description", "string", "description of the task"
                        ),
                    ],
                )
            )

        return functions

    def new_thread(self):
        self.current_thread = "Thread"
        self.threads[self.current_thread] = []

        self.thread.append(TextMsg("system", ENV_PROMPT()))
        self.thread.append(TextMsg("system", TOOLS_PROMPT))
        self.thread.append(TextMsg("system", SYSTEM_PROMPT))

    def on_user_input(self, data: str):
        i = 0
        self.thread.append(TextMsg("user", data))

        while self.task_running or (i == 0):
            if i == 10:
                self.on_function_call(FunctionCallMsg("end_task", {"success": False}))
                break

            response = self.llm.send(self.thread, self.functions)
            self.thread.extend(response)
            for msg in response:
                if isinstance(msg, TextMsg):
                    if len(msg.data) > 0:
                        NicePrint.left(msg.data, self.task_running)
                elif isinstance(msg, FunctionCallMsg):
                    self.on_function_call(msg)
                else:
                    NicePrint.left(f"Unknown msg: {msg}", self.task_running)
            i += 1

    def on_function_call(self, call: FunctionCallMsg):
        self.thread.append(call)

        if call.name == "begin_task" and not self.task_running:
            NicePrint.upper_b()
            NicePrint.middle(f"STARTING TASK {call.params['name']}", True)
            self.fcall_set_task(**call.params)

        elif call.name == "end_task" and self.task_running:
            NicePrint.middle(f"ENDING TASK success={call.params['success']}", True)
            NicePrint.lower_b()
            self.fcall_return_control(**call.params)

        elif self.task_running and call.name in self.task_functions:
            NicePrint.middle(
                f"FUNCTION CALL: {call.name}({call.params})", self.task_running
            )
            if self.config.get("allow_all_function_calls", False):
                user_perm = "y"
            else:
                user_perm = get_user_perm()

            if user_perm == "y":
                result = self.task_functions[call.name](**call.params)
                result = str(result)
                self.thread.append(FunctionResultMsg(call.name, str(result)))

                NicePrint.middle(
                    f"FUNCTION CALL FINISHED ({len(result)} chars)", self.task_running
                )

                n = int(self.config.get("print_fcall_results", 0))
                if n > 0:
                    NicePrint.left(result[:n], True)
            else:
                self.thread.append(
                    FunctionResultMsg(
                        call.name,
                        "Failed to execute the function: user didn't allow this function call",
                    )
                )
        else:
            if not self.task_running:
                NicePrint.middle(
                    "FUNCTION CALL FAILED: task is not running", self.task_running
                )
                self.thread.append(
                    FunctionResultMsg(
                        call.name,
                        "Failed to execute the function: can't execute functions when task is not set",
                    )
                )
            else:
                NicePrint.middle(
                    f"FUNCTION CALL FAILED: function {call.name} does not exist",
                    self.task_running,
                )
                self.thread.append(
                    FunctionResultMsg(
                        call.name,
                        "Failed to execute the function: no such function exists",
                    )
                )

    def fcall_set_task(self, name: str, description: str):
        self.task_running = True
        self.task_name = name
        self.task_description = description

        self.thread.append(
            TextMsg(
                "system",
                f"Active task: {name} | Goal: {description}\n You now have access to wide variety of tools use them to solve the task.",
            )
        )

    def fcall_return_control(self, success: bool):
        self.task_running = False
        self.task_success = success

        self.thread.append(
            TextMsg(
                "system",
                f"Ended task {self.task_name}. Task result is {self.task_success}",
            )
        )
