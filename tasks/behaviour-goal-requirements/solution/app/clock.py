"""The service's clock. Tests replace `now` to control time."""
import time


def now() -> float:
    """Seconds on a monotonic clock."""
    return time.monotonic()
