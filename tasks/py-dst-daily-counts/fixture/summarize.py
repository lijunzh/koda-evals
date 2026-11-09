"""Print daily event counts: python3 summarize.py --tz ZONE LOG..."""

import argparse
import sys

from eventlog.parse import read_events
from eventlog.report import daily_counts, hourly_rate, level_counts


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--tz", default="UTC", help="IANA time zone, e.g. America/New_York")
    ap.add_argument("logs", nargs="+")
    args = ap.parse_args(argv)
    lines = []
    for path in args.logs:
        with open(path, encoding="utf-8") as f:
            lines.extend(f)
    events = read_events(lines)
    counts = daily_counts(events, args.tz)
    rates = hourly_rate(events, args.tz)
    levels = level_counts(events, args.tz)
    print(f"{'day':<12}{'events':>8}{'per hour':>10}  levels")
    for day, n in counts.items():
        lv = ", ".join(f"{k} {v}" for k, v in sorted(levels[day].items()))
        print(f"{day.isoformat():<12}{n:>8}{rates[day]:>10.2f}  {lv}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
