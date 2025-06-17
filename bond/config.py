import typing as T
import toml
import pathlib


class Config:
    def __init__(self, config: T.Dict[str, T.Any]) -> None:
        self._config = config

    def __getitem__(self, key: str) -> T.Any:
        return self.get(key)

    def __setitem__(self, key: str, value: T.Any) -> None:
        self.set(key, value)

    def get(self, key: str, default: T.Any = None):
        return self._config.get(key, default)

    def set(self, key: str, value: str):
        self._config[key] = value
        return

    @staticmethod
    def from_file(path: str):
        config_path = pathlib.Path(path).expanduser().resolve()
        if not config_path.exists():
            raise FileNotFoundError(f"Config file not found: {config_path}")
        if not config_path.is_file():
            raise ValueError(f"Config path is not a file: {config_path}")

        config_data = toml.load(config_path)
        return Config(config_data)

    def merge(self, conf: "Config"):
        self._config.update(conf._config)


GLOBAL_CONFIG = Config({})
