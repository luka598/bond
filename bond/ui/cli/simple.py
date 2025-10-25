import time
from io import StringIO

from prompt_toolkit import PromptSession
from prompt_toolkit.history import InMemoryHistory
from prompt_toolkit import print_formatted_text
from prompt_toolkit.patch_stdout import patch_stdout
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.formatted_text import ANSI, to_formatted_text
from rich.markdown import Markdown
from rich.console import Console

from bond.config import Config
from bond.lib.agent.main import Agent
from bond.lib.llm.interface import (
    MSG_t,
    TextMsg,
    FunctionCallMsg,
    FunctionResultMsg,
    ErorrMsg,
)


class Simple:
    def __init__(self, conf: Config) -> None:
        self.conf = conf

        if self.conf["provider"]["name"] == "gemini":
            from bond.lib.llm.impl.gemini_oai import GeminiLLM

            self.agent = Agent(conf, GeminiLLM(conf), self.handle_msg)
        elif self.conf["provider"]["name"] == "openai":
            from bond.lib.llm.impl.openai import OpenAILLM

            self.agent = Agent(conf, OpenAILLM(conf), self.handle_msg)

        kb = KeyBindings()
        kb.add("c-c")(lambda event: self.agent.cancel.set())
        kb.add("c-d")(lambda event: event.app.exit(exception=KeyboardInterrupt))
        kb.add("escape", "enter")(lambda event: event.current_buffer.insert_text("\n"))

        self.session = PromptSession(
            history=InMemoryHistory(),
            show_frame=True,
            bottom_toolbar=self.bottom_toolbar,
            key_bindings=kb,
        )


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
            G = "\033[32m"
            Y = "\033[33m"
            # STAR = f"{Y}*{R}"
            STAR = f"{Y}✓{R}"
            MAX_LEN = 32

            print_formatted_text(to_formatted_text(ANSI(f"{STAR} {G}{msg.name}{R}")))

        elif isinstance(msg, ErorrMsg):
            txt = ""
            txt += f"\033[31mError: {msg.data}\033[0m"
            if self.conf.get("debug", False):
                txt += f"\n{msg.ext}"

            print_formatted_text(to_formatted_text(ANSI(txt)))

        else:
            print_formatted_text(f"? {msg.__dict__}")

    def bottom_toolbar(self):
        s = ""
        s += f"{self.conf['provider']['name']:<10} | "
        s += f"{self.conf['provider']['model']:<10} | "
        s += f"{'WORKING' if self.agent.busy else 'READY':<10}"
        s += " |==| "
        s += "Enter: Send | "
        s += "C-c: Stop | "
        s += "C-d: Exit | "
        s += "Alt-Enter: Newline"
        return s

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
