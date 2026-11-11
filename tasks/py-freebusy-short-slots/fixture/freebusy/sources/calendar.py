"""Reads the calendar export (`calendar.ics`), a cut-down iCalendar file:
one VEVENT per meeting with local DTSTART and DTEND."""

from datetime import datetime
from pathlib import Path
from typing import List, Tuple

from ..intervals import Interval

FMT = "%Y%m%dT%H%M%S"


def read(path: Path) -> List[Tuple[str, Interval]]:
    """(summary, interval) for every event in the file."""
    events: List[Tuple[str, Interval]] = []
    current: dict = {}
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if line == "BEGIN:VEVENT":
            current = {}
        elif line == "END:VEVENT":
            start = datetime.strptime(current["DTSTART"], FMT)
            end = datetime.strptime(current["DTEND"], FMT)
            events.append((current.get("SUMMARY", ""), Interval(start, end)))
        elif ":" in line:
            key, _, value = line.partition(":")
            current[key.split(";")[0]] = value
    return events
