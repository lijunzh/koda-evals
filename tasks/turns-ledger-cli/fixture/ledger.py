"""A tiny expense ledger.

Usage:
  python3 ledger.py add DATE AMOUNT CATEGORY [NOTE]
  python3 ledger.py list

DATE is YYYY-MM-DD and AMOUNT is in dollars, such as 12.50. The ledger is
ledger.json in the working directory, or the file the LEDGER environment
variable names.
"""

import json
import os
import sys


def ledger_path():
    return os.environ.get("LEDGER", "ledger.json")


def load():
    try:
        with open(ledger_path()) as f:
            return json.load(f)
    except FileNotFoundError:
        return []


def save(entries):
    with open(ledger_path(), "w") as f:
        json.dump(entries, f, indent=2)


def add(date, amount, category, note=""):
    entries = load()
    entries.append({"date": date, "amount": float(amount), "category": category, "note": note})
    save(entries)


def list_entries():
    for e in load():
        print(f"{e['date']}  {e['amount']:>10.2f}  {e['category']}  {e['note']}".rstrip())


def main(argv):
    if not argv:
        print(__doc__)
        return 2
    command, *rest = argv
    if command == "add":
        add(*rest)
        return 0
    if command == "list":
        list_entries()
        return 0
    print(f"unknown command: {command}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
