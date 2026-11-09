"""Per-day figures in a local time zone."""

from collections import Counter
from datetime import date, datetime, time, timedelta, timezone
from typing import Dict, List
from zoneinfo import ZoneInfo

from .parse import Event


def _local_day(event: Event, tz: ZoneInfo) -> date:
    """The local calendar day of `event`, at the offset in force at that moment."""
    return event.when.astimezone(tz).date()


def _hours_in(day: date, tz: ZoneInfo) -> float:
    """How long `day` lasts in `tz`: 23 or 25 hours when the clocks change."""
    start = datetime.combine(day, time(), tzinfo=tz).astimezone(timezone.utc)
    end = datetime.combine(day + timedelta(days=1), time(), tzinfo=tz).astimezone(timezone.utc)
    return (end - start) / timedelta(hours=1)


def daily_counts(events: List[Event], tz_name: str) -> Dict[date, int]:
    """How many events fell on each local calendar day, oldest day first."""
    tz = ZoneInfo(tz_name)
    counts = Counter(_local_day(e, tz) for e in events)
    return dict(sorted(counts.items()))


def hourly_rate(events: List[Event], tz_name: str) -> Dict[date, float]:
    """Events per hour on each local day."""
    tz = ZoneInfo(tz_name)
    return {day: n / _hours_in(day, tz) for day, n in daily_counts(events, tz_name).items()}


def level_counts(events: List[Event], tz_name: str) -> Dict[date, Dict[str, int]]:
    """Events per level on each local day."""
    tz = ZoneInfo(tz_name)
    out: Dict[date, Counter] = {}
    for e in events:
        out.setdefault(_local_day(e, tz), Counter())[e.level] += 1
    return {day: dict(c) for day, c in sorted(out.items())}
