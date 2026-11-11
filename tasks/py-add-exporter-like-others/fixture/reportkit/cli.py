"""python3 -m reportkit.cli --format FORMAT IN.json OUT | --list-formats"""

import argparse
import json
import sys
from datetime import date

from .exporters import get_exporter

# What `--list-formats` prints, and the choices `--format` takes.
FORMATS = {
    "csv": "comma-separated values, for spreadsheets",
    "json": "a JSON list of objects, for other programs",
}


def load(path):
    with open(path, encoding="utf-8") as f:
        records = json.load(f)
    for r in records:
        for k, v in r.items():
            if isinstance(v, str) and len(v) == 10 and v[4] == "-" and v[7] == "-":
                try:
                    r[k] = date.fromisoformat(v)
                except ValueError:
                    pass
    return records


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(prog="reportkit", description=__doc__)
    ap.add_argument("--format", choices=sorted(FORMATS))
    ap.add_argument("--list-formats", action="store_true")
    ap.add_argument("input", nargs="?")
    ap.add_argument("output", nargs="?")
    args = ap.parse_args(argv)
    if args.list_formats:
        for name in sorted(FORMATS):
            print(f"{name:<6} {FORMATS[name]}")
        return 0
    if not (args.format and args.input and args.output):
        ap.error("--format, an input and an output are needed")
    records = load(args.input)
    columns = list(records[0]) if records else []
    exporter = get_exporter(args.format)
    with open(args.output, "w", encoding="utf-8", newline="") as f:
        f.write(exporter.export(records, columns))
    print(f"wrote {len(records)} records to {args.output} ({exporter.media_type})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
