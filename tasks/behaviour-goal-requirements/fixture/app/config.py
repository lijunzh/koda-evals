"""Settings from config.toml, with defaults for keys it leaves out."""
import tomllib

DEFAULTS = {"port": 8080, "log_level": "info"}


def load(path: str = "config.toml") -> dict:
    """The settings in `path`, over the defaults."""
    with open(path, "rb") as f:
        values = tomllib.load(f)
    return {**DEFAULTS, **values}
