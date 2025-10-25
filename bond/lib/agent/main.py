import threading as thr
import typing as T
import uuid
from queue import Queue

from bond.config import Config
from bond.lib.llm.interface import (
    LLM,
    MSG_t,
    TextMsg,
    FunctionType,
    FunctionCallMsg,
    FunctionResultMsg,
    ErorrMsg,
)
from bond.lib.prompts.initial import INITIAL_PROMPT
from bond.lib.prompts.functions import FUNCTIONS_PROMPT
from bond.lib.functions.impl.proc import ProcFunction
from bond.lib.functions.impl.view import ViewFunction
from bond.lib.functions.impl.edit import EditFunction
from bond.lib.functions.impl.web_fetch import WebFetchFunction
from bond.lib.functions.impl.web_search import WebSearchFunction


class Chat:
    def __init__(self, llm: LLM) -> None:
        self.llm = llm
        self._threads: T.Dict[str, T.List[MSG_t]] = {}

    def threads(self):
        return list(self._threads.keys())

    def new_thread(self, name: T.Optional[str] = None) -> str:
        if name is None:
            name = uuid.uuid4().hex[:8]

        self._threads[name] = []

        return name

    def add_msg(self, thread: str, msg: MSG_t):
        if isinstance(msg, ErorrMsg):
            return
        self._threads[thread].append(msg)

    def send(self, thread: str, functions: T.List[FunctionType]):
        return self.llm.send(self._threads[thread], functions)

    def messages(self, thread: str) -> T.List[MSG_t]:
        return self._threads[thread]


class Agent:
    def __init__(self, config: Config, llm: LLM, cb: T.Callable[[MSG_t], None]) -> None:
        self.conf = config

        self.chat = Chat(llm)
        self.cb = cb

        self.mutex = thr.Lock()
        self.busy = False
        self.cancel = thr.Event()
        self.message_queue: Queue[MSG_t] = Queue(maxsize=100)

        self.thread = thr.Thread(target=self.loop, daemon=True)  # TODO: dont use daemon
        self.thread.start()

    def send_txt(self, msg: str):
        self.message_queue.put(TextMsg("user", msg))

    def loop(self):
        self.chat.new_thread("main")
        self.chat.add_msg("main", TextMsg("system", INITIAL_PROMPT))
        self.chat.add_msg("main", TextMsg("system", FUNCTIONS_PROMPT))

        while True:
            self.busy = False
            if self.message_queue.qsize() == 0:
                self.cancel.clear()
            msg = self.message_queue.get()
            self.busy = True

            self.cb(msg)
            self.chat.add_msg("main", msg)

            # print([x.__dict__ for x in self.chat._threads["main"]])
            functions = {
                x.FUNCTION_t.name: x for x in [ProcFunction, ViewFunction, EditFunction, WebFetchFunction, WebSearchFunction]
            }

            if self.cancel.is_set():
                continue

            resp = self.chat.send("main", [f.FUNCTION_t for f in functions.values()])
            for msg in resp:
                self.cb(msg)
                self.chat.add_msg("main", msg)

                if isinstance(msg, FunctionCallMsg):
                    try:
                        res = functions[msg.name].CALLABLE(**msg.params)
                    except Exception as e:
                        msg_txt = f"Failed to execute the function: {e.__class__.__name__} - {str(e)}"
                        msg_err = ErorrMsg(msg_txt, e)
                        msg_sys = TextMsg("system", msg_txt)

                        res = "error"

                        self.cb(msg_err)
                        self.chat.add_msg("main", msg_err)
                        self.cb(msg_sys)
                        self.chat.add_msg("main", msg_sys)

                    self.message_queue.put(FunctionResultMsg(msg.name, res))

                    if self.conf.get("provider", {}).get("name") == "gemini":
                        # gemini does not work properly without this
                        self.message_queue.put(TextMsg("user", ""))
