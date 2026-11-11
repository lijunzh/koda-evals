"""python3 -m freebusy DATA_DIR PERSON YYYY-MM-DD [--min MINUTES]"""

import argparse
from datetime import date
from pathlib import Path

from .report import day_report


def main() -> None:
    ap = argparse.ArgumentParser(prog="freebusy")
    ap.add_argument("data", type=Path)
    ap.add_argument("person")
    ap.add_argument("day", type=date.fromisoformat)
    ap.add_argument("--min", type=float, default=0, dest="min_minutes",
                    help="hide free slots shorter than this many minutes")
    args = ap.parse_args()
    d = day_report(args.data, args.person, args.day, args.min_minutes)
    print(f"{args.person} on {args.day}: busy {d.busy_minutes:.0f} min ({d.utilization:.0%} of the working day)")
    for s in d.free:
        print(f"  free {s.start:%H:%M}-{s.end:%H:%M} ({s.minutes:.0f} min)")


if __name__ == "__main__":
    main()
