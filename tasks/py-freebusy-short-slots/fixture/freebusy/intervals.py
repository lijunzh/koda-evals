"""Half-open time intervals: an interval covers `start` up to, but not
including, `end`, so a 09:00-10:00 meeting and a 10:00-11:00 one touch
without overlapping."""

from datetime import datetime, timedelta
from typing import Iterable, List, NamedTuple


class Interval(NamedTuple):
    start: datetime
    end: datetime

    @property
    def minutes(self) -> float:
        return (self.end - self.start) / timedelta(minutes=1)


def merge(intervals: Iterable[Interval]) -> List[Interval]:
    """Busy time as sorted, disjoint intervals. Intervals that overlap or
    touch are combined, so no two results share an endpoint. Empty
    intervals are dropped."""
    out: List[Interval] = []
    for iv in sorted(i for i in intervals if i.end > i.start):
        if out and iv.start <= out[-1].end:
            if iv.end > out[-1].end:
                out[-1] = Interval(out[-1].start, iv.end)
        else:
            out.append(iv)
    return out


def free_slots(busy: Iterable[Interval], window: Interval, min_minutes: float = 0) -> List[Interval]:
    """Every gap in `window` not covered by `busy`, in order, that lasts at
    least `min_minutes`. Free and busy time together cover the window
    exactly."""
    slots: List[Interval] = []
    cursor = window.start
    for iv in merge(busy):
        if iv.end <= window.start or iv.start >= window.end:
            continue
        if iv.start > cursor:
            slots.append(Interval(cursor, min(iv.start, window.end)))
        cursor = max(cursor, iv.end)
    if cursor < window.end:
        slots.append(Interval(cursor, window.end))
    return [s for s in slots if s.minutes >= min_minutes]


def busy_minutes(busy: Iterable[Interval], window: Interval) -> float:
    """Minutes of `window` covered by `busy`, counting overlaps once."""
    total = 0.0
    for iv in merge(busy):
        start, end = max(iv.start, window.start), min(iv.end, window.end)
        if end > start:
            total += (end - start) / timedelta(minutes=1)
    return total
