"""A person's day: their free slots and how much of the working day is
taken, from their meetings and the rooms they booked."""

from datetime import date, datetime, time, timedelta
from pathlib import Path
from typing import List, NamedTuple

from . import intervals
from .intervals import Interval
from .sources import bookings, calendar

WORKDAY = (time(9, 0), time(17, 0))


class Day(NamedTuple):
    free: List[Interval]
    busy_minutes: float
    utilization: float  # share of the working day that's busy, 0..1


def busy(data: Path, person: str, day: date) -> List[Interval]:
    """Everything that makes `person` busy on `day`: every calendar event
    (the export is theirs) and every room they booked."""
    found = [iv for _, iv in calendar.read(data / "calendar.ics") if iv.start.date() == day]
    # The booking export's `end` is the last booked minute (data/README.md),
    # so a booking is busy until one minute after it.
    found += [Interval(b.interval.start, b.interval.end + timedelta(minutes=1))
              for b in bookings.read(data / "bookings.csv")
              if b.booked_by == person and b.interval.start.date() == day]
    return found


def day_report(data: Path, person: str, day: date, min_minutes: float = 0) -> Day:
    window = Interval(datetime.combine(day, WORKDAY[0]), datetime.combine(day, WORKDAY[1]))
    taken = busy(data, person, day)
    minutes = intervals.busy_minutes(taken, window)
    return Day(
        free=intervals.free_slots(taken, window, min_minutes),
        busy_minutes=minutes,
        utilization=minutes / window.minutes,
    )
