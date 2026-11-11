"""Reads the room-booking export (`bookings.csv`).

The export's `end` is the last booked minute (see data/README.md), so a
booking's interval ends one minute after it: `14:00,14:59` is 14:00-15:00,
and `23:00,23:59` ends at midnight.
"""

import csv
from datetime import datetime, timedelta
from pathlib import Path
from typing import List, NamedTuple

from ..intervals import Interval


class Booking(NamedTuple):
    room: str
    booked_by: str
    interval: Interval


def _at(day: str, hhmm: str) -> datetime:
    return datetime.strptime(f"{day} {hhmm}", "%Y-%m-%d %H:%M")


def read(path: Path) -> List[Booking]:
    with path.open(newline="", encoding="utf-8") as f:
        return [
            Booking(
                room=row["room"].strip(),
                booked_by=row["booked_by"].strip(),
                interval=Interval(
                    _at(row["date"], row["start"]),
                    _at(row["date"], row["end"]) + timedelta(minutes=1),
                ),
            )
            for row in csv.DictReader(f)
        ]
