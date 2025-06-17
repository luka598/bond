import typing as T
import threading

from prompt_toolkit import PromptSession
from prompt_toolkit.history import FileHistory
from prompt_toolkit.completion import Completer, Completion
from concurrent.futures import ThreadPoolExecutor

from bond.config import Config
from bond.agent import Agent


class CommandRegistry:
    def __init__(self, executor):
        self._commands = {}
        self.executor = executor

    def register(self, name, handler, help_text="", run_in_thread=False):
        self._commands[name] = (handler, help_text, run_in_thread)

    def run(self, cmd_line):
        parts = cmd_line.split()
        if not parts:
            return
        name, *args = parts
        entry = self._commands.get(name)
        if not entry:
            print(f"Unknown command: {name}")
            return
        handler, _, run_in_thread = entry
        if run_in_thread:
            self.executor.submit(handler, args)
        else:
            handler(args)

    def get_help(self):
        for name, (_, help_text, _) in self._commands.items():
            print(f"  {name:<10} {help_text}")


class MyCompleter(Completer):
    def __init__(self, registry):
        self.registry = registry

    def get_completions(self, document, complete_event):
        text = document.text_before_cursor
        if text.startswith("/"):
            text = text[1:]
            for cmd in self.registry._commands:
                if cmd.startswith(text):
                    yield Completion(cmd, start_position=-len(text))


class Cli:
    def __init__(self) -> None:
        self.config = Config({})
        try:
            self.config.merge(Config.from_file("~/.bond_conf.toml"))
        except Exception as e:
            pass
        try:
            self.config.merge(Config.from_file(".bond_conf.toml"))
        except Exception as e:
            pass

        self.agent: T.Optional[Agent] = None

        self.executor = ThreadPoolExecutor(max_workers=4)
        self.registry = CommandRegistry(self.executor)

        self.registry.register("help", self.cmd_help, "Show help")
        self.registry.register("lconf", self.cmd_lconf, "Loads config")
        self.registry.register("set", self.cmd_set, "Sets a variable in the config")
        self.registry.register("start", self.cmd_start, "Starts agent")
        self.registry.register("quit", self.cmd_quit, "Exit REPL")

        self.session = PromptSession(history=FileHistory(".bond_history"))
        self.completer = MyCompleter(self.registry)

    def run(self):
        while True:
            try:
                line = self.session.prompt("> ", completer=self.completer)
                if not line:
                    continue

                if line.startswith("/"):
                    self.registry.run(line[1:])
                else:
                    self.on_text(line)

            except (EOFError, KeyboardInterrupt):
                print("Exiting.")
                break

    def cmd_help(self, args):
        self.registry.get_help()

    def cmd_set(self, args):
        if len(args) != 2:
            print("Usage: /set [KEY] [VALUE]")
            return

        self.config[args[0]] = args[1]

    def cmd_lconf(self, args):
        if len(args) == 0:
            print("Usage: /lconf [PATH]")
            return

        try:
            conf = Config.from_file(args[0])
        except Exception as e:
            print("Failed to load the config file.")
            return

        self.config.merge(conf)

    def cmd_start(self, args):
        if self.config.get("llm", None) is None:
            print("NOT CONFIGURED")
            return

        try:
            if self.config["llm"] == "oai":
                from bond.impl_llm.oai.gpt import OAILLM

                llm = OAILLM(self.config)

            elif self.config["llm"] == "gemini":
                from bond.impl_llm.gemini.llm import GeminiLLM

                llm = GeminiLLM(self.config)

            else:
                print(f"Unknown llm {args[0]}")
                return
        except Exception as e:
            print(f"[Error] {e.__class__.__name__}: {str(e)}")
            return

        self.agent = Agent(self.config, llm)

    def cmd_quit(self, args):
        exit(0)

    def on_text(self, text: str):
        if self.agent is None:
            print("No agent running. Start an agent by using /start")
        else:
            self.agent.on_user_input(text)


def cli():
    # print("The name is Bond. James Bond.")
    cli = Cli()
    cli.run()
