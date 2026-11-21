"""A tiny expense ledger.

Usage:
  python3 ledger.py add DATE AMOUNT CATEGORY [NOTE]
  python3 ledger.py list
  python3 ledger.py summary --month YYYY-MM [--category NAME]

DATE is YYYY-MM-DD and AMOUNT is in dollars, such as 12.50. The ledger is
ledger.json in the working directory, or the file the LEDGER environment
variable names. Amounts are stored as whole cents.
"""

import json
import os
import sys
from decimal import ROUND_HALF_UP, Decimal


def ledger_path():
    return os.environ.get("LEDGER", "ledger.json")


def to_cents(amount):
    return int((Decimal(str(amount)) * 100).quantize(Decimal(1), rounding=ROUND_HALF_UP))


def dollars(cents):
    sign = "-" if cents < 0 else ""
    return f"{sign}{abs(cents) // 100}.{abs(cents) % 100:02d}"


def load():
    try:
        with open(ledger_path()) as f:
            entries = json.load(f)
    except FileNotFoundError:
        return []
    for e in entries:
        if "amount" in e:
            e["cents"] = to_cents(e.pop("amount"))
    return entries


def save(entries):
    with open(ledger_path(), "w") as f:
        json.dump(entries, f, indent=2)


def add(date, amount, category, note=""):
    entries = load()
    entries.append({"date": date, "cents": to_cents(amount), "category": category, "note": note})
    save(entries)


def list_entries():
    for e in load():
        print(f"{e['date']}  {dollars(e['cents']):>10}  {e['category']}  {e['note']}".rstrip())


def summary(argv):
    month = category = None
    i = 0
    while i < len(argv):
        if argv[i] == "--month":
            month = argv[i + 1]
        elif argv[i] == "--category":
            category = argv[i + 1]
        else:
            print(f"unknown option: {argv[i]}", file=sys.stderr)
            return 2
        i += 2
    if not month:
        print("summary needs --month YYYY-MM", file=sys.stderr)
        return 2
    totals = {}
    for e in load():
        if not e["date"].startswith(month + "-"):
            continue
        if category is not None and e["category"] != category:
            continue
        totals[e["category"]] = totals.get(e["category"], 0) + e["cents"]
    for name in sorted(totals):
        print(f"{name} {dollars(totals[name])}")
    print(f"TOTAL {dollars(sum(totals.values()))}")
    return 0


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
    if command == "summary":
        return summary(rest)
    print(f"unknown command: {command}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
