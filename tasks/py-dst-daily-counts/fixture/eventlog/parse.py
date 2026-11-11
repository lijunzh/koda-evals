"""Reading log lines into events."""

from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Iterable, List


@dataclass(frozen=True)
class Event:
    when: datetime  # always UTC
    level: str
    message: str


def parse_line(line: str) -> Event:
    """`2026-03-08T06:59:59Z INFO user login` -> Event."""
    stamp, level, message = line.rstrip("\n").split(" ", 2)
    when = datetime.fromisoformat(stamp.replace("Z", "+00:00"))
    return Event(when.astimezone(timezone.utc), level, message)


def read_events(lines: Iterable[str]) -> List[Event]:
    """Every event in `lines`, skipping blank lines and `#` comments, oldest first."""
    events = [parse_line(l) for l in lines if l.strip() and not l.startswith("#")]
    return sorted(events, key=lambda e: e.when)
