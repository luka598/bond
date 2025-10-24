import time
from io import StringIO

from prompt_toolkit import PromptSession
from prompt_toolkit.history import InMemoryHistory
from prompt_toolkit import print_formatted_text
from prompt_toolkit.patch_stdout import patch_stdout
from rich.markdown import Markdown
from rich.console import Console
from prompt_toolkit.formatted_text import ANSI, to_formatted_text

from bond.config import Config
from bond.lib.agent.main import Agent
from bond.lib.llm.interface import (
    MSG_t,
    TextMsg,
    FunctionCallMsg,
    FunctionResultMsg,
    ErorrMsg,
)
from bond.lib.llm.impl.gemini_oai import GeminiLLM


class Simple:
    def __init__(self, conf: Config) -> None:
        self.conf = conf
        self.session = PromptSession(
            history=InMemoryHistory(),
            show_frame=True,
            bottom_toolbar=self.bottom_toolbar,
        )
        self.agent = Agent(GeminiLLM(conf), self.handle_msg)

    def handle_msg(self, msg: MSG_t):
        if isinstance(msg, TextMsg):
            if msg.role == "system":
                print_formatted_text(f"S {msg.data}")
            elif msg.role == "llm":
                console = Console(
                    file=StringIO(),
                    highlight=True,
                    force_terminal=True,
                    color_system="truecolor",
                )
                console.print(Markdown(msg.data))
                txt = to_formatted_text(ANSI(console.file.getvalue()))
                print_formatted_text(txt)
            elif msg.role == "user":
                # print_formatted_text(f"> {msg.data}")
                pass
        elif isinstance(msg, FunctionCallMsg):
            R = "\033[0m"
            G = "\033[32m"  # Green
            Y = "\033[33m"  # Yellow (for star and arg keys)
            O = "\033[93m"  # Bright Yellow (often renders as orange/amber)
            W = "\033[37m"  # White
            STAR = f"{Y}${R}"
            MAX_LEN = 32

            param_strs = []
            for k, v in msg.params.items():
                v_str = str(v)
                if len(v_str) > MAX_LEN:
                    v_str = v_str[:MAX_LEN] + "..."
                param_strs.append(f"{Y}{k}{R}={W}{v_str}{R}")

            params_output = ", ".join(param_strs)
            print_formatted_text(
                to_formatted_text(
                    ANSI(f"{STAR} {G}{msg.name}{R}{O}({R}{params_output}{O}){R}")
                )
            )

        elif isinstance(msg, FunctionResultMsg):
            R = "\033[0m"
            G = "\033[32m"  # Green
            Y = "\033[33m"  # Yellow (for star and arg keys)
            O = "\033[93m"  # Bright Yellow (often renders as orange/amber)
            W = "\033[37m"  # White
            # STAR = f"{Y}*{R}"
            STAR = f"{Y}✓{R}"
            MAX_LEN = 32

            print_formatted_text(
                to_formatted_text(
                    ANSI(f"{STAR} {G}{msg.name}{R}")
                )
            )

        elif isinstance(msg, ErorrMsg):
            print_formatted_text(
                to_formatted_text(ANSI(f" \033[31mError: {msg.data}\033[0m"))
            )

        else:
            print_formatted_text(f"? {msg.__dict__}")

    def bottom_toolbar(self):
        return f"{'WORKING' if self.agent.busy else 'READY'}"

    def loop(self):
        try:
            while True:
                with patch_stdout():
                    txt = self.session.prompt("> ")
                # print("\033[F\033[K", end='')
                self.agent.send_txt(txt)
        except KeyboardInterrupt:
            print("Bye")
            return


def run():
    conf = Config.load(".bond/conf.toml")

    Simple(conf).loop()
