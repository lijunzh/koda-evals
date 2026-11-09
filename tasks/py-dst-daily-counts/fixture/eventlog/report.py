"""Per-day figures in a local time zone."""

from collections import Counter
from datetime import date, datetime, timedelta
from typing import Dict, List
from zoneinfo import ZoneInfo

from .parse import Event


def _zone_offset(tz: ZoneInfo, events: List[Event]) -> timedelta:
    """The zone's offset from UTC for this batch of events."""
    return tz.utcoffset(events[0].when.replace(tzinfo=None))


def daily_counts(events: List[Event], tz_name: str) -> Dict[date, int]:
    """How many events fell on each local calendar day, oldest day first."""
    if not events:
        return {}
    tz = ZoneInfo(tz_name)
    offset = _zone_offset(tz, events)
    counts = Counter((e.when + offset).date() for e in events)
    return dict(sorted(counts.items()))


def hourly_rate(events: List[Event], tz_name: str) -> Dict[date, float]:
    """Events per hour on each local day."""
    return {day: n / 24 for day, n in daily_counts(events, tz_name).items()}


def level_counts(events: List[Event], tz_name: str) -> Dict[date, Dict[str, int]]:
    """Events per level on each local day."""
    tz = ZoneInfo(tz_name)
    if not events:
        return {}
    offset = _zone_offset(tz, events)
    out: Dict[date, Counter] = {}
    for e in events:
        out.setdefault((e.when + offset).date(), Counter())[e.level] += 1
    return {day: dict(c) for day, c in sorted(out.items())}
